//! BidiText: text that may mix right-to-left and left-to-right scripts.
//!
//! On macOS and Linux, and for text without right-to-left letters, this is a plain
//! text node. On Windows it wraps the text by logical words itself and reorders each
//! line with [`crate::bidi`], because GPUI's Windows backend draws right-to-left
//! runs mirrored (see the `bidi` module docs).

use gpui::{
    prelude::*, px, AnyElement, App, AvailableSpace, Bounds, ElementId, GlobalElementId, LayoutId,
    Pixels, ShapedLine, SharedString, Size, Style, TextAlign, TextOverflow, TextStyle, WhiteSpace,
    Window,
};

use crate::bidi;
use crate::components::direction::{is_rtl, TextDirection};

/// Text that renders correctly whatever mix of scripts it holds. Inherits font,
/// size, color and alignment from its parent like a plain string child.
///
/// ```ignore
/// div().text_right().child(BidiText::new("مرحبا بك في rok-ui"))
/// ```
#[derive(IntoElement)]
pub struct BidiText {
    text: SharedString,
}

impl BidiText {
    pub fn new(text: impl Into<SharedString>) -> Self {
        Self { text: text.into() }
    }
}

impl RenderOnce for BidiText {
    fn render(self, _window: &mut Window, _cx: &mut App) -> impl IntoElement {
        if bidi::platform_needs_reordering()
            && bidi::has_rtl(&self.text)
            && !bidi::is_converted(&self.text)
        {
            let direction = if is_rtl() {
                TextDirection::Rtl
            } else {
                TextDirection::Ltr
            };
            ReorderedText {
                text: self.text,
                direction,
            }
            .into_any_element()
        } else {
            self.text.into_any_element()
        }
    }
}

/// Text for the label slot of a component: a [`BidiText`] when it needs
/// reordering, the plain string otherwise.
#[cfg_attr(not(feature = "full"), allow(dead_code))]
pub(crate) fn text(text: impl Into<SharedString>) -> AnyElement {
    BidiText::new(text).into_any_element()
}

/// Lays out logical lines, then shapes each one in visual order.
struct ReorderedText {
    text: SharedString,
    direction: TextDirection,
}

/// Font metrics used for both measuring and painting.
#[derive(Clone)]
struct TextMetrics {
    style: TextStyle,
    font_size: Pixels,
    line_height: Pixels,
}

impl TextMetrics {
    fn current(window: &Window) -> Self {
        let style = window.text_style();
        let rem = window.rem_size();
        let font_size = style.font_size.to_pixels(rem);
        let line_height = style.line_height_in_pixels(rem);
        Self {
            style,
            font_size,
            line_height,
        }
    }

    fn shape(&self, logical: &str, direction: TextDirection, window: &Window) -> ShapedLine {
        let visual: SharedString = if bidi::has_rtl(logical) {
            bidi::visual_text_in_font(logical, direction, &self.style.font_family).into()
        } else {
            SharedString::from(logical.to_string())
        };
        let run = self.style.to_run(visual.len());
        window
            .text_system()
            .shape_line(visual, self.font_size, &[run], None)
    }

    fn width(&self, logical: &str, direction: TextDirection, window: &Window) -> Pixels {
        self.shape(logical, direction, window).width
    }
}

/// Split `text` into lines no wider than `max_width`, breaking between words in
/// logical order. Explicit newlines always break.
fn wrap(
    text: &str,
    max_width: Option<Pixels>,
    metrics: &TextMetrics,
    direction: TextDirection,
    window: &Window,
) -> Vec<String> {
    let mut lines = Vec::new();
    for paragraph in text.split('\n') {
        let Some(max_width) = max_width.filter(|_| metrics.style.white_space == WhiteSpace::Normal)
        else {
            lines.push(truncate(paragraph, max_width, metrics, direction, window));
            continue;
        };
        let mut line = String::new();
        for word in paragraph.split_inclusive(' ') {
            let candidate = format!("{line}{word}");
            if !line.is_empty()
                && metrics.width(candidate.trim_end(), direction, window) > max_width
            {
                lines.push(std::mem::take(&mut line).trim_end().to_string());
                line.push_str(word);
            } else {
                line = candidate;
            }
        }
        lines.push(line.trim_end().to_string());
    }
    lines
}

/// `line` cut to fit `max_width` with the overflow marker (usually "…") at its
/// logical end, when the style asks for truncation (`.truncate()`). Unchanged when it
/// fits, when no width is known or when the style does not truncate.
fn truncate(
    line: &str,
    max_width: Option<Pixels>,
    metrics: &TextMetrics,
    direction: TextDirection,
    window: &Window,
) -> String {
    let (Some(max_width), Some(TextOverflow::Truncate(marker))) =
        (max_width, metrics.style.text_overflow.as_ref())
    else {
        return line.to_string();
    };
    if metrics.width(line, direction, window) <= max_width {
        return line.to_string();
    }
    // Binary search for the longest prefix (on character boundaries) that fits.
    let boundaries: Vec<usize> = line.char_indices().map(|(index, _)| index).collect();
    let fits = |count: usize| {
        let end = boundaries.get(count).copied().unwrap_or(line.len());
        let candidate = format!("{}{marker}", line[..end].trim_end());
        metrics.width(&candidate, direction, window) <= max_width
    };
    let (mut low, mut high) = (0, boundaries.len());
    while low < high {
        let middle = (low + high).div_ceil(2);
        if fits(middle) {
            low = middle;
        } else {
            high = middle - 1;
        }
    }
    let end = boundaries.get(low).copied().unwrap_or(line.len());
    format!("{}{marker}", line[..end].trim_end())
}

