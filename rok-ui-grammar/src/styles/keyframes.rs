//! `keyframes!`: named keyframes compiled to `rok_ui::motion::Keyframes`.

use proc_macro2::TokenStream;
use quote::quote;
use syn::{
    braced,
    parse::{Parse, ParseStream},
    Ident, Token, Visibility,
};

use super::{color, edit_distance, length, number, number_literal, radius, Declarations, Entry};

/// `keyframes! { [pub] NAME = { from: {..}, 50%: {..}, to: {..} } }`.
struct KeyframesObject {
    visibility: Visibility,
    name: Ident,
    frames: Vec<(TokenStream, Ident, Vec<Entry>)>,
}

struct KeyframesInput(Vec<KeyframesObject>);

impl Parse for KeyframesInput {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut objects = Vec::new();
        while !input.is_empty() {
            let visibility: Visibility = input.parse()?;
            let name: Ident = input.parse()?;
            input.parse::<Token![=]>()?;
            let body;
            braced!(body in input);
            let mut frames = Vec::new();
            while !body.is_empty() {
                // `from`, `to`, or `N%`.
                let (offset, label) = if body.peek(Ident) {
                    let label: Ident = body.parse()?;
                    let offset = match label.to_string().as_str() {
                        "from" => quote!(0.0_f32),
                        "to" => quote!(1.0_f32),
                        _ => {
                            return Err(syn::Error::new(
                                label.span(),
                                "a keyframe offset is `from`, `to` or a percentage like `50%`",
                            ))
                        }
                    };
                    (offset, label)
                } else {
                    let literal: proc_macro2::Literal = body.parse()?;
                    body.parse::<Token![%]>()?;
                    let percent = number_literal(&literal)?;
                    (
                        quote!(#percent / 100.0),
                        Ident::new("percent", literal.span()),
                    )
                };
                body.parse::<Token![:]>()?;
                let declarations;
                braced!(declarations in body);
                frames.push((offset, label, declarations.parse::<Declarations>()?.0));
                if body.is_empty() {
                    break;
                }
                body.parse::<Token![,]>()?;
            }
            objects.push(KeyframesObject {
                visibility,
                name,
                frames,
            });
            if input.peek(Token![;]) {
                input.parse::<Token![;]>()?;
            } else if input.peek(Token![,]) {
                input.parse::<Token![,]>()?;
            }
        }
        Ok(KeyframesInput(objects))
    }
}

const ANIMATABLE: &[&str] = &[
    "opacity",
    "x",
    "y",
    "width",
    "height",
    "radius",
    "background",
    "bg",
    "color",
    "border_color",
];

/// One animatable property as a `Frame` builder call.
fn frame_call(name: &Ident, value: &TokenStream) -> syn::Result<TokenStream> {
    let property = name.to_string();
    Ok(match property.as_str() {
        "opacity" => {
            let opacity = number(value)?;
            quote!(.opacity(#opacity))
        }
        "x" | "y" | "width" | "height" => {
            let length = length(value)?;
            quote!(.#name(#length))
        }
        "radius" => {
            let radius = radius(value)?;
            quote!(.radius(#radius))
        }
        "background" | "bg" => {
            let color = color(value)?;
            quote!(.background(#color))
        }
        "color" | "border_color" => {
            let color = color(value)?;
            quote!(.#name(#color))
        }
        _ => {
            let suggestion = ANIMATABLE
                .iter()
                .min_by_key(|candidate| edit_distance(&property, candidate))
                .filter(|candidate| edit_distance(&property, candidate) <= 3)
                .map(|candidate| format!("; did you mean `{candidate}`?"))
                .unwrap_or_default();
            return Err(syn::Error::new(
                name.span(),
                format!(
                    "`{property}` cannot be animated{suggestion} Animatable: {}",
                    ANIMATABLE.join(", ")
                ),
            ));
        }
    })
}

/// Expand `keyframes! { .. }`: a static `Keyframes` per object.
///
/// # Errors
///
/// Fails with a spanned error when the input does not parse or is invalid.
pub fn expand_keyframes(input: TokenStream) -> syn::Result<TokenStream> {
    let KeyframesInput(objects) = syn::parse2(input)?;
    let mut output = TokenStream::new();
    for KeyframesObject {
        visibility,
        name,
        frames,
    } in objects
    {
        let mut calls = Vec::new();
        for (offset, _label, entries) in &frames {
            let mut frame_calls = Vec::new();
            for entry in entries {
                match entry {
                    Entry::Property { name, value } => frame_calls.push(frame_call(name, value)?),
                    Entry::State { name, .. } => {
                        return Err(syn::Error::new(
                            name.span(),
                            "keyframes cannot contain hover / focus / active blocks",
                        ))
                    }
                }
            }
            calls.push(quote!(.at(#offset, ::rok_ui::motion::Frame::new() #(#frame_calls)*)));
        }
        output.extend(quote! {
            #visibility static #name: ::std::sync::LazyLock<::rok_ui::motion::Keyframes> =
                ::std::sync::LazyLock::new(|| ::rok_ui::motion::Keyframes::new() #(#calls)*);
        });
    }
    Ok(output)
}
