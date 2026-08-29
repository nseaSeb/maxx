//! The translations: every key the code cites has to exist, in both languages,
//! and no key should linger with nobody to use it.
//!
//! This test reads the sources rather than calling `t!`. That is deliberate: a
//! missing key does not fail the compilation, it shows up as it is on screen —
//! `message.node_copied` in place of "node copied". Nothing but this test would
//! catch it before the user does.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// The keys of the translation file, with the languages each one carries.
fn catalogue() -> BTreeMap<String, BTreeSet<String>> {
    let source = std::fs::read_to_string(root().join("locales/app.yml")).expect("locales/app.yml");
    let mut keys: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
    let mut current = String::new();
    for line in source.lines() {
        if line.starts_with("_version") || line.trim().is_empty() {
            continue;
        }
        if let Some(key) = line.strip_suffix(':')
            && !line.starts_with(' ')
        {
            current = key.to_string();
            keys.entry(current.clone()).or_default();
            continue;
        }
        if let Some(rest) = line.strip_prefix("  ")
            && let Some((locale, _)) = rest.split_once(": ")
        {
            keys.entry(current.clone()).or_default().insert(locale.to_string());
        }
    }
    keys
}

/// The namespaces a translation key starts with.
///
/// The extractor works on string literals rather than on call sites: a key
/// travels as data — the catalogue's labels, the inspector's field names, the
/// branches of a `tr(if … {…} else {…})` — and only a handful of them are
/// written straight inside `tr("…")`. Listing the namespaces is what tells a
/// key apart from `"maxx.toml"` or `"main.rs"`.
const NAMESPACES: &[&str] = &[
    "about",
    "component",
    "context",
    "designer",
    "error",
    "explorer",
    "menu",
    "palette",
    "message",
    "prefs",
    "prop",
    "run",
    "state_type",
    "status",
    "template",
    "tools",
    "welcome",
];

/// Whether `value` reads as a translation key.
fn is_key(value: &str) -> bool {
    let Some((head, rest)) = value.split_once('.') else {
        return false;
    };
    NAMESPACES.contains(&head)
        && !rest.is_empty()
        && rest
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_' || c == '.')
}

/// Whether the byte at `at` is escaped by an odd run of backslashes.
fn is_escaped(bytes: &[u8], at: usize) -> bool {
    let mut backslashes = 0;
    let mut index = at;
    while index > 0 && bytes[index - 1] == b'\\' {
        backslashes += 1;
        index -= 1;
    }
    backslashes % 2 == 1
}

/// Whether the quote at `at` is the char literal `'"'`.
fn in_char_literal(bytes: &[u8], at: usize) -> bool {
    at > 0 && bytes[at - 1] == b'\'' && bytes.get(at + 1) == Some(&b'\'')
}

/// Every translation key written as a literal in the sources, with its file.
fn keys_used() -> BTreeMap<String, String> {
    let mut used = BTreeMap::new();
    let mut visit = |path: &Path| {
        let Ok(source) = std::fs::read_to_string(path) else {
            return;
        };
        let name = path.strip_prefix(root()).unwrap_or(path).display().to_string();
        // Quotes are counted, not just found: an escaped one inside a literal
        // — `"Tooltip::new(\""` — or a char literal `'"'` used to shift every
        // pair after it, and the keys further down the file were then read as
        // ordinary words and reported unused. The guard has to survive the code
        // it guards.
        let bytes = source.as_bytes();
        let mut index = 0;
        while index < bytes.len() {
            if bytes[index] != b'"' || is_escaped(bytes, index) || in_char_literal(bytes, index) {
                index += 1;
                continue;
            }
            let start = index + 1;
            let mut end = start;
            while end < bytes.len() && (bytes[end] != b'"' || is_escaped(bytes, end)) {
                end += 1;
            }
            if end >= bytes.len() {
                break;
            }
            if let Ok(value) = std::str::from_utf8(&bytes[start..end])
                && is_key(value)
            {
                used.entry(value.to_string()).or_insert_with(|| name.clone());
            }
            index = end + 1;
        }
    };
    for path in rust_sources() {
        visit(&path);
    }
    used
}

/// Every Rust source of maxx and of the demo, the build script included.
///
/// Walked rather than listed: a guard that knows only the directories that
/// existed when it was written stops guarding the day a new one appears — which
/// is exactly how `build.rs` and `demo/` came to carry French nobody saw.
fn rust_sources() -> Vec<PathBuf> {
    let mut out = Vec::new();
    let build = root().join("build.rs");
    if build.exists() {
        out.push(build);
    }
    let mut stack = vec![root().join("src"), root().join("demo/src")];
    while let Some(directory) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&directory) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
            } else if path.extension().is_some_and(|extension| extension == "rs") {
                out.push(path);
            }
        }
    }
    out.sort();
    out
}

