//! Hands the About window the version of `gpui` this build actually resolved.
//!
//! Read out of `Cargo.lock` rather than written a second time in the source:
//! the one in `Cargo.toml` is a requirement (`0.2.2` matches `0.2.3`), the one
//! in the lock is what is linked in.

use std::path::Path;

fn main() {
    let lock = Path::new(env!("CARGO_MANIFEST_DIR")).join("Cargo.lock");
    println!("cargo::rerun-if-changed={}", lock.display());

    let version = std::fs::read_to_string(&lock)
        .ok()
        .and_then(|source| locked_version(&source, "gpui"))
        .unwrap_or_else(|| "unknown".into());

    println!("cargo::rustc-env=MAXX_GPUI_VERSION={version}");
}

/// The version `Cargo.lock` pins `name` to.
fn locked_version(lock: &str, name: &str) -> Option<String> {
    let needle = format!("name = \"{name}\"");
    let mut lines = lock.lines().skip_while(|line| line.trim() != needle);
    lines.next()?;
    lines
        .next()
        .and_then(|line| line.trim().strip_prefix("version = "))
        .map(|version| version.trim_matches('"').to_string())
}
