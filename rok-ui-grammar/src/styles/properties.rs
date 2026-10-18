//! Properties: the names `styles!` accepts and the `Sx` builder call each one makes, with
//! suggestions for misspelled names.

use proc_macro2::{TokenStream, TokenTree};
use quote::{format_ident, quote};
use syn::Ident;

use super::{
    boolean, camel_case, color, expression, font_weight, keyword, keyword_error, length, number,
    pixel_width, radius, single_ident, sx, text_size,
};

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
pub(super) fn property_call(name: &Ident, value: &TokenStream) -> syn::Result<TokenStream> {
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
            let size = text_size(value);
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

pub(super) fn edit_distance(a: &str, b: &str) -> usize {
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
