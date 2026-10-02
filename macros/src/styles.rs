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
    spanned::Spanned,
    token, Ident, Lit, Path, Token, Visibility,
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

fn sx() -> TokenStream {
    quote!(::rok_ui::sx)
}

/// `snake_case` -> `CamelCase`.
fn camel_case(name: &str) -> String {
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
fn single_ident(value: &TokenStream) -> Option<Ident> {
    let mut trees = value.clone().into_iter();
    match (trees.next(), trees.next()) {
        (Some(TokenTree::Ident(ident)), None) => Some(ident),
        _ => None,
    }
}

/// A Rust expression from value tokens; `{ expr }` unwraps to `expr`.
fn expression(value: &TokenStream) -> TokenStream {
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

fn keyword_error(value: &TokenStream, property: &str, allowed: &[&str]) -> syn::Error {
    syn::Error::new(
        value.span(),
        format!(
            "unknown value for `{property}`; expected one of: {}",
            allowed.join(", ")
        ),
    )
}

/// Match a keyword value against `(keyword, tokens)` pairs.
fn keyword(
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
fn length(value: &TokenStream) -> syn::Result<TokenStream> {
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
fn number_literal(literal: &proc_macro2::Literal) -> syn::Result<TokenStream> {
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
fn color(value: &TokenStream) -> syn::Result<TokenStream> {
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

fn radius(value: &TokenStream) -> syn::Result<TokenStream> {
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

fn text_size(value: &TokenStream) -> syn::Result<TokenStream> {
    let sx = sx();
    let trees: Vec<TokenTree> = value.clone().into_iter().collect();
    // `2xl` / `3xl` tokenize as integer literals with an `xl` suffix.
    if let [TokenTree::Literal(literal)] = trees.as_slice() {
        if let Lit::Int(integer) = Lit::new(literal.clone()) {
            match (integer.base10_digits(), integer.suffix()) {
                ("2", "xl") => return Ok(quote!(#sx::SxText::Xl2)),
                ("3", "xl") => return Ok(quote!(#sx::SxText::Xl3)),
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
            return Ok(quote!(#sx::SxText::#variant));
        }
    }
    let expression = expression(value);
    Ok(quote!(#sx::SxText::Px(::rok_ui::gpui::Pixels::from(#expression))))
}

fn font_weight(value: &TokenStream) -> syn::Result<TokenStream> {
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

fn boolean(value: &TokenStream, property: &str) -> syn::Result<bool> {
    match single_ident(value) {
        Some(ident) if ident == "true" => Ok(true),
        Some(ident) if ident == "false" => Ok(false),
        _ => Err(syn::Error::new(
            value.span(),
            format!("`{property}` takes `true` or `false`"),
        )),
    }
}

fn number(value: &TokenStream) -> syn::Result<TokenStream> {
    let trees: Vec<TokenTree> = value.clone().into_iter().collect();
    match trees.as_slice() {
        [TokenTree::Literal(literal)] => number_literal(literal),
        _ => {
            let expression = expression(value);
            Ok(quote!((#expression) as f32))
        }
    }
}

fn pixel_width(value: &TokenStream) -> syn::Result<TokenStream> {
    number(value)
}

const PROPERTIES: &[&str] = &[
    "display",
    "direction",
    "flex_direction",
    "wrap",
    "flex",
    "grow",
    "shrink",
    "basis",
    "align",
    "align_items",
    "align_self",
    "justify",
    "gap",
    "gap_x",
    "gap_y",
    "position",
    "inset",
    "top",
    "right",
    "bottom",
    "left",
    "overflow",
    "width",
    "w",
    "height",
    "h",
    "size",
    "min_width",
    "max_width",
    "min_height",
    "max_height",
    "aspect_ratio",
    "padding",
    "p",
    "padding_x",
    "padding_y",
    "padding_top",
    "padding_right",
    "padding_bottom",
    "padding_left",
    "margin",
    "m",
    "margin_x",
    "margin_y",
    "margin_top",
    "margin_right",
    "margin_bottom",
    "margin_left",
    "background",
    "bg",
    "color",
    "text_color",
    "border_color",
    "border_style",
    "border",
    "border_x",
    "border_y",
    "border_top",
    "border_right",
    "border_bottom",
    "border_left",
    "radius",
    "rounded",
    "radius_top",
    "radius_bottom",
    "radius_left",
    "radius_right",
    "radius_top_left",
    "radius_top_right",
    "radius_bottom_left",
    "radius_bottom_right",
    "shadow",
    "opacity",
    "cursor",
    "text",
    "font_size",
    "font",
    "font_weight",
    "font_family",
    "line_height",
    "text_align",
    "whitespace",
    "nowrap",
    "truncate",
    "line_clamp",
    "italic",
    "underline",
    "line_through",
];

/// One property as a builder call on an `Sx`.
fn property_call(name: &Ident, value: &TokenStream) -> syn::Result<TokenStream> {
    let sx = sx();
    let edges = |edge: &str| {
        let edge = format_ident!("{}", edge);
        quote!(#sx::Edges::#edge)
    };
    let corners = |corner: &str| {
        let corner = format_ident!("{}", corner);
        quote!(#sx::Corners::#corner)
    };
    let property = name.to_string();
    let call = match property.as_str() {
        "display" => {
            let display = keyword(
                value,
                "display",
                &[
                    ("flex", quote!(Flex)),
                    ("block", quote!(Block)),
                    ("hidden", quote!(Hidden)),
                    ("none", quote!(Hidden)),
                ],
            )?;
            quote!(.decl(#sx::Decl::Display(#sx::SxDisplay::#display)))
        }
        "direction" | "flex_direction" => {
            let direction = keyword(
                value,
                "direction",
                &[
                    ("row", quote!(Row)),
                    ("column", quote!(Column)),
                    ("col", quote!(Column)),
                    ("row_reverse", quote!(RowReverse)),
                    ("column_reverse", quote!(ColumnReverse)),
                    ("row_ltr", quote!(RowLtr)),
                ],
            )?;
            quote!(.direction(#sx::SxDirection::#direction))
        }
        "wrap" => {
            let wrap = match single_ident(value) {
                Some(ident) if ident == "wrap" || ident == "true" => true,
                Some(ident) if ident == "nowrap" || ident == "false" => false,
                _ => return Err(keyword_error(value, "wrap", &["wrap", "nowrap"])),
            };
            quote!(.wrap(#wrap))
        }
        "flex" => {
            let trees: Vec<TokenTree> = value.clone().into_iter().collect();
            let flex = match trees.as_slice() {
                [TokenTree::Literal(literal)] if literal.to_string() == "1" => quote!(One),
                _ => keyword(
                    value,
                    "flex",
                    &[
                        ("auto", quote!(Auto)),
                        ("initial", quote!(Initial)),
                        ("none", quote!(None)),
                    ],
                )?,
            };
            quote!(.decl(#sx::Decl::Flex(#sx::SxFlex::#flex)))
        }
        "grow" => {
            let grow = number(value)?;
            quote!(.grow(#grow))
        }
        "shrink" => {
            let shrink = number(value)?;
            quote!(.shrink(#shrink))
        }
        "basis" => {
            let basis = length(value)?;
            quote!(.basis(#basis))
        }
        "align" | "align_items" | "align_self" => {
            let align = keyword(
                value,
                &property,
                &[
                    ("start", quote!(Start)),
                    ("center", quote!(Center)),
                    ("end", quote!(End)),
                    ("stretch", quote!(Stretch)),
                    ("baseline", quote!(Baseline)),
                ],
            )?;
            if property == "align_self" {
                quote!(.align_self(#sx::SxAlign::#align))
            } else {
                quote!(.align(#sx::SxAlign::#align))
            }
        }
        "justify" => {
            let justify = keyword(
                value,
                "justify",
                &[
                    ("start", quote!(Start)),
                    ("center", quote!(Center)),
                    ("end", quote!(End)),
                    ("between", quote!(Between)),
                    ("around", quote!(Around)),
                    ("evenly", quote!(Evenly)),
                ],
            )?;
            quote!(.justify(#sx::SxJustify::#justify))
        }
        "gap" | "gap_x" | "gap_y" => {
            let method = name;
            let gap = length(value)?;
            quote!(.#method(#gap))
        }
        "position" => {
            let absolute = match single_ident(value) {
                Some(ident) if ident == "absolute" => true,
                Some(ident) if ident == "relative" => false,
                _ => return Err(keyword_error(value, "position", &["relative", "absolute"])),
            };
            quote!(.decl(#sx::Decl::Absolute(#absolute)))
        }
        "inset" | "top" | "right" | "bottom" | "left" | "inset_start" | "inset_end" => {
            let edge = match property.as_str() {
                "inset" => edges("All"),
                "inset_start" => edges("Start"),
                "inset_end" => edges("End"),
                other => edges(&camel_case(other)),
            };
            let inset = length(value)?;
            quote!(.inset(#edge, #inset))
        }
        "overflow" => {
            keyword(value, "overflow", &[("hidden", quote!())])?;
            quote!(.overflow_hidden())
        }
        "width" | "w" => {
            let width = length(value)?;
            quote!(.w(#width))
        }
        "height" | "h" => {
            let height = length(value)?;
            quote!(.h(#height))
        }
        "size" => {
            let size = length(value)?;
            quote!(.w(#size).h(#size))
        }
        "min_width" | "max_width" | "min_height" | "max_height" => {
            let method = format_ident!(
                "{}",
                match property.as_str() {
                    "min_width" => "min_w",
                    "max_width" => "max_w",
                    "min_height" => "min_h",
                    _ => "max_h",
                }
            );
            let length = length(value)?;
            quote!(.#method(#length))
        }
        "aspect_ratio" => {
            let ratio = number(value)?;
            quote!(.aspect_ratio(#ratio))
        }
        "padding" | "p" | "margin" | "m" => {
            let method = if property.starts_with('p') {
                quote!(padding)
            } else {
                quote!(margin)
            };
            let all = edges("All");
            let length = length(value)?;
            quote!(.#method(#all, #length))
        }
        _ if property.starts_with("padding_") || property.starts_with("margin_") => {
            let (method, side) = property.split_once('_').unwrap_or_default();
            let edge = match side {
                "x" => edges("X"),
                "y" => edges("Y"),
                "top" => edges("Top"),
                "right" => edges("Right"),
                "bottom" => edges("Bottom"),
                "left" => edges("Left"),
                "start" => edges("Start"),
                "end" => edges("End"),
                _ => return Err(unknown_property(name)),
            };
            let method = format_ident!("{}", method);
            let length = length(value)?;
            quote!(.#method(#edge, #length))
        }
        "background" | "bg" => {
            let color = color(value)?;
            quote!(.bg(#color))
        }
        "color" | "text_color" => {
            let color = color(value)?;
            quote!(.text_color(#color))
        }
        "border_color" => {
            let color = color(value)?;
            quote!(.border_color(#color))
        }
        "border_style" => {
            let border_style = keyword(
                value,
                "border_style",
                &[
                    ("solid", quote!(::rok_ui::gpui::BorderStyle::Solid)),
                    ("dashed", quote!(::rok_ui::gpui::BorderStyle::Dashed)),
                ],
            )?;
            quote!(.border_style(#border_style))
        }
        "border" => {
            let width = pixel_width(value)?;
            quote!(.border(#width))
        }
        "border_x" | "border_y" | "border_top" | "border_right" | "border_bottom"
        | "border_left" | "border_start" | "border_end" => {
            let side = property.trim_start_matches("border_");
            let edge = match side {
                "x" => edges("X"),
                "y" => edges("Y"),
                other => edges(&camel_case(other)),
            };
            let width = pixel_width(value)?;
            quote!(.border_on(#edge, #width))
        }
        "radius" | "rounded" => {
            let radius = radius(value)?;
            quote!(.rounded(#radius))
        }
        _ if property.starts_with("radius_") => {
            let corner = corners(&camel_case(property.trim_start_matches("radius_")));
            let radius = radius(value)?;
            quote!(.rounded_on(#corner, #radius))
        }
        "shadow" => {
            let shadow = keyword(
                value,
                "shadow",
                &[
                    ("none", quote!(None)),
                    ("xs", quote!(Xs)),
                    ("sm", quote!(Sm)),
                    ("md", quote!(Md)),
                    ("lg", quote!(Lg)),
                    ("xl", quote!(Xl)),
                    ("ring", quote!(Ring)),
                ],
            )?;
            quote!(.shadow(#sx::SxShadow::#shadow))
        }
        "opacity" => {
            let opacity = number(value)?;
            quote!(.opacity(#opacity))
        }
        "cursor" => {
            let style = |name: &str| {
                let variant = format_ident!("{}", name);
                quote!(::rok_ui::gpui::CursorStyle::#variant)
            };
            let cursor = match single_ident(value) {
                Some(_) => keyword(
                    value,
                    "cursor",
                    &[
                        ("pointer", style("PointingHand")),
                        ("default", style("Arrow")),
                        ("text", style("IBeam")),
                        ("not_allowed", style("OperationNotAllowed")),
                        ("grab", style("OpenHand")),
                        ("grabbing", style("ClosedHand")),
                        ("ew_resize", style("ResizeLeftRight")),
                        ("col_resize", style("ResizeColumn")),
                        ("ns_resize", style("ResizeUpDown")),
                        ("row_resize", style("ResizeRow")),
                        ("crosshair", style("Crosshair")),
                    ],
                )?,
                None => expression(value),
            };
            quote!(.cursor(#cursor))
        }
        "text" | "font_size" => {
            let size = text_size(value)?;
            quote!(.text(#size))
        }
        "font" | "font_weight" => {
            let weight = font_weight(value)?;
            quote!(.font_weight(#weight))
        }
        "font_family" => {
            let font = match single_ident(value) {
                Some(ident) if ident == "sans" => quote!(#sx::SxFont::Sans),
                Some(ident) if ident == "mono" => quote!(#sx::SxFont::Mono),
                _ => {
                    let expression = expression(value);
                    quote!(#sx::SxFont::Named(::rok_ui::gpui::SharedString::from(#expression)))
                }
            };
            quote!(.font(#font))
        }
        "line_height" => {
            let length = length(value)?;
            quote!(.line_height(#length))
        }
        "text_align" => {
            let align = keyword(
                value,
                "text_align",
                &[
                    ("left", quote!(Left)),
                    ("center", quote!(Center)),
                    ("right", quote!(Right)),
                    ("start", quote!(Start)),
                    ("end", quote!(End)),
                ],
            )?;
            quote!(.text_align(#sx::SxTextAlign::#align))
        }
        "whitespace" => {
            let nowrap = match single_ident(value) {
                Some(ident) if ident == "nowrap" => true,
                Some(ident) if ident == "normal" => false,
                _ => return Err(keyword_error(value, "whitespace", &["nowrap", "normal"])),
            };
            quote!(.decl(#sx::Decl::NoWrap(#nowrap)))
        }
        "nowrap" => {
            let nowrap = boolean(value, "nowrap")?;
            quote!(.decl(#sx::Decl::NoWrap(#nowrap)))
        }
        "line_clamp" => {
            let expression = expression(value);
            quote!(.line_clamp(#expression))
        }
        "truncate" | "italic" | "underline" | "line_through" => {
            if !boolean(value, &property)? {
                return Ok(TokenStream::new());
            }
            let method = name;
            quote!(.#method())
        }
        _ => return Err(unknown_property(name)),
    };
    Ok(call)
}

fn unknown_property(name: &Ident) -> syn::Error {
    let property = name.to_string();
    let suggestion = PROPERTIES
        .iter()
        .min_by_key(|candidate| edit_distance(&property, candidate))
        .filter(|candidate| edit_distance(&property, candidate) <= 3)
        .map(|candidate| format!("; did you mean `{candidate}`?"))
        .unwrap_or_default();
    syn::Error::new(
        name.span(),
        format!("unknown style property `{property}`{suggestion}"),
    )
}

fn edit_distance(a: &str, b: &str) -> usize {
    let b: Vec<char> = b.chars().collect();
    let mut previous: Vec<usize> = (0..=b.len()).collect();
    for (i, a_character) in a.chars().enumerate() {
        let mut current = vec![i + 1];
        for (j, b_character) in b.iter().enumerate() {
            let cost = usize::from(a_character != *b_character);
            current.push(
                (previous[j] + cost)
                    .min(previous[j + 1] + 1)
                    .min(current[j] + 1),
            );
        }
        previous = current;
    }
    previous[b.len()]
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
pub fn expand_style(input: TokenStream) -> syn::Result<TokenStream> {
    let declarations: Declarations = syn::parse2(input)?;
    sx_expression(&declarations.0)
}

/// `styles! { ... }`: a static per object, with a field per key and a lookup
/// method per variant table.
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
                    fields.push(quote!(pub #name: #sx::Sx));
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
        output.extend(quote! {
            #[allow(non_snake_case)]
            #visibility struct #type_name {
                #(#fields,)*
            }

            impl #type_name {
                #(#methods)*
            }

            #visibility static #name: ::std::sync::LazyLock<#type_name> =
                ::std::sync::LazyLock::new(|| #type_name {
                    #(#initializers,)*
                });
        });
    }
    Ok(output)
}

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
