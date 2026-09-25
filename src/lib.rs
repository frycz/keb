//! Rename files to kebab case, safely and idempotently.
//!
//! This crate is the **transform layer**: a pure `String -> String` function with no
//! filesystem access. The binary (`keb`) wraps it with the stateful filesystem layer.
//! See `design/cases.md` for the full specification.
//!
//! ```
//! use keb::kebab;
//!
//! assert_eq!(kebab("My File.md"),      "my-file.md");
//! assert_eq!(kebab("XMLHttpRequest"),  "xml-http-request");
//! assert_eq!(kebab("my___file.MD"),    "my-file.md");
//! assert_eq!(kebab("Żółć Ćma.md"),     "zolc-cma.md");
//! ```
//!
//! # Invariants
//!
//! 1. **Idempotent** — `kebab(kebab(x)) == kebab(x)` for all `x`
//! 2. **Total** — never panics, for any input
//!
//! Injectivity (distinct inputs never collide) is deliberately *not* a property of
//! this layer: `My File` and `my_file` both produce `my-file`. The filesystem layer
//! restores it by suffixing, with [`with_ordinal`]. See `cases.md` — "Architecture:
//! two layers".
//!
//! # Refusal
//!
//! An empty return value means **there is no kebab name for this input** — every
//! character was punctuation, emoji or invisible, or the extension alone exceeds the
//! length budget. Callers should leave such a file alone rather than invent a name;
//! renaming `🚀.md` to `untitled.md` loses the only thing that distinguished it.
//!
//! [`with_ordinal`]: fn.with_ordinal.html

#![forbid(unsafe_code)]

mod fold;
mod translit;

use unicode_normalization::UnicodeNormalization;
use unicode_normalization::char::is_combining_mark;
use unicode_segmentation::UnicodeSegmentation;

/// Longest filename the transform will emit, in bytes *and* in UTF-16 code units.
///
/// 255 is `NAME_MAX` on ext4 (bytes) and the limit on APFS and NTFS (UTF-16 code
/// units). Both are enforced, so a name is portable across all three (`cases.md` §12).
pub const DEFAULT_MAX_LENGTH: usize = 255;

/// The three knobs `cases.md` §8 admits into the transform layer.
///
/// Everything else the tool can do lives in the filesystem layer and never changes
/// what a name transforms to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Options {
    /// Emitted between words. `-` unless you have a reason.
    pub separator: char,
    /// Narrow the output alphabet to ASCII, dropping the scripts that decision 2
    /// otherwise keeps verbatim (CJK, Thai, Arabic, Hebrew, Indic).
    pub ascii: bool,
    /// Length budget, counted in bytes and in UTF-16 code units.
    pub max_length: usize,
}

impl Default for Options {
    fn default() -> Self {
        Options { separator: '-', ascii: false, max_length: DEFAULT_MAX_LENGTH }
    }
}

/// Compound extensions kept whole. Everything else splits on the last dot.
///
/// From `cases.md`, decision 6.
const COMPOUND_EXTS: &[&str] = &[
    "tar.gz",
    "tar.bz2",
    "tar.xz",
    "tar.zst",
    "d.ts",
    "d.mts",
    "d.cts",
    "min.js",
    "min.css",
    "module.css",
    "module.scss",
];

/// Second-to-last extension components that bind to whatever follows them, so that
/// `Component.Test.tsx` keeps `.test.tsx` (decision 6).
const COMPOUND_HEADS: &[&str] = &["test", "spec", "stories"];

/// Convert a filename to kebab case, preserving its extension.
///
/// ```
/// use keb::kebab;
///
/// assert_eq!(kebab("Report (Final).md"), "report-final.md");
/// assert_eq!(kebab("My.Archive.tar.gz"), "my-archive.tar.gz");
/// assert_eq!(kebab(".gitignore"),        ".gitignore");
/// ```
pub fn kebab(name: &str) -> String {
    kebab_with(name, &Options::default())
}

/// [`kebab`], with the separator, output alphabet and length budget spelled out.
///
/// ```
/// use keb::{kebab_with, Options};
///
/// let snake = Options { separator: '_', ..Options::default() };
/// assert_eq!(kebab_with("My File.md", &snake), "my_file.md");
///
/// let ascii = Options { ascii: true, ..Options::default() };
/// assert_eq!(kebab_with("日本語 Report.md", &Options::default()), "日本語-report.md");
/// assert_eq!(kebab_with("日本語 Report.md", &ascii),              "report.md");
/// ```
pub fn kebab_with(name: &str, opts: &Options) -> String {
    let Some((prefix, stem, ext)) = parts(name, opts) else {
        return name.to_string();
    };
    assemble(prefix, &stem, ext.as_deref(), opts).unwrap_or_default()
}

