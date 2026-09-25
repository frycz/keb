//! The protect list — `cases.md` §13, "semantic landmines".
//!
//! These are names whose *correct* kebab transform breaks something: `Makefile` stops
//! being found by `make`, `MyClass.java` stops compiling, `[slug].tsx` stops routing.
//! The transform layer is right about all of them and should still be overruled.
//!
//! Decision 4 settles what "overruled" means, and it is not the same in both
//! directions. A file named on the command line was chosen deliberately, so the tool
//! warns and renames it anyway. A file swept up by `-r` was not chosen at all, so it
//! is skipped and reported. `-f` renames it under `-r` too.

/// Exact names that are part of a contract with a tool that looks them up by name.
const EXACT: &[&str] = &[
    "Makefile",
    "GNUmakefile",
    "CMakeLists.txt",
    "Dockerfile",
    "Containerfile",
    "Gemfile",
    "Rakefile",
    "Brewfile",
    "Procfile",
    "Vagrantfile",
    "Jenkinsfile",
    "Justfile",
    "Caddyfile",
    "LICENSE",
    "LICENCE",
    "COPYING",
    "NOTICE",
    "CODEOWNERS",
    "Info.plist",
    "AndroidManifest.xml",
];

/// Directory suffixes that make the directory a bundle. The name is part of a contract
/// with the metadata inside it, so renaming the wrapper breaks the bundle (§12).
const BUNDLES: &[&str] = &[
    ".app",
    ".framework",
    ".bundle",
    ".rtfd",
    ".xcodeproj",
    ".xcworkspace",
    ".playground",
    ".kext",
    ".plugin",
    ".lproj",
    ".docset",
];

/// Prefixes that mark a file as half of a pair, or as syntax rather than a name.
const PREFIXES: &[(&str, &str)] = &[
    ("[", "a dynamic route segment"),
    ("(", "a route group"),
    ("+", "a framework route file"),
    ("__", "a dunder name"),
    ("_index.", "a section index"),
    ("._", "an AppleDouble companion file"),
    ("~$", "an Office lock file"),
    (".#", "an Emacs lock file"),
];

/// Why this name must not be renamed, or `None` if it may be.
///
/// The string completes the sentence "…because it is X", and goes on stderr.
pub fn reason(name: &str) -> Option<&'static str> {
    if EXACT.contains(&name) {
        return Some("a name a build tool looks up literally");
    }
    // `Icon\r` — a real macOS file whose name ends in a carriage return (§12).
    if name.starts_with("Icon") && name.ends_with('\r') {
        return Some("the macOS custom-icon file");
    }
    // Java requires the filename to equal the public class name (§13).
    if name.ends_with(".java") {
        return Some("a Java source file, whose name must match its public class");
    }
    if BUNDLES.iter().any(|b| name.to_ascii_lowercase().ends_with(b)) {
        return Some("a bundle directory");
    }
    if name.ends_with(".icloud") {
        return Some("an iCloud placeholder");
    }
    for &(prefix, why) in PREFIXES {
        if name.starts_with(prefix) {
            return Some(why);
        }
    }
    if is_reserved(name) {
        return Some("a Windows reserved device name");
    }
    None
}

/// `CON`, `PRN`, `NUL`, `COM1`, `LPT1` and friends are device names on Windows, with
/// or without an extension, in any case (§7).
fn is_reserved(name: &str) -> bool {
    const DEVICES: &[&str] = &["CON", "PRN", "AUX", "NUL"];

    let stem = name.split('.').next().unwrap_or(name);
    if DEVICES.iter().any(|d| stem.eq_ignore_ascii_case(d)) {
        return true;
    }
    let Some(digit) = stem.chars().next_back().filter(char::is_ascii_digit) else {
        return false;
    };
    let head = &stem[..stem.len() - digit.len_utf8()];
    digit != '0' && (head.eq_ignore_ascii_case("COM") || head.eq_ignore_ascii_case("LPT"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_significant_names() {
        assert!(reason("Makefile").is_some());
        assert!(reason("CMakeLists.txt").is_some());
        assert!(reason("MyClass.java").is_some());
        assert!(reason("Info.plist").is_some());
        // ...but `README` is not in the contract with anything
        assert!(reason("README").is_none());
        assert!(reason("My File.md").is_none());
    }

    #[test]
    fn framework_syntax() {
        assert!(reason("[slug].tsx").is_some());
        assert!(reason("[...slug].tsx").is_some());
        assert!(reason("(marketing)").is_some());
        assert!(reason("+page.svelte").is_some());
        assert!(reason("+layout.server.ts").is_some());
        assert!(reason("__init__.py").is_some());
        assert!(reason("_index.md").is_some());
    }

    #[test]
    fn bundles_and_companions() {
        assert!(reason("Foo.app").is_some());
        assert!(reason("Bar.framework").is_some());
        assert!(reason("._My File.md").is_some());
        assert!(reason("~$doc.docx").is_some());
        assert!(reason("Icon\r").is_some());
    }

    #[test]
    fn windows_devices() {
        for name in ["CON", "con.txt", "NUL", "COM1", "lpt9.log", "AUX"] {
            assert!(reason(name).is_some(), "{name}");
        }
        for name in ["COM0", "COM10", "CONFIG", "console.log", "auxiliary.md"] {
            assert!(reason(name).is_none(), "{name}");
        }
    }
}
