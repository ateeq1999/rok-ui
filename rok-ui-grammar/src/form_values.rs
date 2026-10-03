//! `#[derive(FormValues)]`.

use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{spanned::Spanned, Data, DeriveInput, Fields};

/// Expand `#[derive(FormValues)]`: a typed field constant per field and the `FormValues` impl.
///
/// # Errors
///
/// Fails with a spanned error when the input does not parse or is invalid.
pub fn expand_form_values(input: TokenStream) -> syn::Result<TokenStream> {
    let input: DeriveInput = syn::parse2(input)?;
    let name = &input.ident;
    let error = || {
        syn::Error::new(
            input.span(),
            "#[derive(FormValues)] works on structs with named fields",
        )
    };
    let Data::Struct(data) = &input.data else {
        return Err(error());
    };
    let Fields::Named(fields) = &data.fields else {
        return Err(error());
    };
    if !input.generics.params.is_empty() {
        return Err(syn::Error::new(
            input.generics.span(),
            "#[derive(FormValues)] structs cannot be generic",
        ));
    }
    let constants = fields.named.iter().map(|field| {
        let Some(ident) = field.ident.as_ref() else {
            return Err(syn::Error::new(field.span(), "expected a named field"));
        };
        let text = ident.to_string();
        let constant = format_ident!("{}", text.trim_start_matches("r#").to_uppercase(), span = ident.span());
        let ty = &field.ty;
        let vis = &field.vis;
        let doc = format!("The `{text}` field.");
        Ok(quote! {
            #[doc = #doc]
            #vis const #constant: ::rok_ui::form::Field<Self, #ty> =
                ::rok_ui::form::Field::new(#text, |values| &values.#ident, |values| &mut values.#ident);
        })
    }).collect::<syn::Result<Vec<_>>>()?;
    Ok(quote! {
        impl #name {
            #(#constants)*
        }

        impl ::rok_ui::form::FormValues for #name {}
    })
}
