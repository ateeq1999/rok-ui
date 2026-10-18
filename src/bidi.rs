//! Right-to-left and mixed-direction text (Arabic, Persian, Hebrew).
//!
//! GPUI 0.2.2 shapes Arabic correctly on every platform, but its Windows text
//! backend lays out every glyph run left to right, so right-to-left words come out
//! mirrored. macOS and Linux place glyphs where the shaper says and need no help.
//!
//! On Windows, [`display_text`] works around it: it runs the Unicode Bidirectional
//! Algorithm, converts Arabic and Persian letters to their contextual presentation
//! forms, reverses each right-to-left run, and wraps the result in a left-to-right
//! override so DirectWrite draws it exactly in that order. On other platforms it
//! returns the text unchanged.
//!
//! rok-ui components apply this to the text they render. Use [`BidiText`](crate::components::BidiText)
//! (or [`display_text`] for single-line strings) for text you render yourself:
//!
//! ```no_run
//! # use rok_ui::prelude::*;
//! # fn example(window: &mut Window, cx: &mut App) {
//! let greeting = div().child(BidiText::new("مرحبا بك في rok-ui"));
//! # }
//! ```

use std::ops::Range;

use gpui::SharedString;

use crate::components::direction::TextDirection;

mod arabic;

use arabic::{is_rtl_char, joined_clusters};
pub use arabic::{presentation_forms, shape_arabic};

/// Starts a left-to-right override, so the platform does no reordering of its own.
const LEFT_TO_RIGHT_OVERRIDE: char = '\u{202D}';
/// Ends the override.
const POP_DIRECTIONAL_FORMATTING: char = '\u{202C}';

/// Whether this platform needs [`visual_text`] applied before drawing.
#[must_use]
pub const fn platform_needs_reordering() -> bool {
    cfg!(target_os = "windows")
}

/// Whether `text` contains any right-to-left letter.
pub fn has_rtl(text: &str) -> bool {
    text.chars().any(is_rtl_char)
}

/// `text` ready to hand to GPUI for one line of display, in a paragraph that runs
/// in `direction`. Unchanged when it has no right-to-left letters, on platforms that
/// handle bidirectional text natively, or when it is already converted.
pub fn display_text(text: impl Into<SharedString>, direction: TextDirection) -> SharedString {
    let text = text.into();
    if !platform_needs_reordering() || !has_rtl(&text) || is_converted(&text) {
        return text;
    }
    visual_text(&text, direction).into()
}

/// Whether `text` is already the output of [`visual_text`].
#[must_use]
pub fn is_converted(text: &str) -> bool {
    text.starts_with(LEFT_TO_RIGHT_OVERRIDE)
}

/// Reorder `text` for a renderer that draws every character left to right, on any
/// platform. Each line is a separate paragraph with base `direction`.
#[must_use]
pub fn visual_text(text: &str, direction: TextDirection) -> String {
    visual_text_in_font(text, direction, &crate::sx::current_theme().font_family)
}

/// [`visual_text`] for text drawn in `font_family`, so letters avoid any
/// presentation forms that family is missing (see [`crate::fonts`]).
#[must_use]
pub fn visual_text_in_font(text: &str, direction: TextDirection, font_family: &str) -> String {
    text.split('\n')
        .map(|line| visual_line_in_font(line, direction, font_family).text)
        .collect::<Vec<_>>()
        .join("\n")
}

/// One logical character of a [`VisualLine`] and where it ended up.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VisualChar {
    /// Byte range in the logical (original) text.
    pub logical: Range<usize>,
    /// Byte range in [`VisualLine::text`] of the glyph cluster that draws it. Letters
    /// joined into one ligature (lam-alef) share a cluster.
    pub visual: Range<usize>,
    /// Whether the character sits in a right-to-left run.
    pub rtl: bool,
}

