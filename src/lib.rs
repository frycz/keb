//! Rename files to kebab case, safely and idempotently.
//!
//! This crate is the **transform layer**: a pure `String -> String` function with no
//! filesystem access. The binary (`keb`) wraps it with the stateful filesystem layer.
//! See `cases.md` for the full specification.
//!
//! ```
//! use keb::kebab;
//!
//! assert_eq!(kebab("My File.md"),      "my-file.md");
//! assert_eq!(kebab("XMLHttpRequest"),  "xml-http-request");
//! assert_eq!(kebab("my___file.MD"),    "my-file.md");
//! ```
//!
//! # Invariants
//!
//! 1. **Idempotent** — `kebab(kebab(x)) == kebab(x)` for all `x`
//! 2. **Total** — never panics, for any input
//!
//! Injectivity (distinct inputs never collide) is deliberately *not* a property of
//! this layer: `My File` and `my_file` both produce `my-file`. The filesystem layer
//! restores it by suffixing. See `cases.md` — "Architecture: two layers".
//!
//! # Status
//!
//! Stub. Implements separator normalization ([`cases.md` §1]), case boundaries (§2),
//! and basic extension handling (§3). Unicode (§4), symbol expansion (§5), and
//! truncation are not yet wired in.
//!
//! [`cases.md` §1]: https://github.com/frycz/keb/blob/main/cases.md

#![forbid(unsafe_code)]

/// Compound extensions kept whole. Everything else splits on the last dot.
///
/// From `cases.md`, decision 6.
const COMPOUND_EXTS: &[&str] = &[
    "tar.gz",
    "tar.bz2",
    "tar.xz",
    "d.ts",
    "min.js",
    "module.css",
];

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
    // Leading dots mark a dotfile and are preserved verbatim: `.gitignore` is not a
    // file with a `gitignore` extension, and `..config.md` is legitimate (§3, §14).
    let dot_run = name.len() - name.trim_start_matches('.').len();
    let (prefix, rest) = name.split_at(dot_run);

    // `.` and `..` — pass through unchanged so the transform stays idempotent; the
    // filesystem layer refuses them (§3).
    if rest.is_empty() {
        return name.to_string();
    }

    // Step 1: split basename/extension first — dot rules are decided on the
    // original string (cases.md §9).
    let (stem, ext) = split_extension(rest);

    let stem = kebab_word(stem);

    // The extension runs through the same whitelist, per dot-separated component, so
    // that `.tar.gz` survives while `.🌀` does not. Without this the transform is not
    // idempotent: junk kept verbatim inside an extension gets stripped on the second
    // pass, once it is no longer in extension position.
    let ext = ext
        .map(|e| e.split('.').map(kebab_word).collect::<Vec<_>>().join("."))
        .filter(|e| !e.is_empty() && !e.starts_with('.'));

    match ext {
        Some(ext) => format!("{prefix}{stem}.{ext}"),
        None => format!("{prefix}{stem}"),
    }
}

/// Kebab-case a bare string, with no extension handling.
///
/// ```
/// use keb::kebab_word;
///
/// assert_eq!(kebab_word("parseJSONData"), "parse-json-data");
/// assert_eq!(kebab_word("Chapter10Part2"), "chapter10-part2");
/// ```
pub fn kebab_word(s: &str) -> String {
    // Step 7: split on word boundaries, *before* lowercasing — lowercasing first
    // would destroy camelCase (cases.md §9, trap 1).
    let mut out = String::with_capacity(s.len());
    let chars: Vec<char> = s.chars().collect();

    for (i, &c) in chars.iter().enumerate() {
        if c.is_alphanumeric() {
            if is_boundary(&chars, i) {
                push_sep(&mut out);
            }
            // Step 8: lowercase locale-invariant, never with the system locale
            // (cases.md §10 — Turkish dotless i).
            out.extend(c.to_lowercase());
        } else {
            // Step 10: everything else becomes a separator; runs collapse below.
            push_sep(&mut out);
        }
    }

    out.trim_matches('-').to_string()
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
    // NFKC at step 3 will fold these away and make the guard moot (`cases.md` §11).
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
        let lowercase_run_end = chars[i + 1..]
            .iter()
            .position(|c| !c.is_lowercase())
            .map(|off| i + 1 + off);

        if matches!(lowercase_run_end, Some(end) if chars[end].is_numeric()) {
            return false;
        }
    }

    true
}

/// Append a separator unless one is already pending or the output is empty.
fn push_sep(out: &mut String) {
    if !out.is_empty() && !out.ends_with('-') {
        out.push('-');
    }
}

/// Split a leading-dot-free name into `(stem, extension)`.
///
/// A trailing dot is not an extension separator — `Backup.` yields no extension and
/// the dot is dropped as punctuation (`cases.md` §3).
fn split_extension(s: &str) -> (&str, Option<&str>) {
    let lower = s.to_lowercase();

    // Decision 6: compound extensions come from a whitelist; everything else is
    // last-dot-only.
    for compound in COMPOUND_EXTS {
        let suffix = format!(".{compound}");
        if lower.ends_with(&suffix) && s.len() > suffix.len() {
            let split = s.len() - suffix.len();
            return (&s[..split], Some(&s[split + 1..]));
        }
    }

    match s.rfind('.') {
        Some(idx) if idx > 0 && idx + 1 < s.len() => (&s[..idx], Some(&s[idx + 1..])),
        _ => (s, None),
    }
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
        assert_eq!(kebab_word("IPv6Address"), "ipv6-address"); // decision 5
    }

    #[test]
    fn hashes_and_versions_survive() {
        // cases.md §13 — the case that pulls against decision 1
        assert_eq!(kebab("A1B2C3D4-E5F6.bin"), "a1b2c3d4-e5f6.bin");
        assert_eq!(kebab("v1.2.3-Release.zip"), "v1-2-3-release.zip");
        assert_eq!(kebab("Backup."), "backup");
        assert_eq!(kebab("..config.md"), "..config.md");
    }

    #[test]
    fn extensions() {
        // cases.md §3
        assert_eq!(kebab("My File.MD"), "my-file.md");
        assert_eq!(kebab("My.Archive.tar.gz"), "my-archive.tar.gz");
        assert_eq!(kebab("types.d.ts"), "types.d.ts");
        assert_eq!(kebab(".gitignore"), ".gitignore");
        assert_eq!(kebab(".env.Local"), ".env.local");
        assert_eq!(kebab("README"), "readme");
        assert_eq!(kebab("My.File.Name.md"), "my-file-name.md");
    }

    #[test]
    fn degenerate_input_does_not_panic() {
        // cases.md §14
        for s in ["", ".", "..", "-", "---", "___", "\n", "\u{202e}", "🚀"] {
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

        /// Totality: never panics, for any input.
        #[test]
        fn never_panics(s: String) {
            let _ = kebab(&s);
        }
    }
}
