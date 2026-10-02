//! Procedural macros for `rok-ui`.
//!
//! The only macro is [`component`], which turns a plain function into a GPUI
//! component with a builder API, the way a React function component turns
//! props into JSX.

use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::{format_ident, quote};
use syn::{
    parse_macro_input, spanned::Spanned, Attribute, FnArg, GenericArgument, Ident, ItemFn, Pat,
    PathArguments, Type,
};

/// Turn a function into a reusable component, React style.
///
/// ```ignore
/// use rok_ui::prelude::*;
///
/// /// A greeting card.
/// #[component]
/// pub fn Greeting(
///     name: SharedString,
///     #[prop(optional)] excited: bool,
///     #[prop(optional)] on_wave: Option<EventHandler<ClickEvent>>,
///     #[children] children: Vec<AnyElement>,
///     window: &mut Window,
///     cx: &mut App,
/// ) -> impl IntoElement {
///     div().child(format!("Hello, {name}{}", if excited { "!" } else { "." })).children(children)
/// }
///
/// // Usage: required props go to `new`, optional props are builder methods,
/// // children use the familiar `.child(..)`.
/// Greeting::new("Ada").excited(true).on_wave(|_, _, _| {}).child("Welcome back");
/// ```
///
/// Parameter rules:
/// - A parameter named `window` or `cx` receives the GPUI window or app context.
/// - Plain parameters are required props and become arguments of `new(..)` (taking `impl Into<T>`).
/// - `#[prop(optional)]` parameters start at `Default::default()` and get a builder method
///   with the same name. For `Option<T>` the method takes `impl Into<T>`.
/// - Props of type `EventHandler<E>` (or `Option<EventHandler<E>>`) take a closure
///   `Fn(&E, &mut Window, &mut App)` directly.
/// - One `#[children]` parameter of type `Vec<AnyElement>` makes the component a
///   `ParentElement`, so `.child(..)` and `.children(..)` work.
/// - One `#[style]` parameter of type `StyleRefinement` makes the component `Styled`,
///   so callers can chain `.w_full()`, `.mt_4()`, … like a `className`. Apply it in
///   the body with `.apply_style_overrides(&style_overrides)`.
/// - One `#[sx] sx: Sx` parameter makes the component accept `.sx(..)` styles
///   (StyleX's `xstyle`). Apply them last in the body: `.sx((&MY_STYLES.base, &sx))`.
#[proc_macro_attribute]
pub fn component(attribute_arguments: TokenStream, item: TokenStream) -> TokenStream {
    if !attribute_arguments.is_empty() {
        return syn::Error::new(
            proc_macro2::Span::call_site(),
            "#[component] takes no arguments",
        )
        .to_compile_error()
        .into();
    }
    let function = parse_macro_input!(item as ItemFn);
    match expand_component(function) {
        Ok(tokens) => tokens.into(),
        Err(error) => error.to_compile_error().into(),
    }
}

enum PropertyKind {
    Required,
    Optional,
    Children,
    StyleOverrides,
    Sx,
}

struct ComponentProperty {
    name: Ident,
    property_type: Type,
    kind: PropertyKind,
    documentation: Vec<Attribute>,
}