#[test]
fn every_key_the_code_cites_is_translated() {
    let catalogue = catalogue();
    let mut missing = Vec::new();
    for (key, file) in keys_used() {
        match catalogue.get(&key) {
            None => missing.push(format!("{key} (cited in {file}): absent")),
            Some(locales) => {
                for locale in ["en", "fr"] {
                    if !locales.contains(locale) {
                        missing.push(format!("{key}: no {locale}"));
                    }
                }
            }
        }
    }
    assert!(missing.is_empty(), "{}", missing.join("\n"));
}

/// No translation is left with nothing to use it.
///
/// An orphan key is not a visible defect, but it is a translation somebody will
/// maintain for nothing — and, more often, the sign of a text replaced elsewhere
/// without the key following.
#[test]
fn no_translation_is_left_unused() {
    let used = keys_used();
    let orphans: Vec<String> =
        catalogue().into_keys().filter(|key| !used.contains_key(key)).collect();
    assert!(orphans.is_empty(), "unused keys:\n{}", orphans.join("\n"));
}

/// The sources no longer carry hard-coded French text.
///
/// That is the half of the job a test can do: a forgotten string stays in one
/// language whatever the user chooses, and nothing at compile time is troubled
/// by it.
///
/// Accents alone are not enough to find them, and that is the lesson this test
/// was rewritten on: `"rustfmt est introuvable"` and `"chemin sans nom de
/// fichier"` carry none, and went out to users in French while this test stayed
/// green. French function words are what actually gives a sentence away.
#[test]
fn no_french_text_is_left_in_the_interface() {
    // A language names itself in its own language: "Français" is the right
    // answer in both locales, not a forgotten string.
    const ALLOWED: &[&str] = &["Français"];

    /// Words English does not use, which no interface string can hold by
    /// accident. Deliberately short: a wrong hit here costs a false failure.
    const FRENCH: &[&str] = &[
        "le", "la", "les", "un", "une", "des", "du", "dans", "pour", "qui", "que", "est", "sont",
        "pas", "sans", "avec", "cette", "ces", "ne", "se", "ce", "au", "aux", "elle", "leur",
        "doit", "être", "aucun", "rien", "fichier", "dossier", "chemin",
    ];

    let french = |value: &str| {
        value.chars().any(|c| "éèêëàâçùûôîïœÉÈÀÇ".contains(c))
            || value
                .split(|c: char| !c.is_alphabetic())
                .any(|word| FRENCH.contains(&word.to_lowercase().as_str()))
    };

    let mut found = Vec::new();
    for path in rust_sources() {
        let Ok(source) = std::fs::read_to_string(&path) else {
            continue;
        };
        let name = path.strip_prefix(root()).unwrap_or(&path).display().to_string();
        let mut rest = source.as_str();
        while let Some(open) = rest.find('"') {
            rest = &rest[open + 1..];
            let Some(close) = rest.find('"') else { break };
            let value = &rest[..close];
            rest = &rest[close + 1..];
            if !ALLOWED.contains(&value) && french(value) {
                found.push(format!("{name}: {value:?}"));
            }
        }
    }
    assert!(found.is_empty(), "hard-coded French text:\n{}", found.join("\n"));
}

/// A helper that takes a key translates it, rather than showing it.
///
/// The other half, and the nastier one: `section_title` was handed keys in place
/// of its text without anything changing type, and every section title showed
/// `designer.properties` for a whole batch. The compilation does not flinch,
/// `no_french_text_is_left_in_the_interface` does not either, and the key really
/// is in `app.yml` — only an eye on the screen catches it, or this.
#[test]
fn a_helper_that_takes_a_key_translates_it() {
    let helpers = [
        ("src/designer.rs", "fn section_title("),
        ("src/designer/menus.rs", "fn menu_button("),
        ("src/workspace/explorer.rs", "fn panel_icon("),
        ("src/preferences.rs", "fn action_button("),
    ];
    for (file, signature) in helpers {
        let source = std::fs::read_to_string(root().join(file)).expect(file);
        let start = source.find(signature).unwrap_or_else(|| panic!("{file}: {signature}"));
        let end =
            source[start..].find("\n}\n").map(|offset| start + offset).unwrap_or(source.len());
        let body = &source[start..end];
        assert!(
            body.contains("crate::tr("),
            "{file}: {signature} takes a key and does not translate it:\n{body}"
        );
    }
}
