//! Character folding — pipeline steps 2, 4, 9 and decision 12 of `cases.md`.
//!
//! Everything here runs *before* word-boundary detection, so every replacement has to
//! preserve the case signal of the character it replaces. A multi-character expansion
//! of an uppercase letter is therefore emitted as `Ae` or `AE` depending on its
//! neighbours — `Æther` must become `aether`, and `ÆTHER` must not become `a-ether`.

use unicode_normalization::char::decompose_canonical;

use crate::translit;

/// Characters deleted outright at step 2: controls, bidi overrides, joiners, and the
/// rest of the invisible zoo from `cases.md` §11.
///
/// Stripping these first is what stops `photo\u{202e}gpj.txt` from renaming to
/// something that renders as a different name than it is.
pub(crate) fn is_ignorable(c: char) -> bool {
    // Whitespace controls are the exception: a tab or a newline inside a filename is
    // a word gap, and `cases.md` §1 wants `my\tfile.md` to become `my-file.md`. They
    // fall through to step 10, which turns them into a separator.
    if c.is_control() && !c.is_whitespace() {
        return true;
    }
    // For the same reason U+200B is absent below: it is a zero-width *space*, and §1
    // groups it with NBSP. The joiners and marks around it carry no width and no gap,
    // so they are deleted outright.
    matches!(c as u32,
        0x00AD                              // soft hyphen
        | 0x0600..=0x0605 | 0x061C | 0x06DD | 0x070F | 0x08E2  // Arabic format chars
        | 0x180E                            // Mongolian vowel separator
        | 0x200C..=0x200F                   // ZWNJ, ZWJ, LRM, RLM
        | 0x202A..=0x202E                   // bidi embedding / override
        | 0x2060..=0x2064 | 0x2066..=0x206F // word joiner, invisible operators, bidi isolates
        | 0xFEFF                            // BOM / zero-width no-break space
        | 0xFFF9..=0xFFFB                   // interlinear annotation
        | 0xFFFE | 0xFFFF                   // noncharacters
        | 0x110BD | 0x110CD
        | 0x13430..=0x1343F
        | 0x1BCA0..=0x1BCA3
        | 0x1D173..=0x1D17A
        | 0xE0000..=0xE007F                 // tag characters
    )
}

/// Letters with no canonical decomposition, which the standard "NFD then drop combining
/// marks" trick passes through untouched — and step 10 then deletes, turning `Łódź.md`
/// into `d.md`. This table is what prevents that (`cases.md` §9, trap 2).
///
/// Values are the *lowercase* expansion; [`fold`] re-cases them.
fn special_fold(c: char) -> Option<&'static str> {
    Some(match c {
        'ß' | 'ẞ' => "ss",
        'ł' | 'Ł' => "l",
        'ø' | 'Ø' | 'ǿ' | 'Ǿ' => "o",
        'đ' | 'Đ' | 'ð' | 'Ð' => "d",
        'þ' | 'Þ' => "th",
        'ħ' | 'Ħ' => "h",
        'ŋ' | 'Ŋ' => "ng",
        'ı' | 'İ' | 'ɨ' => "i",
        'æ' | 'Æ' | 'ǣ' | 'Ǣ' | 'ǽ' | 'Ǽ' => "ae",
        'œ' | 'Œ' => "oe",
        'ŧ' | 'Ŧ' => "t",
        'ĸ' => "k",
        'ǥ' | 'Ǥ' | 'ǧ' | 'Ǧ' => "g",
        'ƒ' | 'Ƒ' => "f",
        'ȼ' | 'Ȼ' => "c",
        'ɇ' | 'Ɇ' => "e",
        'ɍ' | 'Ɍ' => "r",
        'ƶ' | 'Ƶ' => "z",
        'ɏ' | 'Ɏ' => "y",
        'ſ' => "s",
        'µ' => "u",
        _ => return None,
    })
}

/// Step 4 and step 6 together: special folds and script transliteration, both applied
/// before NFD so that a transliterated letter is not first mangled by mark-stripping.
///
/// Running transliteration here rather than at its nominal step 6 matters for exactly
/// one reason: `й` is `и` plus a breve, so NFD-then-strip would silently turn BGN/PCGN
/// `y` into `i`. Looking the character up whole, first, keeps the romanization honest.
pub(crate) fn fold(s: &str) -> String {
    let chars: Vec<char> = s.chars().collect();
    let mut out = String::with_capacity(s.len());

    for (i, &c) in chars.iter().enumerate() {
        let Some(rep) = lookup(c) else {
            out.push(c);
            continue;
        };

        if !c.is_uppercase() {
            out.push_str(rep);
        } else if rep.chars().count() == 1 {
            out.extend(rep.chars().flat_map(char::to_uppercase));
        } else if adjacent_uppercase(&chars, i) {
            // `ЖУК` -> `ZHUK`. Title-casing here would give `ZhUK`, and step 7 would
            // then read the `h`->`U` transition as a word boundary.
            out.extend(rep.chars().flat_map(char::to_uppercase));
        } else {
            // `Жук` -> `Zhuk`, `Æther` -> `Aether`.
            let mut cs = rep.chars();
            if let Some(first) = cs.next() {
                out.extend(first.to_uppercase());
            }
            out.extend(cs);
        }
    }

    out
}

