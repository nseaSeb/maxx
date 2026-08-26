//! The version number shown has to come from the lockfile, not from a copied
//! constant that ages in silence.

#[test]
fn the_gpui_version_comes_from_the_lockfile() {
    let version = maxx::about::GPUI_VERSION;
    assert_ne!(version, "unknown", "build.rs could not read gpui's version from Cargo.lock");
    assert!(
        version.split('.').count() >= 2
            && version.split('.').all(|part| part.chars().all(|c| c.is_ascii_digit())),
        "`{version}` does not look like a version"
    );

    // Line by line, and not by substring: git checks the lockfile out with CRLF
    // on Windows, and a pattern holding a `\n` finds nothing there. It is this
    // test that made the first Windows CI go red, not the code.
    let lock = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/Cargo.lock")).unwrap();
    let lines: Vec<&str> = lock.lines().collect();
    let found = lines.windows(2).any(|pair| {
        pair[0].trim() == "name = \"gpui\"" && pair[1].trim() == format!("version = \"{version}\"")
    });
    assert!(found, "the version shown is not the lockfile's: {version}");
}

#[test]
fn the_crate_declares_its_own_version() {
    assert!(!env!("CARGO_PKG_VERSION").is_empty());
}