/// Kebab-case a bare string, with no extension handling and no length budget.
///
/// ```
/// use keb::kebab_word;
///
/// assert_eq!(kebab_word("parseJSONData"),  "parse-json-data");
/// assert_eq!(kebab_word("Chapter10Part2"), "chapter10-part2");
/// assert_eq!(kebab_word("Привет Мир"),     "privet-mir");
/// ```
pub fn kebab_word(s: &str) -> String {
    kebab_word_with(s, &Options::default())
}

/// [`kebab_word`], with explicit options. The length budget is not applied — only
/// [`kebab_with`] knows where the extension ends.
pub fn kebab_word_with(s: &str, opts: &Options) -> String {
    // Step 2: delete the invisible zoo before anything can be confused by it.
    let s: String = s.chars().filter(|&c| !fold::is_ignorable(c)).collect();

    // Step 3: NFKC folds fullwidth forms, math alphanumerics, ligatures and fractions
    // down to characters the rest of the pipeline recognizes.
    let s: String = s.nfkc().collect();

    // Steps 4 and 6: special folds and script transliteration, both before NFD.
    let s = fold::fold(&s);

    // Step 5: NFD, drop combining marks, recompose.
    //
    // A mark is dropped only when the base it sits on is ASCII. By this point steps 4
    // and 6 have folded every script we romanize down to ASCII, so an ASCII base means
    // "this is a Latin letter wearing an accent" and stripping is the whole point:
    // `é` -> `e`, and the keycap `1\u{fe0f}\u{20e3}` -> `1`. A non-ASCII base means a
    // script we keep, where the mark carries meaning — Devanagari `अध्याय` must not
    // lose its virama and vowel signs (`cases.md` §10, Indic).
    //
    // The trailing NFC is what keeps Hangul intact: NFD explodes syllables into Jamo,
    // which are not marks and so survive the filter, and only NFC puts them back.
    let mut ascii_base = false;
    let s: String = s
        .nfd()
        .filter(|&c| {
            if is_combining_mark(c) {
                return !ascii_base;
            }
            ascii_base = c.is_ascii();
            true
        })
        .collect::<String>()
        .nfc()
        .collect();

    // Decision 12: Arabic-Indic and Devanagari digits to ASCII. NFKC covered fullwidth.
    let s = fold::ascii_digits(&s);

    // Step 9, hoisted above step 10 so the symbols still exist when they are read.
    let s = fold::expand_tokens(&s);

    // Steps 7, 8 and 10.
    emit(&s, opts)
}

/// Append an ordinal to a name's stem, for the filesystem layer's collision suffixing.
///
/// The result is still a fixed point of [`kebab_with`], and still within the length
/// budget — truncation and collision resolution interact, and this is where they meet
/// (`cases.md` §14).
///
/// ```
/// use keb::{with_ordinal, Options};
///
/// let o = Options::default();
/// assert_eq!(with_ordinal("my-file.md",     2, &o), "my-file-2.md");
/// assert_eq!(with_ordinal("my-file-2.md",   3, &o), "my-file-2-3.md");
/// assert_eq!(with_ordinal("archive.tar.gz", 2, &o), "archive-2.tar.gz");
/// ```
pub fn with_ordinal(name: &str, n: u32, opts: &Options) -> String {
    let Some((prefix, stem, ext)) = parts(name, opts) else {
        return name.to_string();
    };

    let tail = format!("{}{n}", opts.separator);
    // Reserve room for the ordinal before fitting, or a name already at the limit
    // would truncate straight back onto the name it collided with.
    let budget = Options { max_length: opts.max_length.saturating_sub(tail.len()), ..opts.clone() };
    let Some(fitted) = assemble(prefix, &stem, ext.as_deref(), &budget) else {
        return String::new();
    };

    let Some((prefix, stem, ext)) = parts(&fitted, opts) else {
        return String::new();
    };
    assemble(prefix, &format!("{stem}{tail}"), ext.as_deref(), opts).unwrap_or_default()
}

