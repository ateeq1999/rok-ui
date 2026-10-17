//! Property values: lengths, colors, radii, text sizes, keywords and numbers, each checked
//! and turned into the `rok_ui::sx` value it stands for.

use proc_macro2::{TokenStream, TokenTree};
use quote::{format_ident, quote, quote_spanned};
use syn::{spanned::Spanned, Ident, Lit};

use super::edit_distance;

pub(super) fn sx() -> TokenStream {
    quote!(::rok_ui::sx)
}

/// `snake_case` -> `CamelCase`.
pub(super) fn camel_case(name: &str) -> String {
    name.split('_')
        .filter(|part| !part.is_empty())
        .map(|part| {
            let mut characters = part.chars();
            match characters.next() {
                Some(first) => first.to_uppercase().chain(characters).collect(),
                None => String::new(),
            }
        })
        .collect()
}

/// The single identifier a value consists of, if it is one.
pub(super) fn single_ident(value: &TokenStream) -> Option<Ident> {
    let mut trees = value.clone().into_iter();
    match (trees.next(), trees.next()) {
        (Some(TokenTree::Ident(ident)), None) => Some(ident),
        _ => None,
    }
}

/// A Rust expression from value tokens; `{ expr }` unwraps to `expr`.
pub(super) fn expression(value: &TokenStream) -> TokenStream {
    let mut trees = value.clone().into_iter();
    match (trees.next(), trees.next()) {
        (Some(TokenTree::Group(group)), None)
            if group.delimiter() == proc_macro2::Delimiter::Brace =>
        {
            group.stream()
        }
        _ => value.clone(),
    }
}

pub(super) fn keyword_error(value: &TokenStream, property: &str, allowed: &[&str]) -> syn::Error {
    syn::Error::new(
        value.span(),
        format!(
            "unknown value for `{property}`; expected one of: {}",
            allowed.join(", ")
        ),
    )
}

/// Match a keyword value against `(keyword, tokens)` pairs.
pub(super) fn keyword(
    value: &TokenStream,
    property: &str,
    options: &[(&str, TokenStream)],
) -> syn::Result<TokenStream> {
    let allowed: Vec<&str> = options.iter().map(|(name, _)| *name).collect();
    let ident = single_ident(value).ok_or_else(|| keyword_error(value, property, &allowed))?;
    options
        .iter()
        .find(|(name, _)| ident == name)
        .map(|(_, tokens)| tokens.clone())
        .ok_or_else(|| keyword_error(value, property, &allowed))
}

