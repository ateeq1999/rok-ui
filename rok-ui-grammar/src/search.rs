//! `#[derive(Search)]`.

use proc_macro2::TokenStream;
use quote::quote;
use syn::{
    spanned::Spanned, Data, DeriveInput, Expr, Fields, GenericArgument, LitStr, PathArguments, Type,
};

fn option_inner(ty: &Type) -> Option<&Type> {
    let Type::Path(path) = ty else { return None };
    let segment = path.path.segments.last()?;
    if segment.ident != "Option" {
        return None;
    }
    let PathArguments::AngleBracketed(arguments) = &segment.arguments else {
        return None;
    };
    match arguments.args.first()? {
        GenericArgument::Type(inner) => Some(inner),
        _ => None,
    }
}

/// Expand `#[derive(Search)]`: the `Search` impl with per-field defaults and renames.
///
/// # Errors
///
/// Fails with a spanned error when the input does not parse or is invalid.
pub fn expand_search(input: TokenStream) -> syn::Result<TokenStream> {
    let input: DeriveInput = syn::parse2(input)?;
    let name = &input.ident;
    let Data::Struct(data) = &input.data else {
        return Err(syn::Error::new(
            input.span(),
            "#[derive(Search)] works on structs with named fields",
        ));
    };
    let Fields::Named(fields) = &data.fields else {
        return Err(syn::Error::new(
            input.span(),
            "#[derive(Search)] works on structs with named fields",
        ));
    };
    if !input.generics.params.is_empty() {
        return Err(syn::Error::new(
            input.generics.span(),
            "#[derive(Search)] structs cannot be generic",
        ));
    }

    let mut keys = Vec::new();
    let mut defaults = Vec::new();
    let mut reads = Vec::new();
    let mut writes = Vec::new();
    for field in &fields.named {
        let Some(ident) = field.ident.as_ref() else {
            return Err(syn::Error::new(field.span(), "expected a named field"));
        };
        let mut key = ident.to_string();
        let mut default: Option<Expr> = None;
        for attribute in field
            .attrs
            .iter()
            .filter(|attribute| attribute.path().is_ident("search"))
        {
            attribute.parse_nested_meta(|meta| {
                if meta.path.is_ident("default") {
                    if meta.input.peek(syn::Token![=]) {
                        default = Some(meta.value()?.parse()?);
                    }
                    Ok(())
                } else if meta.path.is_ident("rename") {
                    key = meta.value()?.parse::<LitStr>()?.value();
                    Ok(())
                } else {
                    Err(meta.error("expected `default`, `default = ..` or `rename = \"..\"`"))
                }
            })?;
        }
        let ty = &field.ty;
        keys.push(key.clone());
        if let Some(inner) = option_inner(ty) {
            defaults.push(quote!(#ident: ::core::option::Option::None));
            reads.push(quote!(#ident: ::rok_ui::router::parse_value::<#inner>(pairs, #key)));
            writes.push(quote! {
                if let ::core::option::Option::Some(value) = &self.#ident {
                    query.push((::std::string::String::from(#key), ::std::string::ToString::to_string(value)));
                }
            });
        } else {
            let default = default.map_or_else(
                || quote!(<#ty as ::core::default::Default>::default()),
                |expr| quote!({ let value: #ty = #expr; value }),
            );
            defaults.push(quote!(#ident: #default));
            reads.push(quote! {
                #ident: ::rok_ui::router::parse_value::<#ty>(pairs, #key).unwrap_or_else(|| #default)
            });
            writes.push(quote! {
                if self.#ident != defaults.#ident {
                    query.push((::std::string::String::from(#key), ::std::string::ToString::to_string(&self.#ident)));
                }
            });
        }
    }

    Ok(quote! {
        impl ::rok_ui::router::Search for #name {
            const FIELDS: &'static [&'static str] = &[#(#keys),*];

            fn defaults() -> Self {
                Self { #(#defaults,)* }
            }

            fn from_query(pairs: &[(::rok_ui::gpui::SharedString, ::rok_ui::gpui::SharedString)]) -> Self {
                Self { #(#reads,)* }
            }

            #[allow(unused_variables)]
            fn to_query(&self) -> ::std::vec::Vec<(::std::string::String, ::std::string::String)> {
                let defaults = <Self as ::rok_ui::router::Search>::defaults();
                let mut query = ::std::vec::Vec::new();
                #(#writes)*
                query
            }
        }
    })
}
