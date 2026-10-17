//! Arabic shaping for platforms that do not shape reordered text: contextual letter forms,
//! the lam-alef ligature, and the clusters a right-to-left run is laid out in.

use std::ops::Range;

/// Keeps a letter from joining the character beside it.
const ZERO_WIDTH_NON_JOINER: char = '\u{200C}';

/// Split a right-to-left run (logical order) into clusters that keep their shape
/// when the clusters are then laid out in reverse order.
///
/// Joined letters (initial, medial and final forms, and the lam-alef ligature) become
/// their presentation-form characters. Isolated letters stay base letters wrapped in
/// zero-width non-joiners: many fonts (Cairo among them) map the joined presentation
/// forms but not the isolated ones, and an isolated base letter draws the same glyph.
/// (Forcing joined forms with zero-width joiners instead breaks in DirectWrite once
/// the run is reversed.) Marks stay in their letter's cluster. Paired punctuation is
/// mirrored.
pub(super) fn joined_clusters(text: &str, font_family: &str) -> Vec<(String, Range<usize>)> {
    struct Cluster {
        /// Starts with the letter (or other character), followed by its marks.
        text: String,
        range: Range<usize>,
        /// For an isolated letter, its isolated presentation form.
        isolated_form: Option<char>,
    }

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

    let mut clusters: Vec<Cluster> = Vec::new();
    let mut index = 0;
    while index < characters.len() {
        let character = characters[index].1;
        let range = range_of(index);

        if is_combining(character) {
            match clusters.last_mut() {
                Some(cluster) => {
                    cluster.text.push(character);
                    cluster.range.end = range.end;
                }
                None => clusters.push(Cluster {
                    text: character.to_string(),
                    range,
                    isolated_form: None,
                }),
            }
            index += 1;
            continue;
        }

        let joined_before = neighbour(index, -1).is_some_and(joins_forward);
        let Some(forms) = forms(character) else {
            clusters.push(Cluster {
                text: mirror(character).to_string(),
                range,
                isolated_form: None,
            });
            index += 1;
            continue;
        };
        let joined_before = joined_before && joins_backward(character);

        // Lam then alef (with only marks between) make one ligature.
        if character == 'ل' {
            if let Some(alef_index) =
                (index + 1..characters.len()).find(|&next| !is_combining(characters[next].1))
            {
                if let Some((isolated, last)) = lam_alef(characters[alef_index].1) {
                    let mut text = String::from(if joined_before { last } else { isolated });
                    // Marks that sat between lam and alef.
                    text.extend((index + 1..alef_index).map(|mark| characters[mark].1));
                    clusters.push(Cluster {
                        text,
                        range: range.start..range_of(alef_index).end,
                        isolated_form: None,
                    });
                    index = alef_index + 1;
                    continue;
                }
            }
        }

        let joined_after =
            forms.joining == Joining::Dual && neighbour(index, 1).is_some_and(joins_backward);
        let (text, isolated_form) = match (joined_before, joined_after) {
            // Decided below, once the neighbouring letters are known.
            (false, false) => (character.to_string(), Some(forms.isolated)),
            (true, false) => (forms.last.to_string(), None),
            (false, true) => (forms.first.to_string(), None),
            (true, true) => (forms.middle.to_string(), None),
        };
        clusters.push(Cluster {
            text,
            range,
            isolated_form,
        });
        index += 1;
    }

    // An isolated letter is best drawn as its base letter: the presentation forms
    // around it do not join, so the font's own shaping draws it isolated, dots and
    // all (some fonts map isolated presentation forms to dotless shapes, and many do
    // not map them at all). Two base letters side by side would join, though, so in a
    // run of isolated letters every other one uses its presentation form, or, if the
    // font lacks it, is fenced off with zero-width non-joiners.
    let mut start = 0;
    while start < clusters.len() {
        if clusters[start].isolated_form.is_none() {
            start += 1;
            continue;
        }
        let end = (start..clusters.len())
            .find(|&index| clusters[index].isolated_form.is_none())
            .unwrap_or(clusters.len());
        // Keep the letters whose isolated forms are most often dotless as base letters.
        let prefers_base = |index: usize| {
            matches!(
                clusters[index].text.chars().next(),
                Some('ن' | 'ي' | 'ی' | 'ة')
            )
        };
        let score = |parity: usize| {
            (start..end)
                .filter(|index| (index - start) % 2 == parity && prefers_base(*index))
                .count()
        };
        let base_parity = usize::from(score(1) > score(0));
        for (offset, cluster) in clusters[start..end].iter_mut().enumerate() {
            if offset % 2 == base_parity {
                continue;
            }
            let form = cluster.isolated_form.unwrap_or_default();
            let marks: String = cluster.text.chars().skip(1).collect();
            cluster.text = if crate::fonts::lacks_presentation_form(font_family, form) {
                let letter = cluster.text.chars().next().unwrap_or_default();
                format!("{ZERO_WIDTH_NON_JOINER}{letter}{marks}{ZERO_WIDTH_NON_JOINER}")
            } else {
                format!("{form}{marks}")
            };
        }
        start = end;
    }

    clusters
        .into_iter()
        .map(|cluster| (cluster.text, cluster.range))
        .collect()
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

pub(super) fn is_rtl_char(character: char) -> bool {
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

/// Every presentation-form character [`visual_line`] can emit, to check a font's
/// coverage against.
#[doc(hidden)]
pub fn presentation_forms() -> Vec<char> {
    let letters = "آأؤإئابةتثجحخدذرزسشصضطظعغفقكلمنهوىيپچژکگی";
    let mut characters: Vec<char> = letters
        .chars()
        .filter_map(forms)
        .flat_map(|forms| [forms.isolated, forms.last, forms.first, forms.middle])
        .collect();
    characters.extend(
        "آأإا"
            .chars()
            .filter_map(lam_alef)
            .flat_map(|(isolated, last)| [isolated, last]),
    );
    characters.sort_unstable();
    characters.dedup();
    characters
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
#[must_use]
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
