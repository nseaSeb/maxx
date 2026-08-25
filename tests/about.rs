//! Le numéro de version affiché doit venir du verrou, pas d'une constante
//! recopiée qui vieillit en silence.

#[test]
fn the_gpui_version_comes_from_the_lockfile() {
    let version = maxx::about::GPUI_VERSION;
    assert_ne!(version, "inconnue", "build.rs n'a pas su lire la version de gpui dans Cargo.lock");
    assert!(
        version.split('.').count() >= 2
            && version.split('.').all(|part| part.chars().all(|c| c.is_ascii_digit())),
        "« {version} » ne ressemble pas à une version"
    );

    // Ligne à ligne, et non par sous-chaîne : git extrait le verrou en CRLF sur
    // Windows, et un motif contenant « \n » n'y trouve alors rien. C'est ce
    // test-ci qui a fait rougir la première CI Windows, pas le code.
    let lock = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/Cargo.lock")).unwrap();
    let lignes: Vec<&str> = lock.lines().collect();
    let trouvee = lignes.windows(2).any(|paire| {
        paire[0].trim() == "name = \"gpui\""
            && paire[1].trim() == format!("version = \"{version}\"")
    });
    assert!(trouvee, "la version affichée n'est pas celle du verrou : {version}");
}

#[test]
fn the_crate_declares_its_own_version() {
    assert!(!env!("CARGO_PKG_VERSION").is_empty());
}
