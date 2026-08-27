//! Hands the About window the version of `gpui` this build actually resolved,
//! and lays the project shapes out where a compiler can read them.
//!
//! The version is read out of `Cargo.lock` rather than written a second time in
//! the source: the one in `Cargo.toml` is a requirement (`0.2.2` matches
//! `0.2.3`), the one in the lock is what is linked in.

use std::path::Path;

// The shapes maxx writes into a new project, included rather than imported:
// a build script is a program of its own and cannot use the crate it builds.
// Which is the whole reason `src/scaffold/templates.rs` depends on nothing.
include!("src/scaffold/templates.rs");

fn main() {
    let lock = Path::new(env!("CARGO_MANIFEST_DIR")).join("Cargo.lock");
    println!("cargo::rerun-if-changed={}", lock.display());

    write_shapes();

    let version = std::fs::read_to_string(&lock)
        .ok()
        .and_then(|source| locked_version(&source, "gpui"))
        .unwrap_or_else(|| "unknown".into());

    println!("cargo::rustc-env=MAXX_GPUI_VERSION={version}");
}

/// Writes the project shapes into `OUT_DIR`, for `examples/shapes.rs` to
/// compile.
///
/// Nothing else proves that what maxx writes into a project still exists in
/// `gpui-component`: maxx compiles whether or not `SidebarMenuItem::active`
/// is a method, and the developer is the one who finds out. Written from the
/// same functions the projects get, so the two cannot drift apart.
fn write_shapes() {
    println!("cargo::rerun-if-changed=src/scaffold/templates.rs");
    let out = std::path::PathBuf::from(std::env::var("OUT_DIR").expect("cargo sets OUT_DIR"));

    // Both pages at once: one holds a view maxx designs, the other the
    // settings screen, and the shell has to compile against both.
    let shell =
        shell_rs(&[("home", "Home", "Home"), ("settings_screen", "SettingsScreen", "Settings")]);
    std::fs::write(out.join("shell.rs"), as_module_body(&shell))
        .expect("the shape must be written");
    std::fs::write(out.join("settings_screen.rs"), as_module_body(&settings_screen_rs()))
        .expect("the shape must be written");
}

/// The same text, as something `include!` accepts.
///
/// A file maxx writes opens on a `//!` header, which is what a module doc
/// comment is; `include!` refuses one, because what it expands into is not the
/// start of a file. Only the comment markers change — the code compiled here is
/// the code the project gets, character for character.
fn as_module_body(source: &str) -> String {
    let mut out = String::with_capacity(source.len());
    for line in source.lines() {
        match line.strip_prefix("//!") {
            Some(rest) => out.push_str(&format!("//{rest}")),
            None => out.push_str(line),
        }
        out.push('\n');
    }
    out
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