/// Split a name into its leading dots, its transformed stem, and its transformed
/// extension. `None` for a name that is nothing but dots.
fn parts<'a>(name: &'a str, opts: &Options) -> Option<(&'a str, String, Option<String>)> {
    // Leading dots mark a dotfile and are preserved verbatim: `.gitignore` is not a
    // file with a `gitignore` extension, and `..config.md` is legitimate (§3, §14).
    let dot_run = name.len() - name.trim_start_matches('.').len();
    let (prefix, rest) = name.split_at(dot_run);

    // `.` and `..` — pass through unchanged so the transform stays idempotent; the
    // filesystem layer refuses them (§3).
    if rest.is_empty() {
        return None;
    }

    // Step 1: split basename/extension first — dot rules are decided on the original
    // string (cases.md §9).
    let (stem, ext) = split_extension(rest);

    // The extension runs through the same whitelist, per dot-separated component, so
    // that `.tar.gz` survives while `.🌀` does not. Without this the transform is not
    // idempotent: junk kept verbatim inside an extension gets stripped on the second
    // pass, once it is no longer in extension position.
    let ext =
        ext.map(|e| e.split('.').map(|c| kebab_word_with(c, opts)).collect::<Vec<_>>().join("."));

    // An extension that transforms to digits is not an extension — `A.᭐` becomes
    // `a.0`, and on the next pass `split_extension` would read that `.0` as a version
    // segment and re-split it. Whatever fails to be an extension here falls back into
    // the stem, where the digit-dot rule in `emit` decides the dot's fate, so nothing
    // is dropped and the second pass agrees with the first.
    let usable = ext.as_deref().is_some_and(|e| {
        !e.is_empty()
            && !e.starts_with('.')
            && !e.ends_with('.')
            && !e.contains("..")
            && !e.chars().all(|c| c.is_ascii_digit())
    });

    match usable {
        true => Some((prefix, kebab_word_with(stem, opts), ext)),
        false => Some((prefix, kebab_word_with(rest, opts), None)),
    }
}

/// Join the pieces back together, truncating the stem to fit the budget.
///
/// `None` when nothing survives: an empty stem, or an extension that alone overruns
/// the limit. Both mean refusal, not a fabricated name.
fn assemble(prefix: &str, stem: &str, ext: Option<&str>, opts: &Options) -> Option<String> {
    let tail = ext.map(|e| format!(".{e}")).unwrap_or_default();
    let fixed = prefix.len() + tail.len();
    let fixed_utf16 = width(prefix) + width(&tail);

    if stem.is_empty() || fixed >= opts.max_length || fixed_utf16 >= opts.max_length {
        return None;
    }

    // Step 11: truncate on grapheme boundaries, tracking both units at once — 255
    // *bytes* on ext4, 255 *UTF-16 code units* on APFS and NTFS (`cases.md` §12).
    let (mut bytes, mut units) = (fixed, fixed_utf16);
    let mut cut = stem.len();
    for (at, g) in stem.grapheme_indices(true) {
        if bytes + g.len() > opts.max_length || units + width(g) > opts.max_length {
            cut = at;
            break;
        }
        bytes += g.len();
        units += width(g);
    }

    let stem = stem[..cut].trim_end_matches([opts.separator, '.']);
    if stem.is_empty() {
        return None;
    }
    Some(format!("{prefix}{stem}{tail}"))
}

/// Length in UTF-16 code units — the unit APFS and NTFS count in.
fn width(s: &str) -> usize {
    s.chars().map(char::len_utf16).sum()
}

