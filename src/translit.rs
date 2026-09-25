//! Script transliteration — pipeline step 6, decisions 2 and 11 of `cases.md`.
//!
//! Only scripts with a deterministic, dictionary-free romanization are transliterated:
//! Cyrillic and Greek. CJK, Thai, Arabic, Hebrew and Indic are **kept verbatim**, since
//! 東京 -> `tokyo` is not derivable from codepoints. That is decision 2, and it is why
//! the default output alphabet is not `[a-z0-9-]` — `--ascii` is what narrows it.
//!
//! Cyrillic follows **BGN/PCGN** (decision 11), which romanizes to ASCII directly where
//! ISO 9 would emit `šč`. BGN/PCGN is language-specific and we cannot know the language
//! of a filename, so the Russian table is used throughout: `г` is `g`, not Ukrainian
//! `h`. Greek is romanized letter-wise in the same spirit, without BGN/PCGN's
//! context-sensitive digraph rules (`μπ` -> `b`), which need word segmentation.
//!
//! Lookups are keyed on the lowercase letter; [`crate::fold`] restores the case.

/// Romanize one lowercase Cyrillic or Greek letter, or `None` to keep it as-is.
pub(crate) fn translit(c: char) -> Option<&'static str> {
    Some(match c {
        // ── Cyrillic, BGN/PCGN (Russian) ──────────────────────────────────────────
        'а' => "a",
        'б' => "b",
        'в' => "v",
        'г' => "g",
        'д' => "d",
        'е' => "e",
        'ё' => "e",
        'ж' => "zh",
        'з' => "z",
        'и' => "i",
        'й' => "y",
        'к' => "k",
        'л' => "l",
        'м' => "m",
        'н' => "n",
        'о' => "o",
        'п' => "p",
        'р' => "r",
        'с' => "s",
        'т' => "t",
        'у' => "u",
        'ф' => "f",
        'х' => "kh",
        'ц' => "ts",
        'ч' => "ch",
        'ш' => "sh",
        'щ' => "shch",
        'ы' => "y",
        'э' => "e",
        'ю' => "yu",
        'я' => "ya",
        // BGN/PCGN writes the hard and soft signs as `"` and `'`; both would only
        // become separators, so they are dropped.
        'ъ' | 'ь' => "",
        // Other Cyrillic alphabets, best-effort.
        'і' => "i",
        'ї' => "yi",
        'є' => "ye",
        'ґ' => "g",
        'ў' => "w",
        'ј' => "j",
        'љ' => "lj",
        'њ' => "nj",
        'ћ' => "c",
        'ђ' => "d",
        'џ' => "dz",
        'ѕ' => "dz",
        'ѓ' => "g",
        'ќ' => "k",

        // ── Greek ─────────────────────────────────────────────────────────────────
        'α' => "a",
        'β' => "v",
        'γ' => "g",
        'δ' => "d",
        'ε' => "e",
        'ζ' => "z",
        'η' => "i",
        'θ' => "th",
        'ι' => "i",
        'κ' => "k",
        'λ' => "l",
        'μ' => "m",
        'ν' => "n",
        'ξ' => "x",
        'ο' => "o",
        'π' => "p",
        'ρ' => "r",
        // `cases.md` §10: final sigma. Both forms romanize to `s`, so the positional
        // ambiguity that breaks naive lowercasing never reaches the output.
        'σ' | 'ς' => "s",
        'τ' => "t",
        'υ' => "y",
        'φ' => "f",
        'χ' => "ch",
        'ψ' => "ps",
        'ω' => "o",

        _ => return None,
    })
}