/// One fold lookup: explicit table, then the transliteration tables, then the same
/// tables again against the character's canonical base (`ά` -> `α` -> `a`).
fn lookup(c: char) -> Option<&'static str> {
    let lower = c.to_lowercase().next().unwrap_or(c);
    special_fold(c).or_else(|| translit::translit(lower)).or_else(|| {
        let base = base_char(c)?;
        if base == c {
            return None;
        }
        special_fold(base).or_else(|| translit::translit(base.to_lowercase().next()?))
    })
}

/// The first character of `c`'s canonical decomposition — its base letter.
fn base_char(c: char) -> Option<char> {
    let mut first = None;
    decompose_canonical(c, |d| {
        if first.is_none() {
            first = Some(d);
        }
    });
    first
}

/// Does an uppercase letter sit next to position `i`? Decides between `AE` and `Ae`.
fn adjacent_uppercase(chars: &[char], i: usize) -> bool {
    if matches!(chars.get(i + 1), Some(n) if n.is_lowercase()) {
        return false;
    }
    matches!(chars.get(i + 1), Some(n) if n.is_uppercase())
        || (i > 0 && chars[i - 1].is_uppercase())
}

/// Decimal digits from every script, mapped to ASCII (`cases.md` decision 12).
///
/// NFKC covers fullwidth `３` but leaves Arabic-Indic `٣` and Devanagari `३` alone,
/// and `char::to_digit` is ASCII-only — hence the explicit table. Each entry is the
/// codepoint of that script's zero; its digits are always the nine that follow.
const DIGIT_ZEROS: &[u32] = &[
    0x0660, 0x06F0, 0x07C0, 0x0966, 0x09E6, 0x0A66, 0x0AE6, 0x0B66, 0x0BE6, 0x0C66, 0x0CE6, 0x0D66,
    0x0DE6, 0x0E50, 0x0ED0, 0x0F20, 0x1040, 0x1090, 0x17E0, 0x1810, 0x1946, 0x19D0, 0x1A80, 0x1A90,
    0x1B50, 0x1BB0, 0x1C40, 0x1C50, 0xA620, 0xA8D0, 0xA900, 0xA9D0, 0xA9F0, 0xAA50, 0xABF0, 0xFF10,
];

pub(crate) fn ascii_digits(s: &str) -> String {
    s.chars()
        .map(|c| {
            let cp = c as u32;
            DIGIT_ZEROS
                .iter()
                .find(|&&zero| (zero..zero + 10).contains(&cp))
                .and_then(|&zero| char::from_digit(cp - zero, 10))
                .unwrap_or(c)
        })
        .collect()
}

/// Tokens whose punctuation carries the meaning, from `cases.md` decision 8. Longest
/// first, so `c++` is not shadowed by a shorter prefix.
const TOKENS: &[(&str, &str)] =
    &[("c++", "cpp"), ("c#", "csharp"), ("f#", "fsharp"), (".net", "dotnet")];

/// Step 9, pulled one step earlier than its nominal position so that step 10 does not
/// delete the symbols before they are read. Expansions are wrapped in `-`, which step
/// 10 turns into the configured separator: `A&B` becomes `a-and-b`, not `aand-b`.
pub(crate) fn expand_tokens(s: &str) -> String {
    // Indexed by character, not by byte: `Ⱥ` is two bytes and its lowercase `ⱥ` is
    // three, so a lowercased copy of the string does not share the original's offsets.
    let chars: Vec<char> = s.chars().collect();
    let mut out = String::with_capacity(s.len());
    let mut i = 0;

    'outer: while i < chars.len() {
        for &(pat, rep) in TOKENS {
            let len = pat.chars().count();
            if matches_at(&chars, i, pat) && is_token(&chars, i, pat, len) {
                out.push('-');
                out.push_str(rep);
                out.push('-');
                i += len;
                continue 'outer;
            }
        }
        match chars[i] {
            // Decision 7: `&` becomes a word.
            '&' => out.push_str("-and-"),
            // `Don't Stop` -> `dont-stop`: an apostrophe is deleted, not separated
            // (`cases.md` §5).
            '\'' | '\u{2019}' | '\u{2018}' | '\u{02BC}' | '`' => {}
            c => out.push(c),
        }
        i += 1;
    }

    out
}

/// Case-insensitive match of an all-ASCII pattern at character position `i`.
fn matches_at(chars: &[char], i: usize, pat: &str) -> bool {
    pat.chars()
        .enumerate()
        .all(|(n, p)| matches!(chars.get(i + n), Some(&c) if c.is_ascii() && c.to_ascii_lowercase() == p))
}

/// Is the match at `i` a token rather than a fragment of a longer word?
///
/// `Asp.Net.dll` must expand and `x.network.md` must not, so the right edge rejects a
/// following letter (but allows a digit, for `.net5` and `C++11`). The left edge only
/// applies when the pattern itself starts with a letter.
fn is_token(chars: &[char], i: usize, pat: &str, len: usize) -> bool {
    let left = !pat.starts_with(|c: char| c.is_ascii_alphanumeric())
        || i == 0
        || !chars[i - 1].is_ascii_alphanumeric();
    let right = !matches!(chars.get(i + len), Some(c) if c.is_ascii_alphabetic());
    left && right
}
