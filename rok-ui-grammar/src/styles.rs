//! `styles!` and `style!`: StyleX-style style objects compiled to `rok_ui::sx::Sx`
//! builder calls.
//!
//! ```text
//! styles! {
//!     pub CARD = {
//!         base: { display: flex, direction: column, gap: 6, background: card,
//!                 hover: { border_color: ring } },
//!         variant(ButtonVariant): { Primary: { background: primary }, ... },
//!     }
//! }
//! style! { width: px(120.), background: primary/90 }
//! ```

use proc_macro2::{Span, TokenStream, TokenTree};
use quote::{format_ident, quote, quote_spanned};
use syn::{
    braced,
    parse::{Parse, ParseStream},
    token, Ident, Path, Token, Visibility,
};

mod keyframes;
mod properties;
mod values;

pub use keyframes::expand_keyframes;
use properties::{edit_distance, property_call};
use values::{
    boolean, camel_case, color, expression, font_weight, keyword, keyword_error, length, number,
    number_literal, pixel_width, radius, single_ident, sx, text_size,
};

/// One `property: value` or `state: { ... }` entry.
enum Entry {
    Property { name: Ident, value: TokenStream },
    State { name: Ident, entries: Vec<Entry> },
}

/// A brace-delimited list of entries.
struct Declarations(Vec<Entry>);

impl Parse for Declarations {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut entries = Vec::new();
        while !input.is_empty() {
            let name: Ident = input.parse()?;
            input.parse::<Token![:]>()?;
            if matches!(name.to_string().as_str(), "hover" | "focus" | "active") {
                let content;
                braced!(content in input);
                entries.push(Entry::State {
                    name,
                    entries: content.parse::<Declarations>()?.0,
                });
            } else {
                // A value runs until the next top-level comma.
                let mut value = TokenStream::new();
                while !input.is_empty() && !input.peek(Token![,]) {
                    let tree: TokenTree = input.parse()?;
                    value.extend(std::iter::once(tree));
                }
                if value.is_empty() {
                    return Err(syn::Error::new(name.span(), "missing value"));
                }
                entries.push(Entry::Property { name, value });
            }
            if input.is_empty() {
                break;
            }
            input.parse::<Token![,]>()?;
        }
        Ok(Declarations(entries))
    }
}

/// `key: { ... }` or `key(EnumType): { Variant: { ... }, ... }` inside a styles object.
enum StyleKey {
    Plain {
        name: Ident,
        declarations: Vec<Entry>,
    },
    Variants {
        name: Ident,
        enum_type: Path,
        arms: Vec<(Ident, Vec<Entry>)>,
    },
}

/// `[pub] NAME = { key: {...}, ... }`.
struct StylesObject {
    visibility: Visibility,
    name: Ident,
    keys: Vec<StyleKey>,
}

struct StylesInput(Vec<StylesObject>);

impl Parse for StylesInput {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut objects = Vec::new();
        while !input.is_empty() {
            let visibility: Visibility = input.parse()?;
            let name: Ident = input.parse()?;
            input.parse::<Token![=]>()?;
            let body;
            braced!(body in input);
            let mut keys = Vec::new();
            while !body.is_empty() {
                let key_name: Ident = body.parse()?;
                if body.peek(token::Paren) {
                    let enum_content;
                    syn::parenthesized!(enum_content in body);
                    let enum_type: Path = enum_content.parse()?;
                    body.parse::<Token![:]>()?;
                    let arms_content;
                    braced!(arms_content in body);
                    let mut arms = Vec::new();
                    while !arms_content.is_empty() {
                        let arm: Ident = arms_content.parse()?;
                        arms_content.parse::<Token![:]>()?;
                        let declarations;
                        braced!(declarations in arms_content);
                        arms.push((arm, declarations.parse::<Declarations>()?.0));
                        if arms_content.is_empty() {
                            break;
                        }
                        arms_content.parse::<Token![,]>()?;
                    }
                    keys.push(StyleKey::Variants {
                        name: key_name,
                        enum_type,
                        arms,
                    });
                } else {
                    body.parse::<Token![:]>()?;
                    let declarations;
                    braced!(declarations in body);
                    keys.push(StyleKey::Plain {
                        name: key_name,
                        declarations: declarations.parse::<Declarations>()?.0,
                    });
                }
                if body.is_empty() {
                    break;
                }
                body.parse::<Token![,]>()?;
            }
            objects.push(StylesObject {
                visibility,
                name,
                keys,
            });
            // Objects may be separated by `;` or `,`.
            if input.peek(Token![;]) {
                input.parse::<Token![;]>()?;
            } else if input.peek(Token![,]) {
                input.parse::<Token![,]>()?;
            }
        }
        Ok(StylesInput(objects))
    }
}