/// Steps 7, 8 and 10: split on word boundaries, lowercase, then whitelist.
///
/// The tool does not enumerate characters. It transforms aggressively and then keeps
/// only what is alphanumeric, so an unknown codepoint falls into the separator case
/// instead of needing a table entry (`cases.md` §9).
fn emit(s: &str, opts: &Options) -> String {
    let chars: Vec<char> = s.chars().collect();
    let mut out = String::with_capacity(s.len());
    // Whether the last character emitted was a letter from a script we keep, and so
    // can carry a combining mark.
    let mut carrier = false;

    for (i, &c) in chars.iter().enumerate() {
        // Marks that reach step 10 belong to a script the tool does not transliterate
        // — a Devanagari virama or vowel sign, which must survive. Being zero-width, a
        // mark either rides the character before it or vanishes; it never becomes a
        // separator. Dropping an orphaned one is what keeps the transform idempotent:
        // otherwise a mark left behind by a stripped emoji base would be emitted on
        // the first pass and deburred away on the second.
        if is_combining_mark(c) {
            if carrier {
                out.push(c);
            }
            continue;
        }

        let keep = if opts.ascii { c.is_ascii_alphanumeric() } else { c.is_alphanumeric() };
        carrier = keep && !c.is_ascii();

        if keep {
            if is_boundary(&chars, i) {
                push_sep(&mut out, opts.separator);
            }
            // Step 8: lowercase locale-invariant, never with the system locale
            // (cases.md §10 — Turkish dotless i).
            out.extend(c.to_lowercase());
        } else if c == '.' && between_digits(&chars, i) {
            // `cases.md` §13: `v1.2.3`, `192.168.1.1` and `2024.01.02` must survive the
            // interior-dot rule. A dot with a digit on each side is part of a number;
            // any other dot is punctuation.
            out.push('.');
        } else {
            push_sep(&mut out, opts.separator);
        }
    }

    out.trim_matches(opts.separator).to_string()
}

fn between_digits(chars: &[char], i: usize) -> bool {
    i > 0
        && chars[i - 1].is_ascii_digit()
        && matches!(chars.get(i + 1), Some(c) if c.is_ascii_digit())
}

/// Is there a word boundary immediately before `chars[i]`?
///
/// Decision 1 in `cases.md`: **only case transitions split**. A digit never *starts*
/// a new word, which is what protects `1080p`, `sha256`, `v1`, and every hash and
/// UUID in §13.
///
/// The subtle case is a digit *followed by* an uppercase letter, where two rules in
/// `cases.md` pull against each other:
///
/// - `Chapter10Part2` → `chapter10-part2` (decision 1) — must split
/// - `A1B2C3D4` → `a1b2c3d4` (§13, hashes) — must not
///
/// What separates them is whether the uppercase letter begins a lowercase-continuing
/// word. `Part` does; the `B` in `A1B2` does not. The same test resolves decision 5:
/// `IPv6Address` → `ipv6-address`.
fn is_boundary(chars: &[char], i: usize) -> bool {
    if i == 0 {
        return false;
    }
    let prev = chars[i - 1];
    let cur = chars[i];

    // A separator was already emitted; do not double up.
    if !prev.is_alphanumeric() || !cur.is_uppercase() {
        return false;
    }

    // An "uppercase" character with no lowercase mapping — math alphanumerics like
    // `𝔍`, for instance — is not a case transition, and treating it as one breaks
    // idempotency: it survives step 8 unchanged and splits again on the next pass.
    // NFKC at step 3 folds these away, so this is defence in depth (decision 14).
    if cur.to_lowercase().next() == Some(cur) {
        return false;
    }

    // lower -> Upper:  "myFile" -> my|File
    //
    // Except at position 1, where a single leading lowercase letter belongs to the
    // word that follows: `iPhone14Pro` -> `iphone14-pro`, `eBay` -> `ebay`
    // (`cases.md` decision 13).
    if prev.is_lowercase() {
        return i > 1;
    }

    // Upper -> Upper, or digit -> Upper: split only when the uppercase letter opens
    // a lowercase word.  "XMLHttp" -> XML|Http,  "Chapter10Part" -> chapter10|Part,
    // but "A1B2" stays whole.
    if !matches!(chars.get(i + 1), Some(next) if next.is_lowercase()) {
        return false;
    }

    // Decision 5: "an uppercase run plus following lowercase plus trailing digits is
    // one token", so `IPv6Address` -> `ipv6-address` rather than `i-pv6-address`.
    // What distinguishes `IPv6` from `XMLHttp` is that the lowercase run is followed
    // by a digit rather than by another word.
    //
    // This applies only mid-uppercase-run. After a digit the boundary always stands,
    // which is what keeps `Chapter10Part2` -> `chapter10-part2` (decision 1).
    if prev.is_uppercase() {
        let lowercase_run_end =
            chars[i + 1..].iter().position(|c| !c.is_lowercase()).map(|off| i + 1 + off);

        if matches!(lowercase_run_end, Some(end) if chars[end].is_numeric()) {
            return false;
        }
    }

    true
}

