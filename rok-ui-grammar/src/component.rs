//! `#[component]`: a function becomes a component struct with a builder.

use proc_macro2::{Span, TokenStream};
use quote::{format_ident, quote};
use syn::{
    spanned::Spanned, Attribute, FnArg, GenericArgument, Ident, ItemFn, Pat, PathArguments, Type,
};

/// Expand `#[component]` (with its attribute arguments) on `item`, a function.
///
/// # Errors
///
/// Fails on attribute arguments, generic or method functions, unsupported parameter
/// attributes, and more than one `#[children]` parameter.
pub fn expand(attribute_arguments: TokenStream, item: TokenStream) -> syn::Result<TokenStream> {
    if !attribute_arguments.is_empty() {
        return Err(syn::Error::new(
            Span::call_site(),
            "#[component] takes no arguments",
        ));
    }
    let function: ItemFn = syn::parse2(item)?;
    expand_component(&function)
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
    /// `#[default(expr)]`: the starting value of an optional prop.
    default: Option<syn::Expr>,
    documentation: Vec<Attribute>,
}

fn expand_component(function: &ItemFn) -> syn::Result<TokenStream> {
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
        let mut default = None;
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
            } else if attribute.path().is_ident("default") {
                kind = PropertyKind::Optional;
                if let syn::Meta::List(list) = &attribute.meta {
                    default = Some(list.parse_args::<syn::Expr>()?);
                }
            } else if attribute.path().is_ident("doc") {
                parameter_documentation.push(attribute.clone());
            } else {
                return Err(syn::Error::new(
                    attribute.span(),
                    "unsupported attribute on a component parameter; use #[default], #[default(expr)], #[prop(optional)], #[children], #[style] or #[sx]",
                ));
            }
        }

        properties.push(ComponentProperty {
            name: parameter_name,
            property_type: (*typed_argument.ty).clone(),
            kind,
            default,
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
    let uses_cx = context_type.as_ref().is_some_and(is_cx_type);
    if uses_cx && window_type.is_some() {
        return Err(syn::Error::new(
            function.sig.inputs.span(),
            "a component taking `cx: &mut Cx` reaches the window through `cx.window`; remove the `window` parameter",
        ));
    }
    // Keep the caller's spelling of `&mut Window` / `&mut App` so their imports stay used.
    let window_type = window_type.map_or_else(
        || quote!(&mut #gpui_path::Window),
        |window_type| quote!(#window_type),
    );
    let context_type = context_type.map_or_else(
        || quote!(&mut #gpui_path::App),
        |context_type| quote!(#context_type),
    );

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
        let fallback_documentation = format!("Set the `{name}` prop.");
        let method_documentation = if property.documentation.is_empty() {
            vec![quote!(#[doc = #fallback_documentation])]
        } else {
            property
                .documentation
                .iter()
                .map(|attribute| quote!(#attribute))
                .collect()
        };
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
                let initial = property.default.as_ref().map_or_else(
                    || quote!(::core::default::Default::default()),
                    |default| quote!({ let value: #property_type = #default; value }),
                );
                constructor_field_values.push(quote! { #name: #initial });
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

    let render_implementation = if uses_cx {
        quote! {
            impl #gpui_path::RenderOnce for #component_name {
                #[allow(unused_variables, clippy::needless_return)]
                fn render(self, __rok_window: &mut #gpui_path::Window, __rok_app: &mut #gpui_path::App) -> #return_type {
                    let Self { #(#field_names,)* } = self;
                    let mut __rok_cx = ::rok_ui::Cx::new(__rok_window, __rok_app);
                    let #context_name: &mut ::rok_ui::Cx<'_> = &mut __rok_cx;
                    #body
                }
            }
        }
    } else {
        quote! {
            impl #gpui_path::RenderOnce for #component_name {
                #[allow(unused_variables, clippy::needless_return)]
                fn render(self, #window_name: #window_type, #context_name: #context_type) -> #return_type {
                    let Self { #(#field_names,)* } = self;
                    #body
                }
            }
        }
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

        #render_implementation

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

/// Whether `context_type` is `&mut Cx` (any path ending in `Cx`).
fn is_cx_type(context_type: &Type) -> bool {
    let Type::Reference(reference) = context_type else {
        return false;
    };
    let Type::Path(type_path) = reference.elem.as_ref() else {
        return false;
    };
    type_path
        .path
        .segments
        .last()
        .is_some_and(|segment| segment.ident == "Cx")
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
