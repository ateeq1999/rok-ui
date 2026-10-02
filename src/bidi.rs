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
//! ```ignore
//! div().child(BidiText::new("مرحبا بك في rok-ui"))
//! ```

use std::ops::Range;

use gpui::SharedString;

use crate::components::direction::TextDirection;

/// Starts a left-to-right override, so the platform does no reordering of its own.
const LEFT_TO_RIGHT_OVERRIDE: char = '\u{202D}';
/// Ends the override.
const POP_DIRECTIONAL_FORMATTING: char = '\u{202C}';

/// Whether this platform needs [`visual_text`] applied before drawing.
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
pub fn is_converted(text: &str) -> bool {
    text.starts_with(LEFT_TO_RIGHT_OVERRIDE)
}

/// Reorder `text` for a renderer that draws every character left to right, on any
/// platform. Each line is a separate paragraph with base `direction`.
pub fn visual_text(text: &str, direction: TextDirection) -> String {
    text.split('\n')
        .map(|line| visual_line(line, direction).text)
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
pub fn visual_line(line: &str, direction: TextDirection) -> VisualLine {
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
                    push_rtl_run(line, run, &mut text, &mut chars);
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
fn push_rtl_run(line: &str, run: Range<usize>, text: &mut String, chars: &mut Vec<VisualChar>) {
    let mut clusters: Vec<(String, Range<usize>)> = Vec::new();
    for (character, range) in shape_arabic_pieces(&line[run.clone()]) {
        match clusters.last_mut() {
            Some((cluster, cluster_range)) if is_combining(character) => {
                cluster.push(character);
                cluster_range.start = cluster_range.start.min(range.start);
                cluster_range.end = cluster_range.end.max(range.end);
            }
            _ => clusters.push((mirror(character).to_string(), range)),
        }
    }
    for (cluster, range) in clusters.iter().rev() {
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

fn mirror(character: char) -> char {
    match character {
        '(' => ')',
        ')' => '(',
        '[' => ']',
        ']' => '[',
        '{' => '}',
        '}' => '{',
        '<' => '>',
        '>' => '<',
        '«' => '»',
        '»' => '«',
        '‹' => '›',
        '›' => '‹',
        other => other,
    }
}

fn is_rtl_char(character: char) -> bool {
    matches!(character as u32,
        0x0590..=0x08FF      // Hebrew, Arabic, Syriac, Thaana, NKo, Samaritan, Arabic Extended
        | 0xFB1D..=0xFDFF    // Hebrew and Arabic presentation forms A
        | 0xFE70..=0xFEFF    // Arabic presentation forms B
        | 0x10800..=0x10FFF  // Historic right-to-left scripts
        | 0x1E800..=0x1EFFF) // Mende Kikakui, Adlam, Arabic mathematical symbols
}

/// Marks that attach to the previous character and are skipped when deciding
/// how neighbouring letters join.
fn is_combining(character: char) -> bool {
    matches!(character as u32,
        0x0300..=0x036F      // Combining diacritics
        | 0x0591..=0x05BD | 0x05BF | 0x05C1..=0x05C2 | 0x05C4..=0x05C5 | 0x05C7 // Hebrew points
        | 0x0610..=0x061A    // Arabic signs
        | 0x064B..=0x065F    // Arabic harakat
        | 0x0670             // Superscript alef
        | 0x06D6..=0x06DC | 0x06DF..=0x06E4 | 0x06E7..=0x06E8 | 0x06EA..=0x06ED
        | 0x200C..=0x200D    // Zero-width (non-)joiner
        | 0xFE00..=0xFE0F) // Variation selectors
}

/// How a letter connects to its neighbours.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Joining {
    /// Connects only to the letter before it (alef, dal, reh, waw...).
    Right,
    /// Connects on both sides.
    Dual,
}

/// Presentation forms (isolated, final, initial, medial) of a letter. Right-joining
/// letters have no initial or medial form.
struct Forms {
    joining: Joining,
    isolated: char,
    last: char,
    first: char,
    middle: char,
}

fn forms(letter: char) -> Option<Forms> {
    let right = |isolated: u32| Forms {
        joining: Joining::Right,
        isolated: char_at(isolated),
        last: char_at(isolated + 1),
        first: char_at(isolated),
        middle: char_at(isolated + 1),
    };
    let dual = |isolated: u32| Forms {
        joining: Joining::Dual,
        isolated: char_at(isolated),
        last: char_at(isolated + 1),
        first: char_at(isolated + 2),
        middle: char_at(isolated + 3),
    };
    Some(match letter {
        'آ' => right(0xFE81),
        'أ' => right(0xFE83),
        'ؤ' => right(0xFE85),
        'إ' => right(0xFE87),
        'ئ' => dual(0xFE89),
        'ا' => right(0xFE8D),
        'ب' => dual(0xFE8F),
        'ة' => right(0xFE93),
        'ت' => dual(0xFE95),
        'ث' => dual(0xFE99),
        'ج' => dual(0xFE9D),
        'ح' => dual(0xFEA1),
        'خ' => dual(0xFEA5),
        'د' => right(0xFEA9),
        'ذ' => right(0xFEAB),
        'ر' => right(0xFEAD),
        'ز' => right(0xFEAF),
        'س' => dual(0xFEB1),
        'ش' => dual(0xFEB5),
        'ص' => dual(0xFEB9),
        'ض' => dual(0xFEBD),
        'ط' => dual(0xFEC1),
        'ظ' => dual(0xFEC5),
        'ع' => dual(0xFEC9),
        'غ' => dual(0xFECD),
        'ف' => dual(0xFED1),
        'ق' => dual(0xFED5),
        'ك' => dual(0xFED9),
        'ل' => dual(0xFEDD),
        'م' => dual(0xFEE1),
        'ن' => dual(0xFEE5),
        'ه' => dual(0xFEE9),
        'و' => right(0xFEED),
        'ى' => right(0xFEEF),
        'ي' => dual(0xFEF1),
        // Persian and Urdu letters.
        'پ' => dual(0xFB56),
        'چ' => dual(0xFB7A),
        'ژ' => right(0xFB8A),
        'ک' => dual(0xFB8E),
        'گ' => dual(0xFB92),
        'ی' => dual(0xFBFC),
        _ => return None,
    })
}

fn char_at(code: u32) -> char {
    char::from_u32(code).unwrap_or('\u{FFFD}')
}

/// Lam followed by an alef form one ligature: (isolated, final).
fn lam_alef(alef: char) -> Option<(char, char)> {
    match alef {
        'آ' => Some(('\u{FEF5}', '\u{FEF6}')),
        'أ' => Some(('\u{FEF7}', '\u{FEF8}')),
        'إ' => Some(('\u{FEF9}', '\u{FEFA}')),
        'ا' => Some(('\u{FEFB}', '\u{FEFC}')),
        _ => None,
    }
}

/// Tatweel (kashida) stretches a connection and joins on both sides.
const TATWEEL: char = 'ـ';

/// Whether `letter` connects to the letter after it.
fn joins_forward(letter: char) -> bool {
    letter == TATWEEL || forms(letter).is_some_and(|forms| forms.joining == Joining::Dual)
}

/// Whether `letter` connects to the letter before it.
fn joins_backward(letter: char) -> bool {
    letter == TATWEEL || forms(letter).is_some()
}

/// Replace Arabic and Persian letters in `text` (logical order) with the
/// presentation form their neighbours call for, merging lam-alef ligatures.
pub fn shape_arabic(text: &str) -> String {
    shape_arabic_pieces(text)
        .into_iter()
        .map(|(character, _)| character)
        .collect()
}

/// [`shape_arabic`], with the byte range in `text` each output character came from.
/// A lam-alef ligature covers both letters (and any marks between them, which are
/// also output on their own right after it).
fn shape_arabic_pieces(text: &str) -> Vec<(char, Range<usize>)> {
    let characters: Vec<(usize, char)> = text.char_indices().collect();
    let range_of = |index: usize| {
        let (start, character) = characters[index];
        start..start + character.len_utf8()
    };
    // The nearest non-combining character before and after each position.
    let neighbour = |from: usize, step: isize| -> Option<char> {
        let mut index = from as isize + step;
        while index >= 0 && (index as usize) < characters.len() {
            let character = characters[index as usize].1;
            if !is_combining(character) {
                return Some(character);
            }
            index += step;
        }
        None
    };

    let mut out = Vec::with_capacity(characters.len());
    let mut index = 0;
    while index < characters.len() {
        let character = characters[index].1;
        let joined_before = neighbour(index, -1).is_some_and(joins_forward);

        if character == 'ل' {
            if let Some(next_index) =
                (index + 1..characters.len()).find(|&next| !is_combining(characters[next].1))
            {
                if let Some((isolated, last)) = lam_alef(characters[next_index].1) {
                    let ligature = if joined_before { last } else { isolated };
                    out.push((ligature, range_of(index).start..range_of(next_index).end));
                    // Keep marks that sat between lam and alef.
                    out.extend(
                        (index + 1..next_index).map(|mark| (characters[mark].1, range_of(mark))),
                    );
                    index = next_index + 1;
                    continue;
                }
            }
        }

        let Some(forms) = forms(character) else {
            out.push((character, range_of(index)));
            index += 1;
            continue;
        };
        let joined_before = joined_before && joins_backward(character);
        let joined_after =
            forms.joining == Joining::Dual && neighbour(index, 1).is_some_and(joins_backward);
        let form = match (joined_before, joined_after) {
            (false, false) => forms.isolated,
            (true, false) => forms.last,
            (false, true) => forms.first,
            (true, true) => forms.middle,
        };
        out.push((form, range_of(index)));
        index += 1;
    }
    out
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