/// A list of entries as an `Sx` expression.
fn sx_expression(entries: &[Entry]) -> syn::Result<TokenStream> {
    let sx = sx();
    let mut calls = Vec::new();
    for entry in entries {
        match entry {
            Entry::Property { name, value } => calls.push(property_call(name, value)?),
            Entry::State { name, entries } => {
                let inner = state_calls(entries)?;
                calls.push(quote!(.#name(|state| state #inner)));
            }
        }
    }
    Ok(quote!(#sx::Sx::new() #(#calls)*))
}

fn state_calls(entries: &[Entry]) -> syn::Result<TokenStream> {
    let mut calls = Vec::new();
    for entry in entries {
        match entry {
            Entry::Property { name, value } => calls.push(property_call(name, value)?),
            Entry::State { name, .. } => {
                return Err(syn::Error::new(
                    name.span(),
                    "states cannot be nested inside other states",
                ))
            }
        }
    }
    Ok(quote!(#(#calls)*))
}

/// `style! { ... }`: one `Sx` value.
///
/// # Errors
///
/// Fails with a spanned error when the input does not parse or is invalid.
pub fn expand_style(input: TokenStream) -> syn::Result<TokenStream> {
    let declarations: Declarations = syn::parse2(input)?;
    sx_expression(&declarations.0)
}

/// `styles! { ... }`: a static per object, with a field per key and a lookup
/// method per variant table.
///
/// # Errors
///
/// Fails with a spanned error when the input does not parse or is invalid.
pub fn expand_styles(input: TokenStream) -> syn::Result<TokenStream> {
    let StylesInput(objects) = syn::parse2(input)?;
    let sx = sx();
    let mut output = TokenStream::new();
    for object in objects {
        let StylesObject {
            visibility,
            name,
            keys,
        } = object;
        let type_name = format_ident!(
            "{}Styles",
            camel_case(&name.to_string().to_lowercase()),
            span = name.span()
        );
        let mut fields = Vec::new();
        let mut initializers = Vec::new();
        let mut methods = Vec::new();
        for key in &keys {
            match key {
                StyleKey::Plain { name, declarations } => {
                    let value = sx_expression(declarations)?;
                    let doc = format!("The `{name}` style.");
                    fields.push(quote!(#[doc = #doc] pub #name: #sx::Sx));
                    initializers.push(quote!(#name: #value));
                }
                StyleKey::Variants {
                    name,
                    enum_type,
                    arms,
                } => {
                    let mut match_arms = Vec::new();
                    for (arm, declarations) in arms {
                        let field = format_ident!("__{}_{}", name, arm, span = Span::mixed_site());
                        let value = sx_expression(declarations)?;
                        fields.push(quote!(#[doc(hidden)] #field: #sx::Sx));
                        initializers.push(quote!(#field: #value));
                        match_arms.push(quote!(#enum_type::#arm => &self.#field));
                    }
                    // Spanned on the table, so a missing variant points there.
                    let lookup = quote_spanned!(name.span()=> match value { #(#match_arms,)* });
                    methods.push(quote! {
                        /// The style for one variant.
                        pub fn #name(&self, value: #enum_type) -> &#sx::Sx {
                            #lookup
                        }
                    });
                }
            }
        }
        let struct_doc = format!("The styles of [`{name}`], generated by `styles!`.");
        let static_doc = format!("Styles generated by `styles!` (see [`{type_name}`]).");
        output.extend(quote! {
            #[doc = #struct_doc]
            #[allow(non_snake_case)]
            #visibility struct #type_name {
                #(#fields,)*
            }

            impl #type_name {
                #(#methods)*
            }

            #[doc = #static_doc]
            #visibility static #name: ::std::sync::LazyLock<#type_name> =
                ::std::sync::LazyLock::new(|| #type_name {
                    #(#initializers,)*
                });
        });
    }
    Ok(output)
}