/// A line reordered for left-to-right drawing, with a map back to the logical text
/// for cursors, selections and hit testing.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VisualLine {
    /// The text to draw, wrapped in a left-to-right override.
    pub text: String,
    /// Every logical character, in logical order.
    pub chars: Vec<VisualChar>,
}

/// Reorder one line (no newlines) and record where each character went.
#[must_use]
pub fn visual_line(line: &str, direction: TextDirection) -> VisualLine {
    visual_line_in_font(line, direction, &crate::sx::current_theme().font_family)
}

/// [`visual_line`] for text drawn in `font_family`.
#[must_use]
pub fn visual_line_in_font(line: &str, direction: TextDirection, font_family: &str) -> VisualLine {
    use unicode_bidi::{BidiInfo, Level};

    let mut text = String::with_capacity(line.len() * 3 + 6);
    let mut chars = Vec::with_capacity(line.len());
    text.push(LEFT_TO_RIGHT_OVERRIDE);
    if !line.is_empty() {
        let base = match direction {
            TextDirection::Rtl => Level::rtl(),
            TextDirection::Ltr => Level::ltr(),
        };
        let info = BidiInfo::new(line, Some(base));
        for paragraph in &info.paragraphs {
            let (levels, runs) = info.visual_runs(paragraph, paragraph.range.clone());
            for run in runs {
                if levels[run.start].is_rtl() {
                    push_rtl_run(line, run, font_family, &mut text, &mut chars);
                } else {
                    for (offset, character) in line[run.clone()].char_indices() {
                        let start = text.len();
                        text.push(character);
                        let logical = run.start + offset;
                        chars.push(VisualChar {
                            logical: logical..logical + character.len_utf8(),
                            visual: start..text.len(),
                            rtl: false,
                        });
                    }
                }
            }
        }
    }
    text.push(POP_DIRECTIONAL_FORMATTING);
    chars.sort_by_key(|character| character.logical.start);
    VisualLine { text, chars }
}

