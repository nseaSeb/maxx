//! Les traductions : chaque clé citée dans le code doit exister, dans les deux
//! langues, et aucune clé ne doit traîner sans personne pour l'employer.
//!
//! Ce test lit les sources plutôt que d'appeler `t!`. C'est voulu : une clé
//! absente ne fait pas échouer la compilation, elle s'affiche telle quelle à
//! l'écran — « message.node_copied » à la place de « nœud copié ». Rien
//! d'autre que ce test ne l'attraperait avant l'utilisateur.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

fn racine() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// Les clés du fichier de traductions, avec les langues que chacune porte.
fn catalogue() -> BTreeMap<String, BTreeSet<String>> {
    let source =
        std::fs::read_to_string(racine().join("locales/app.yml")).expect("locales/app.yml");
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
    "menu",
    "palette",
    "message",
    "prefs",
    "prop",
    "run",
    "state_type",
    "status",
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

/// Every translation key written as a literal in the sources, with its file.
fn keys_used() -> BTreeMap<String, String> {
    let mut used = BTreeMap::new();
    let mut visit = |path: &Path| {
        let Ok(source) = std::fs::read_to_string(path) else {
            return;
        };
        let name = path.strip_prefix(racine()).unwrap_or(path).display().to_string();
        let mut rest = source.as_str();
        while let Some(open) = rest.find('"') {
            rest = &rest[open + 1..];
            let Some(close) = rest.find('"') else { break };
            let value = &rest[..close];
            rest = &rest[close + 1..];
            if is_key(value) {
                used.entry(value.to_string()).or_insert_with(|| name.clone());
            }
        }
    };
    for directory in ["src", "src/workspace"] {
        let Ok(entries) = std::fs::read_dir(racine().join(directory)) else {
            continue;
        };
        for entry in entries.flatten() {
            if entry.path().extension().is_some_and(|extension| extension == "rs") {
                visit(&entry.path());
            }
        }
    }
    used
}

#[test]
fn every_key_the_code_cites_is_translated() {
    let catalogue = catalogue();
    let mut manquantes = Vec::new();
    for (key, file) in keys_used() {
        match catalogue.get(&key) {
            None => manquantes.push(format!("{key} (citée dans {file}) : absente")),
            Some(locales) => {
                for locale in ["en", "fr"] {
                    if !locales.contains(locale) {
                        manquantes.push(format!("{key} : pas de {locale}"));
                    }
                }
            }
        }
    }
    assert!(manquantes.is_empty(), "{}", manquantes.join("\n"));
}

/// Aucune traduction ne reste sans emploi.
///
/// Une clé orpheline n'est pas un défaut visible, mais c'est une traduction que
/// quelqu'un maintiendra pour rien — et le signe, plus souvent, d'un texte
/// remplacé ailleurs sans que la clé suive.
#[test]
fn no_translation_is_left_unused() {
    let used = keys_used();
    let orphelines: Vec<String> =
        catalogue().into_keys().filter(|key| !used.contains_key(key)).collect();
    assert!(orphelines.is_empty(), "clés sans emploi :\n{}", orphelines.join("\n"));
}