/// Append a separator unless one is already pending or the output is empty.
fn push_sep(out: &mut String, sep: char) {
    if !out.is_empty() && !out.ends_with(sep) {
        out.push(sep);
    }
}

/// Split a leading-dot-free name into `(stem, extension)`.
///
/// A trailing dot is not an extension separator — `Backup.` yields no extension and
/// the dot is dropped as punctuation (`cases.md` §3). Neither is a dot before a pure
/// number: `v1.2.3` and `192.168.1.1` have no extension, they have version segments.
fn split_extension(s: &str) -> (&str, Option<&str>) {
    // Every entry in both tables is ASCII, so the comparison is ASCII-case-insensitive
    // rather than `to_lowercase()` — a lowercased copy does not share `s`'s byte
    // offsets (`Ⱥ` is two bytes, `ⱥ` is three) and slicing it against them panics.

    // Decision 6: compound extensions come from a whitelist; everything else is
    // last-dot-only.
    for compound in COMPOUND_EXTS {
        let len = compound.len() + 1;
        if s.len() > len && s.is_char_boundary(s.len() - len) {
            let (head, tail) = s.split_at(s.len() - len);
            if let Some(ext) = tail.strip_prefix('.')
                && ext.eq_ignore_ascii_case(compound)
            {
                return (head, Some(ext));
            }
        }
    }

    let last = match s.rfind('.') {
        Some(idx) if idx > 0 && idx + 1 < s.len() => idx,
        _ => return (s, None),
    };

    // `.test.*`, `.spec.*`, `.stories.*` — the head binds to whatever follows it.
    if let Some(prev) = s[..last].rfind('.')
        && prev > 0
        && COMPOUND_HEADS.iter().any(|h| s[prev + 1..last].eq_ignore_ascii_case(h))
    {
        return (&s[..prev], Some(&s[prev + 1..]));
    }

    // A dot before a pure number is a version or address segment, not an extension.
    if s[last + 1..].chars().all(|c| c.is_ascii_digit()) {
        return (s, None);
    }

    (&s[..last], Some(&s[last + 1..]))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn separators() {
        // cases.md §1
        assert_eq!(kebab("My File.md"), "my-file.md");
        assert_eq!(kebab("my___file.md"), "my-file.md");
        assert_eq!(kebab("my   file.md"), "my-file.md");
        assert_eq!(kebab("my - file.md"), "my-file.md");
        assert_eq!(kebab("my–file.md"), "my-file.md");
        assert_eq!(kebab("my\tfile.md"), "my-file.md");
        assert_eq!(kebab("my\u{a0}file.md"), "my-file.md");
        assert_eq!(kebab("my\u{200b}file.md"), "my-file.md");
        assert_eq!(kebab("-my-file-.md"), "my-file.md");
    }

    #[test]
    fn case_boundaries() {
        // cases.md §2
        assert_eq!(kebab_word("myFileName"), "my-file-name");
        assert_eq!(kebab_word("MyFileName"), "my-file-name");
        assert_eq!(kebab_word("MY_FILE_NAME"), "my-file-name");
        assert_eq!(kebab_word("HTTPServer"), "http-server");
        assert_eq!(kebab_word("parseJSONData"), "parse-json-data");
        assert_eq!(kebab_word("XMLHttpRequest"), "xml-http-request");
    }

    #[test]
    fn digits_never_split() {
        // cases.md decision 1 — this is what protects hashes and versions
        assert_eq!(kebab_word("Chapter10Part2"), "chapter10-part2");
        assert_eq!(kebab_word("1080p"), "1080p");
        assert_eq!(kebab_word("sha256"), "sha256");
        assert_eq!(kebab_word("file2go"), "file2go");
        assert_eq!(kebab_word("iPhone14Pro"), "iphone14-pro");
        assert_eq!(kebab_word("eBay"), "ebay");
        // decision 5
        assert_eq!(kebab_word("IPv6Address"), "ipv6-address");
    }

    #[test]
    fn hashes_and_versions_survive() {
        // cases.md §13 — the case that pulls against decision 1
        assert_eq!(kebab("A1B2C3D4-E5F6.bin"), "a1b2c3d4-e5f6.bin");
        assert_eq!(kebab("v1.2.3-Release.zip"), "v1.2.3-release.zip");
        assert_eq!(kebab("192.168.1.1"), "192.168.1.1");
        assert_eq!(kebab("2024.01.02"), "2024.01.02");
        assert_eq!(kebab("Backup."), "backup");
        assert_eq!(kebab("..config.md"), "..config.md");
    }

    #[test]
    fn extensions() {
        // cases.md §3
        assert_eq!(kebab("My File.MD"), "my-file.md");
        assert_eq!(kebab("My.Archive.tar.gz"), "my-archive.tar.gz");
        assert_eq!(kebab("types.d.ts"), "types.d.ts");
        assert_eq!(kebab("app.min.js"), "app.min.js");
        assert_eq!(kebab("Component.Test.tsx"), "component.test.tsx");
        assert_eq!(kebab("File.Spec.js"), "file.spec.js");
        assert_eq!(kebab(".gitignore"), ".gitignore");
        assert_eq!(kebab(".env.Local"), ".env.local");
        assert_eq!(kebab("README"), "readme");
        assert_eq!(kebab("My.File.Name.md"), "my-file-name.md");
    }

    #[test]
    fn unicode() {
        // cases.md §4
        assert_eq!(kebab("Żółć Ćma.md"), "zolc-cma.md");
        assert_eq!(kebab("Café Übung.md"), "cafe-ubung.md");
        assert_eq!(kebab("Æther Straße.md"), "aether-strasse.md");
        assert_eq!(kebab("Łódź.md"), "lodz.md");
        assert_eq!(kebab("📁 Report 🚀.md"), "report.md");
        // decision 2: transliterate what is deterministic, keep what is not
        assert_eq!(kebab("Привет.md"), "privet.md");
        assert_eq!(kebab("ЩУКА.md"), "shchuka.md");
        assert_eq!(kebab("Щука.md"), "shchuka.md");
        assert_eq!(kebab("Ελλάδα.md"), "ellada.md");
        assert_eq!(kebab("日本語 Report.md"), "日本語-report.md");
        // NFD input (what macOS hands back) matches NFC input
        assert_eq!(kebab("Cafe\u{301}.md"), kebab("Café.md"));
    }

    #[test]
    fn unicode_trickery() {
        // cases.md §11
        assert_eq!(kebab("Ｆｉｌｅ．ｔｘｔ"), "file-txt");
        assert_eq!(kebab("𝓗𝓮𝓵𝓵𝓸.md"), "hello.md");
        assert_eq!(kebab("File\u{ad}Name.md"), "file-name.md");
        assert_eq!(kebab("\u{feff}Report.md"), "report.md");
        assert_eq!(kebab("photo\u{202e}gpj.jpg"), "photogpj.jpg");
        // keycap: FE0F and the enclosing keycap are combining marks, leaving `1`
        assert_eq!(kebab("1\u{fe0f}\u{20e3}.png"), "1.png");
        // decision 12: non-ASCII digits map to ASCII
        assert_eq!(kebab("Chapter ٣.md"), "chapter-3.md");
        assert_eq!(kebab("अध्याय ३.md"), "अध्याय-3.md");
        // zalgo is a length-blowup vector, not a crash
        let zalgo: String =
            std::iter::once('a').chain(std::iter::repeat_n('\u{301}', 300)).collect();
        assert_eq!(kebab(&zalgo), "a");
    }

    #[test]
    fn punctuation() {
        // cases.md §5
        assert_eq!(kebab("Tom & Jerry.md"), "tom-and-jerry.md");
        assert_eq!(kebab("A&B.md"), "a-and-b.md");
        assert_eq!(kebab("Don't Stop.md"), "dont-stop.md");
        assert_eq!(kebab("Report (Final) [v2].md"), "report-final-v2.md");
        assert_eq!(kebab("50% Off, $20.md"), "50-off-20.md");
        assert_eq!(kebab("@user #tag.md"), "user-tag.md");
        assert_eq!(kebab("file:name*?<>|\"x.md"), "file-name-x.md");
        assert_eq!(kebab("a/b.md"), "a-b.md");
        // decision 8: the punctuation *is* the name
        assert_eq!(kebab("C++ Notes.md"), "cpp-notes.md");
        assert_eq!(kebab("C#.md"), "csharp.md");
        assert_eq!(kebab("Learn F# Fast.md"), "learn-fsharp-fast.md");
        assert_eq!(kebab("Asp.Net Core.md"), "asp-dotnet-core.md");
        // ...but only as a whole token
        assert_eq!(kebab("x.network.md"), "x-network.md");
    }

    #[test]
    fn refuses_when_nothing_survives() {
        // cases.md §7 — an empty result is a refusal, not `untitled`
        assert_eq!(kebab("🚀.md"), "");
        assert_eq!(kebab("___.md"), "");
        assert_eq!(kebab("🚀"), "");
    }

    #[test]
    fn locale_invariant() {
        // cases.md §10 — never `tıtle`, whatever the system locale says
        assert_eq!(kebab_word("TITLE"), "title");
        assert_eq!(kebab_word("İstanbul"), "istanbul");
        // Greek final sigma: both forms romanize to `s`
        assert_eq!(kebab_word("ΟΔΟΣ"), kebab_word("οδός"));
        // Hangul survives NFD
        assert_eq!(kebab("한국어 Report.md"), "한국어-report.md");
    }

    #[test]
    fn options() {
        let snake = Options { separator: '_', ..Options::default() };
        assert_eq!(kebab_with("My File.md", &snake), "my_file.md");
        assert_eq!(kebab_with("XMLHttpRequest", &snake), "xml_http_request");

        let ascii = Options { ascii: true, ..Options::default() };
        assert_eq!(kebab_with("日本語 Report.md", &ascii), "report.md");
        assert_eq!(kebab_with("Żółć.md", &ascii), "zolc.md");
    }

    #[test]
    fn truncation() {
        // cases.md §12 — grapheme-safe, byte-aware, extension-preserving
        let opts = Options { max_length: 12, ..Options::default() };
        assert_eq!(kebab_with("AVeryLongNameIndeed.md", &opts), "a-very-lo.md");
        // the extension alone overruns the budget: refuse rather than eat it
        let tight = Options { max_length: 4, ..Options::default() };
        assert_eq!(kebab_with("Name.json", &tight), "");
        // never split a grapheme
        let opts = Options { max_length: 5, ..Options::default() };
        assert!(kebab_with("日本語語語", &opts).chars().all(|c| c != '\u{fffd}'));
    }

    #[test]
    fn ordinals() {
        let o = Options::default();
        assert_eq!(with_ordinal("my-file.md", 2, &o), "my-file-2.md");
        assert_eq!(with_ordinal("archive.tar.gz", 2, &o), "archive-2.tar.gz");
        assert_eq!(with_ordinal(".gitignore", 2, &o), ".gitignore-2");
        // truncation and collision resolution interact (cases.md §14): the ordinal is
        // reserved out of the budget, so the suffixed name still fits
        let tight = Options { max_length: 10, ..Options::default() };
        let suffixed = with_ordinal("aaaaaaaaaaaaaaa.md", 2, &tight);
        assert_eq!(suffixed, "aaaaa-2.md");
        assert!(suffixed.len() <= 10);
        // and the result is still a fixed point of the transform
        assert_eq!(kebab_with(&suffixed, &tight), suffixed);
    }

    #[test]
    fn degenerate_input_does_not_panic() {
        // cases.md §14
        for s in
            ["", ".", "..", "...", "-", "---", "___", "\n", "\u{202e}", "🚀", "-rf", "$(rm -rf ~)"]
        {
            let _ = kebab(s);
        }
    }

    proptest::proptest! {
        /// The Z invariant. cases.md — "Properties".
        #[test]
        fn idempotent(s: String) {
            let once = kebab(&s);
            proptest::prop_assert_eq!(kebab(&once), once.clone());
        }

        /// Idempotent under every option combination too — a truncated or
        /// transliterated name must still be a fixed point.
        #[test]
        fn idempotent_with_options(s: String, ascii: bool, sep in proptest::sample::select(vec!['-', '_', '.']), max in 3usize..40) {
            let opts = Options { separator: sep, ascii, max_length: max };
            let once = kebab_with(&s, &opts);
            proptest::prop_assert_eq!(kebab_with(&once, &opts), once.clone());
        }

        /// Totality: never panics, for any input.
        #[test]
        fn never_panics(s: String) {
            let _ = kebab(&s);
        }

        /// The length budget is honoured in both units at once.
        #[test]
        fn within_budget(s: String, max in 3usize..40) {
            let opts = Options { max_length: max, ..Options::default() };
            let out = kebab_with(&s, &opts);
            proptest::prop_assert!(out.len() <= max);
            proptest::prop_assert!(width(&out) <= max);
        }
    }
}