/// Append a right-to-left run: letters in their contextual forms, clusters (a
/// character plus its combining marks) in reverse order, paired punctuation mirrored.
fn push_rtl_run(
    line: &str,
    run: Range<usize>,
    font_family: &str,
    text: &mut String,
    chars: &mut Vec<VisualChar>,
) {
    for (cluster, range) in joined_clusters(&line[run.clone()], font_family)
        .iter()
        .rev()
    {
        let start = text.len();
        text.push_str(cluster);
        let visual = start..text.len();
        let logical_start = run.start + range.start;
        for (offset, character) in line[logical_start..run.start + range.end].char_indices() {
            let logical = logical_start + offset;
            chars.push(VisualChar {
                logical: logical..logical + character.len_utf8(),
                visual: visual.clone(),
                rtl: true,
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn visual(text: &str, direction: TextDirection) -> String {
        visual_text(text, direction)
            .trim_start_matches(LEFT_TO_RIGHT_OVERRIDE)
            .trim_end_matches(POP_DIRECTIONAL_FORMATTING)
            .to_string()
    }

    #[test]
    fn letters_take_their_contextual_forms() {
        // مرحبا: initial meem, final reh, initial hah, medial beh, final alef.
        assert_eq!(
            shape_arabic("مرحبا"),
            "\u{FEE3}\u{FEAE}\u{FEA3}\u{FE92}\u{FE8E}"
        );
        // A lone letter keeps its isolated form; hamza never joins.
        assert_eq!(shape_arabic("ب"), "\u{FE8F}");
        assert_eq!(shape_arabic("ء"), "ء");
    }

    #[test]
    fn lam_alef_becomes_one_ligature() {
        assert_eq!(shape_arabic("لا"), "\u{FEFB}");
        // After a joining letter the ligature takes its final form: سلام.
        assert_eq!(shape_arabic("سلام"), "\u{FEB3}\u{FEFC}\u{FEE1}");
    }

    #[test]
    fn harakat_do_not_break_joining() {
        // بَب: the fatha sits on the first beh, which still joins the second.
        assert_eq!(shape_arabic("بَب"), "\u{FE91}\u{064E}\u{FE90}");
    }

    #[test]
    fn rtl_runs_are_reversed_and_ltr_runs_kept() {
        assert_eq!(
            visual("مرحبا", TextDirection::Rtl),
            "\u{FE8E}\u{FE92}\u{FEA3}\u{FEAE}\u{FEE3}"
        );
        // In a right-to-left paragraph the English word sits to the left.
        let mixed = visual("مرحبا Rust", TextDirection::Rtl);
        assert!(mixed.starts_with("Rust "), "{mixed:?}");
        // Numbers keep their left-to-right digit order.
        let price = visual("السعر 45", TextDirection::Rtl);
        assert!(price.starts_with("45 "), "{price:?}");
    }

    #[test]
    fn brackets_mirror_inside_rtl_runs() {
        let text = visual("(نص)", TextDirection::Rtl);
        assert!(text.starts_with('(') && text.ends_with(')'), "{text:?}");
    }

    #[test]
    fn plain_ltr_text_is_untouched() {
        assert!(!has_rtl("Hello, world"));
        assert_eq!(
            display_text("Hello", TextDirection::Rtl),
            SharedString::from("Hello")
        );
    }

    #[test]
    fn visual_lines_map_every_logical_character() {
        let line = visual_line("مرحبا", TextDirection::Rtl);
        assert_eq!(line.chars.len(), 5);
        assert!(line.chars.iter().all(|character| character.rtl));
        // The first letter is drawn last (rightmost), the last letter first.
        assert!(line.chars[0].visual.start > line.chars[4].visual.start);

        // Both letters of a lam-alef ligature map to the one glyph.
        let ligature = visual_line("لا", TextDirection::Rtl);
        assert_eq!(ligature.chars.len(), 2);
        assert_eq!(ligature.chars[0].visual, ligature.chars[1].visual);

        // Left-to-right characters keep their order and are not flagged.
        let mixed = visual_line("ab مر", TextDirection::Ltr);
        assert!(!mixed.chars[0].rtl && !mixed.chars[1].rtl);
        assert!(mixed.chars[0].visual.start < mixed.chars[1].visual.start);
        assert!(mixed.chars[3].rtl);
        // Logical byte ranges tile the original text.
        assert_eq!(mixed.chars.last().unwrap().logical.end, "ab مر".len());
    }

    #[test]
    fn adjacent_isolated_letters_alternate_base_and_presentation_forms() {
        // و، ز and ن are all isolated. Plain base letters draw best (the font shapes them,
        // dots included) but two side by side would join, so the middle one uses its
        // presentation form.
        let line = visual_line_in_font("وزن", TextDirection::Rtl, "Any Font");
        let text = line.text.trim_start_matches(LEFT_TO_RIGHT_OVERRIDE);
        let text = text.trim_end_matches(POP_DIRECTIONAL_FORMATTING);
        assert_eq!(text, "ن\u{FEAF}و");

        // A font without that form gets the base letter fenced off with non-joiners.
        crate::fonts::mark_missing_presentation_forms("Gappy Font", ['\u{FEAF}']);
        let line = visual_line_in_font("وزن", TextDirection::Rtl, "Gappy Font");
        let text = line.text.trim_start_matches(LEFT_TO_RIGHT_OVERRIDE);
        let text = text.trim_end_matches(POP_DIRECTIONAL_FORMATTING);
        assert_eq!(text, "ن\u{200C}ز\u{200C}و");
    }

    #[test]
    fn conversion_is_idempotent() {
        let once = visual_text("مرحبا", TextDirection::Rtl);
        assert!(is_converted(&once));
        if platform_needs_reordering() {
            assert_eq!(
                display_text(once.clone(), TextDirection::Rtl).as_ref(),
                once
            );
        }
    }
}