fn expand_component(function: ItemFn) -> syn::Result<TokenStream2> {
    if !function.sig.generics.params.is_empty() {
        return Err(syn::Error::new(
            function.sig.generics.span(),
            "#[component] functions cannot be generic yet",
        ));
    }

    let component_name = function.sig.ident.clone();
    let visibility = function.vis.clone();
    let documentation: Vec<&Attribute> = function
        .attrs
        .iter()
        .filter(|attribute| attribute.path().is_ident("doc"))
        .collect();
    let body = &function.block;

    let mut properties = Vec::new();
    let mut window_name = format_ident!("_window");
    let mut context_name = format_ident!("_cx");
    let mut window_type: Option<Type> = None;
    let mut context_type: Option<Type> = None;

    for argument in &function.sig.inputs {
        let FnArg::Typed(typed_argument) = argument else {
            return Err(syn::Error::new(
                argument.span(),
                "#[component] functions cannot take self",
            ));
        };
        let Pat::Ident(pattern) = typed_argument.pat.as_ref() else {
            return Err(syn::Error::new(
                typed_argument.pat.span(),
                "component parameters must be plain names",
            ));
        };
        let parameter_name = pattern.ident.clone();

        if parameter_name == "window" {
            window_name = parameter_name;
            window_type = Some((*typed_argument.ty).clone());
            continue;
        }
        if parameter_name == "cx" {
            context_name = parameter_name;
            context_type = Some((*typed_argument.ty).clone());
            continue;
        }

        let mut kind = PropertyKind::Required;
        let mut parameter_documentation = Vec::new();
        for attribute in &typed_argument.attrs {
            if attribute.path().is_ident("children") {
                kind = PropertyKind::Children;
            } else if attribute.path().is_ident("style") {
                kind = PropertyKind::StyleOverrides;
            } else if attribute.path().is_ident("sx") {
                kind = PropertyKind::Sx;
            } else if attribute.path().is_ident("prop") {
                attribute.parse_nested_meta(|meta| {
                    if meta.path.is_ident("optional") {
                        kind = PropertyKind::Optional;
                        Ok(())
                    } else {
                        Err(meta.error("expected `optional`"))
                    }
                })?;
            } else if attribute.path().is_ident("doc") {
                parameter_documentation.push(attribute.clone());
            } else {
                return Err(syn::Error::new(
                    attribute.span(),
                    "unsupported attribute on a component parameter; use #[prop(optional)], #[children], #[style] or #[sx]",
                ));
            }
        }

        properties.push(ComponentProperty {
            name: parameter_name,
            property_type: (*typed_argument.ty).clone(),
            kind,
            documentation: parameter_documentation,
        });
    }

    let children_count = properties
        .iter()
        .filter(|property| matches!(property.kind, PropertyKind::Children))
        .count();
    if children_count > 1 {
        return Err(syn::Error::new(
            function.sig.span(),
            "a component can have at most one #[children] parameter",
        ));
    }

    let gpui_path = quote!(::rok_ui::gpui);
    // Keep the caller's spelling of `&mut Window` / `&mut App` so their imports stay used.
    let window_type = window_type
        .map(|window_type| quote!(#window_type))
        .unwrap_or_else(|| quote!(&mut #gpui_path::Window));
    let context_type = context_type
        .map(|context_type| quote!(#context_type))
        .unwrap_or_else(|| quote!(&mut #gpui_path::App));

    let struct_fields = properties.iter().map(|property| {
        let name = &property.name;
        let property_type = &property.property_type;
        let field_documentation = &property.documentation;
        quote! { #(#field_documentation)* #name: #property_type }
    });

    let mut constructor_parameters = Vec::new();
    let mut constructor_field_values = Vec::new();
    let mut builder_methods = Vec::new();

    for property in &properties {
        let name = &property.name;
        let property_type = &property.property_type;
        let method_documentation = &property.documentation;
        match property.kind {
            PropertyKind::Required => {
                if let Some(event_type) = event_handler_event_type(property_type) {
                    constructor_parameters.push(quote! {
                        #name: impl Fn(&#event_type, &mut #gpui_path::Window, &mut #gpui_path::App) + 'static
                    });
                    constructor_field_values.push(quote! { #name: ::std::rc::Rc::new(#name) });
                } else {
                    constructor_parameters
                        .push(quote! { #name: impl ::core::convert::Into<#property_type> });
                    constructor_field_values.push(quote! { #name: #name.into() });
                }
            }
            PropertyKind::Optional => {
                constructor_field_values
                    .push(quote! { #name: ::core::default::Default::default() });
                let inner_type = option_inner_type(property_type);
                let event_type = event_handler_event_type(inner_type.unwrap_or(property_type));
                let method = match (inner_type, event_type) {
                    (Some(_), Some(event_type)) => quote! {
                        #(#method_documentation)*
                        pub fn #name(mut self, handler: impl Fn(&#event_type, &mut #gpui_path::Window, &mut #gpui_path::App) + 'static) -> Self {
                            self.#name = ::core::option::Option::Some(::std::rc::Rc::new(handler));
                            self
                        }
                    },
                    (None, Some(event_type)) => quote! {
                        #(#method_documentation)*
                        pub fn #name(mut self, handler: impl Fn(&#event_type, &mut #gpui_path::Window, &mut #gpui_path::App) + 'static) -> Self {
                            self.#name = ::std::rc::Rc::new(handler);
                            self
                        }
                    },
                    (Some(inner_type), None) => quote! {
                        #(#method_documentation)*
                        pub fn #name(mut self, value: impl ::core::convert::Into<#inner_type>) -> Self {
                            self.#name = ::core::option::Option::Some(value.into());
                            self
                        }
                    },
                    (None, None) => quote! {
                        #(#method_documentation)*
                        pub fn #name(mut self, value: impl ::core::convert::Into<#property_type>) -> Self {
                            self.#name = value.into();
                            self
                        }
                    },
                };
                builder_methods.push(method);
            }
            PropertyKind::Children => {
                constructor_field_values.push(quote! { #name: ::std::vec::Vec::new() });
            }
            PropertyKind::StyleOverrides | PropertyKind::Sx => {
                constructor_field_values
                    .push(quote! { #name: ::core::default::Default::default() });
            }
        }
    }

    let field_names: Vec<&Ident> = properties.iter().map(|property| &property.name).collect();

    let parent_element_implementation = properties
        .iter()
        .find(|property| matches!(property.kind, PropertyKind::Children))
        .map(|children_property| {
            let children_name = &children_property.name;
            quote! {
                impl #gpui_path::ParentElement for #component_name {
                    fn extend(&mut self, elements: impl ::core::iter::IntoIterator<Item = #gpui_path::AnyElement>) {
                        self.#children_name.extend(elements);
                    }
                }
            }
        });

    let style_count = properties
        .iter()
        .filter(|property| matches!(property.kind, PropertyKind::StyleOverrides))
        .count();
    if style_count > 1 {
        return Err(syn::Error::new(
            function.sig.span(),
            "a component can have at most one #[style] parameter",
        ));
    }
    let styled_implementation = properties
        .iter()
        .find(|property| matches!(property.kind, PropertyKind::StyleOverrides))
        .map(|style_property| {
            let style_name = &style_property.name;
            quote! {
                impl #gpui_path::Styled for #component_name {
                    fn style(&mut self) -> &mut #gpui_path::StyleRefinement {
                        &mut self.#style_name
                    }
                }
            }
        });

    let sx_implementation = properties
        .iter()
        .find(|property| matches!(property.kind, PropertyKind::Sx))
        .map(|sx_property| {
            let sx_name = &sx_property.name;
            quote! {
                impl ::rok_ui::sx::SxStyled for #component_name {
                    fn apply_sx(mut self, sx: ::rok_ui::sx::Sx) -> Self {
                        self.#sx_name.merge(&sx);
                        self
                    }
                }
            }
        });

    let return_type = match &function.sig.output {
        syn::ReturnType::Default => {
            return Err(syn::Error::new(
                function.sig.span(),
                "a component must return `impl IntoElement`",
            ))
        }
        syn::ReturnType::Type(_, return_type) => return_type.clone(),
    };

    Ok(quote! {
        #(#documentation)*
        #[must_use = "components do nothing unless rendered as a child"]
        #visibility struct #component_name {
            #(#struct_fields,)*
        }

        impl #component_name {
            /// Create the component with its required props.
            #[allow(clippy::new_without_default)]
            pub fn new(#(#constructor_parameters),*) -> Self {
                Self { #(#constructor_field_values,)* }
            }

            #(#builder_methods)*
        }

        impl #gpui_path::RenderOnce for #component_name {
            #[allow(unused_variables, clippy::needless_return)]
            fn render(self, #window_name: #window_type, #context_name: #context_type) -> #return_type {
                let Self { #(#field_names,)* } = self;
                #body
            }
        }

        impl #gpui_path::IntoElement for #component_name {
            type Element = #gpui_path::Component<Self>;

            #[track_caller]
            fn into_element(self) -> Self::Element {
                #gpui_path::Component::new(self)
            }
        }

        #parent_element_implementation
        #styled_implementation
        #sx_implementation
    })
}

/// `Option<T>` -> `Some(T)`.
fn option_inner_type(property_type: &Type) -> Option<&Type> {
    single_generic_argument(property_type, "Option")
}

/// `EventHandler<E>` -> `Some(E)`.
fn event_handler_event_type(property_type: &Type) -> Option<&Type> {
    single_generic_argument(property_type, "EventHandler")
}

fn single_generic_argument<'a>(property_type: &'a Type, wrapper_name: &str) -> Option<&'a Type> {
    let Type::Path(type_path) = property_type else {
        return None;
    };
    let last_segment = type_path.path.segments.last()?;
    if last_segment.ident != wrapper_name {
        return None;
    }
    let PathArguments::AngleBracketed(arguments) = &last_segment.arguments else {
        return None;
    };
    match arguments.args.first()? {
        GenericArgument::Type(inner_type) if arguments.args.len() == 1 => Some(inner_type),
        _ => None,
    }
}

mod children;
mod styles;

/// Define StyleX-style style objects once, at module level.
///
/// ```ignore
/// styles! {
///     pub CARD = {
///         base: {
///             display: flex, direction: column, gap: 6, padding: 6,
///             radius: xl, border: 1, border_color: border, background: card,
///             hover: { border_color: ring },
///         },
///         compact: { padding: 3, gap: 3 },
///         variant(ButtonVariant): {
///             Primary: { background: primary, color: primary_foreground },
///             Outline: { border: 1, border_color: input },
///         },
///     }
/// }
///
/// div().sx((&CARD.base, compact.then_some(&CARD.compact), CARD.variant(variant)))
/// ```
///
/// Each object becomes a static (`CARD`) with one `Sx` field per key and one
/// lookup method per variant table. Values:
/// - lengths: numbers are multiples of 4px (`gap: 6` is 24px); also `50%`,
///   `full`, `auto`, or any expression (`px(10.)`, `{width}`);
/// - colors: theme tokens (`primary`, `muted_foreground`), with opacity
///   (`primary/90`), `transparent`, or an expression;
/// - radius: `none`, `sm`, `md`, `lg`, `xl`, `full`, or a length;
/// - `hover`, `focus` and `active` blocks style interaction states.
#[proc_macro]
pub fn styles(input: TokenStream) -> TokenStream {
    styles::expand_styles(input.into())
        .unwrap_or_else(|error| error.to_compile_error())
        .into()
}

/// One inline style object, for one-off or dynamic values:
/// `style! { width: {px(width)}, background: primary/90 }`.
#[proc_macro]
pub fn style(input: TokenStream) -> TokenStream {
    styles::expand_style(input.into())
        .unwrap_or_else(|error| error.to_compile_error())
        .into()
}

/// A `Vec<AnyElement>` from mixed element types, with control flow:
///
/// ```ignore
/// div().children(children![
///     Title::new("Projects"),
///     if loading { Spinner::new() } else { Badge::new("Ready") },
///     for project in &projects => Item::new(project.id).title(project.name.clone()),
///     match status { Status::Ok => "Up to date", Status::Stale => Button::new("refresh") },
///     "plain text",
/// ])
/// ```
#[proc_macro]
pub fn children(input: TokenStream) -> TokenStream {
    children::expand_children(input.into())
        .unwrap_or_else(|error| error.to_compile_error())
        .into()
}

/// JSX-like markup compiled to builder calls.
///
/// ```ignore
/// view! {
///     Card(sx = [CARD.base, compact => CARD.compact]) {
///         CardHeader {
///             CardTitle("Create project")
///             CardDescription("Deploy your new project in one click.")
///         }
///         if let Some(error) = error {
///             Alert("Deploy failed", description = error).destructive()
///         }
///         for project in &projects {
///             Item(project.id, title = project.name.clone())
///         }
///         div(sx = ROW.end) { "Raw text" {some_element} }
///     }
/// }
/// ```
///
/// - `Name(a, b, key = value)` is `Name::new(a, b).key(value)`; lowercase
///   `div(..)`, `img(src)` and `svg()` are GPUI element functions.
/// - `sx = [a, cond => b]` merges styles like `sx![..]`; `sx = expr` passes one.
/// - `.method(..)` after the arguments is passed through unchanged.
/// - `{ .. }` after an element holds its children: elements, `"text"`,
///   `{expr}`, `if` / `if let` / `match` / `for`.
#[proc_macro]
pub fn view(input: TokenStream) -> TokenStream {
    children::expand_view(input.into())
        .unwrap_or_else(|error| error.to_compile_error())
        .into()
}