impl IntoElement for ReorderedText {
    type Element = Self;

    fn into_element(self) -> Self::Element {
        self
    }
}

impl Element for ReorderedText {
    type RequestLayoutState = ();
    type PrepaintState = Vec<ShapedLine>;

    fn id(&self) -> Option<ElementId> {
        None
    }

    fn source_location(&self) -> Option<&'static core::panic::Location<'static>> {
        None
    }

    fn request_layout(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&gpui::InspectorElementId>,
        window: &mut Window,
        _cx: &mut App,
    ) -> (LayoutId, Self::RequestLayoutState) {
        let text = self.text.clone();
        let direction = self.direction;
        let metrics = TextMetrics::current(window);
        let layout_id =
            window.request_measured_layout(Style::default(), move |known, available, window, _| {
                let max_width = known.width.or(match available.width {
                    AvailableSpace::Definite(width) => Some(width),
                    _ => None,
                });
                let lines = wrap(&text, max_width, &metrics, direction, window);
                let widest = lines
                    .iter()
                    .map(|line| metrics.width(line, direction, window))
                    .fold(px(0.), Pixels::max);
                Size {
                    width: known.width.unwrap_or(widest),
                    height: metrics.line_height * lines.len().max(1) as f32,
                }
            });
        (layout_id, ())
    }

    fn prepaint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&gpui::InspectorElementId>,
        bounds: Bounds<Pixels>,
        _request_layout: &mut Self::RequestLayoutState,
        window: &mut Window,
        _cx: &mut App,
    ) -> Self::PrepaintState {
        let metrics = TextMetrics::current(window);
        // A hair of slack so rounding never wraps a line that fit while measuring.
        let lines = wrap(
            &self.text,
            Some(bounds.size.width + px(0.5)),
            &metrics,
            self.direction,
            window,
        );
        lines
            .iter()
            .map(|line| metrics.shape(line, self.direction, window))
            .collect()
    }

    fn paint(
        &mut self,
        _id: Option<&GlobalElementId>,
        _inspector_id: Option<&gpui::InspectorElementId>,
        bounds: Bounds<Pixels>,
        _request_layout: &mut Self::RequestLayoutState,
        lines: &mut Self::PrepaintState,
        window: &mut Window,
        cx: &mut App,
    ) {
        let metrics = TextMetrics::current(window);
        let mut top = bounds.top();
        for line in lines.iter() {
            let left = match metrics.style.text_align {
                TextAlign::Left => bounds.left(),
                TextAlign::Center => bounds.left() + (bounds.size.width - line.width) / 2.,
                TextAlign::Right => bounds.right() - line.width,
            };
            line.paint(gpui::point(left, top), metrics.line_height, window, cx)
                .ok();
            top += metrics.line_height;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct EmptyView;

    impl Render for EmptyView {
        fn render(&mut self, _: &mut Window, _: &mut gpui::Context<Self>) -> impl IntoElement {
            gpui::div()
        }
    }

    #[gpui::test]
    fn long_lines_truncate_with_an_ellipsis(cx: &mut gpui::TestAppContext) {
        let (_view, window_context) = cx.add_window_view(|_, _| EmptyView);
        window_context.update(|window, _| {
            let style = TextStyle {
                text_overflow: Some(TextOverflow::Truncate("…".into())),
                ..TextStyle::default()
            };
            let metrics = TextMetrics {
                style,
                font_size: px(14.),
                line_height: px(20.),
            };
            let text = "مرحبا بكم في واجهة روك للمكونات وهي مكتبة لتطبيقات سطح المكتب";
            let direction = TextDirection::Rtl;
            let max_width = px(120.);
            assert!(metrics.width(text, direction, window) > max_width);

            let cut = truncate(text, Some(max_width), &metrics, direction, window);
            assert!(cut.ends_with('…'), "{cut}");
            assert!(metrics.width(&cut, direction, window) <= max_width);
            assert!(text.starts_with(cut.trim_end_matches('…').trim_end()));

            // Text that fits, or a style without truncation, is left alone.
            assert_eq!(
                truncate("قصير", Some(px(400.)), &metrics, direction, window),
                "قصير"
            );
            let plain = TextMetrics {
                style: TextStyle::default(),
                ..metrics
            };
            assert_eq!(
                truncate(text, Some(max_width), &plain, direction, window),
                text
            );
        });
    }
}