/// A length: `6` (spacing units), `0.5`, `50%`, `full`, `auto`, or any expression
/// convertible to `SxLength` (`px(10.)`, `relative(0.3)`, `{width}`).
pub(super) fn length(value: &TokenStream) -> syn::Result<TokenStream> {
    let sx = sx();
    let trees: Vec<TokenTree> = value.clone().into_iter().collect();
    match trees.as_slice() {
        [TokenTree::Literal(literal)] => {
            let number = number_literal(literal)?;
            Ok(quote!(#sx::SxLength::Units(#number)))
        }
        [TokenTree::Literal(literal), TokenTree::Punct(punct)] if punct.as_char() == '%' => {
            let number = number_literal(literal)?;
            Ok(quote!(#sx::SxLength::Relative(#number / 100.0)))
        }
        [TokenTree::Punct(minus), TokenTree::Literal(literal)] if minus.as_char() == '-' => {
            let number = number_literal(literal)?;
            Ok(quote!(#sx::SxLength::Units(-#number)))
        }
        [TokenTree::Ident(ident)] if ident == "full" => Ok(quote!(#sx::SxLength::Full)),
        [TokenTree::Ident(ident)] if ident == "auto" => Ok(quote!(#sx::SxLength::Auto)),
        _ => {
            let expression = expression(value);
            Ok(quote!(#sx::SxLength::from(#expression)))
        }
    }
}

/// An integer or float literal as an `f32` literal.
pub(super) fn number_literal(literal: &proc_macro2::Literal) -> syn::Result<TokenStream> {
    match Lit::new(literal.clone()) {
        Lit::Int(integer) if integer.suffix().is_empty() => {
            let value: f64 = integer.base10_parse()?;
            let value = value as f32;
            Ok(quote_spanned!(literal.span()=> #value))
        }
        Lit::Float(float) => {
            let value: f32 = float.base10_parse()?;
            Ok(quote_spanned!(literal.span()=> #value))
        }
        _ => Err(syn::Error::new(literal.span(), "expected a number")),
    }
}

const COLOR_TOKENS: &[&str] = &[
    "background",
    "foreground",
    "card",
    "card_foreground",
    "popover",
    "popover_foreground",
    "primary",
    "primary_foreground",
    "secondary",
    "secondary_foreground",
    "muted",
    "muted_foreground",
    "accent",
    "accent_foreground",
    "destructive",
    "destructive_foreground",
    "destructive_text",
    "border",
    "input",
    "ring",
    "overlay",
];

/// A color: a theme token (`primary`, `muted_foreground`), a token with
/// opacity (`primary/90`), `transparent`, or an expression (`{color}`, `rgb(0x..)`).
pub(super) fn color(value: &TokenStream) -> syn::Result<TokenStream> {
    let sx = sx();
    let trees: Vec<TokenTree> = value.clone().into_iter().collect();
    let token = |ident: &Ident| -> syn::Result<TokenStream> {
        let name = ident.to_string();
        if !COLOR_TOKENS.contains(&name.as_str()) {
            let suggestion = COLOR_TOKENS
                .iter()
                .min_by_key(|candidate| edit_distance(&name, candidate))
                .filter(|candidate| edit_distance(&name, candidate) <= 3)
                .map(|candidate| format!(" did you mean `{candidate}`?"))
                .unwrap_or_default();
            return Err(syn::Error::new(
                ident.span(),
                format!(
                    "unknown color token `{name}`;{suggestion} Theme colors: {}; or `transparent`, or an expression in braces",
                    COLOR_TOKENS.join(", ")
                ),
            ));
        }
        let variant = Ident::new(&camel_case(&name), ident.span());
        Ok(quote!(#sx::ColorToken::#variant))
    };
    match trees.as_slice() {
        [TokenTree::Ident(ident)] if ident == "transparent" => {
            Ok(quote!(#sx::SxColor::TRANSPARENT))
        }
        [TokenTree::Ident(ident)] => {
            let token = token(ident)?;
            Ok(quote!(#sx::SxColor::from(#token)))
        }
        [TokenTree::Ident(ident), TokenTree::Punct(slash), TokenTree::Literal(literal)]
            if slash.as_char() == '/' =>
        {
            let token = token(ident)?;
            let percent = number_literal(literal)?;
            Ok(quote!(#token.alpha(#percent / 100.0)))
        }
        _ => {
            let expression = expression(value);
            Ok(quote!(#sx::SxColor::from(#expression)))
        }
    }
}

pub(super) fn radius(value: &TokenStream) -> syn::Result<TokenStream> {
    let sx = sx();
    if let Some(ident) = single_ident(value) {
        let variant = match ident.to_string().as_str() {
            "none" => Some(quote!(None)),
            "sm" => Some(quote!(Sm)),
            "md" => Some(quote!(Md)),
            "lg" => Some(quote!(Lg)),
            "xl" => Some(quote!(Xl)),
            "full" => Some(quote!(Full)),
            _ => None,
        };
        if let Some(variant) = variant {
            return Ok(quote!(#sx::SxRadius::#variant));
        }
    }
    let length = length(value)?;
    Ok(quote!(#sx::SxRadius::Length(#length)))
}

pub(super) fn text_size(value: &TokenStream) -> TokenStream {
    let sx = sx();
    let trees: Vec<TokenTree> = value.clone().into_iter().collect();
    // `2xl` / `3xl` tokenize as integer literals with an `xl` suffix.
    if let [TokenTree::Literal(literal)] = trees.as_slice() {
        if let Lit::Int(integer) = Lit::new(literal.clone()) {
            match (integer.base10_digits(), integer.suffix()) {
                ("2", "xl") => return quote!(#sx::SxText::Xl2),
                ("3", "xl") => return quote!(#sx::SxText::Xl3),
                _ => {}
            }
        }
    }
    if let Some(ident) = single_ident(value) {
        let variant = match ident.to_string().as_str() {
            "theme" => Some(quote!(Theme)),
            "xs" => Some(quote!(Xs)),
            "sm" => Some(quote!(Sm)),
            "base" => Some(quote!(Base)),
            "lg" => Some(quote!(Lg)),
            "xl" => Some(quote!(Xl)),
            "xl2" => Some(quote!(Xl2)),
            "xl3" => Some(quote!(Xl3)),
            _ => None,
        };
        if let Some(variant) = variant {
            return quote!(#sx::SxText::#variant);
        }
    }
    let expression = expression(value);
    quote!(#sx::SxText::Px(::rok_ui::gpui::Pixels::from(#expression)))
}

pub(super) fn font_weight(value: &TokenStream) -> syn::Result<TokenStream> {
    let weight = |name: &str| {
        let constant = format_ident!("{}", name);
        quote!(::rok_ui::gpui::FontWeight::#constant)
    };
    keyword(
        value,
        "font_weight",
        &[
            ("thin", weight("THIN")),
            ("extralight", weight("EXTRA_LIGHT")),
            ("light", weight("LIGHT")),
            ("normal", weight("NORMAL")),
            ("medium", weight("MEDIUM")),
            ("semibold", weight("SEMIBOLD")),
            ("bold", weight("BOLD")),
            ("extrabold", weight("EXTRA_BOLD")),
            ("black", weight("BLACK")),
        ],
    )
}

pub(super) fn boolean(value: &TokenStream, property: &str) -> syn::Result<bool> {
    match single_ident(value) {
        Some(ident) if ident == "true" => Ok(true),
        Some(ident) if ident == "false" => Ok(false),
        _ => Err(syn::Error::new(
            value.span(),
            format!("`{property}` takes `true` or `false`"),
        )),
    }
}

pub(super) fn number(value: &TokenStream) -> syn::Result<TokenStream> {
    let trees: Vec<TokenTree> = value.clone().into_iter().collect();
    if let [TokenTree::Literal(literal)] = trees.as_slice() {
        number_literal(literal)
    } else {
        let expression = expression(value);
        Ok(quote!((#expression) as f32))
    }
}

pub(super) fn pixel_width(value: &TokenStream) -> syn::Result<TokenStream> {
    number(value)
}
