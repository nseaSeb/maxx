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

    // The handler bodies, each wrapped in the two parameters a handler stub
    // carries. Written from the same table `view::fill_handler` inserts from,
    // so a call gpui-component drops is a build that fails here rather than a
    // project that stops compiling on a line maxx wrote.
    let mut boxes = String::new();
    let mut seen: Vec<&str> = Vec::new();
    for (name, imports, body) in BOXES {
        // Two boxes needing the same `use` is the common case, and a repeated
        // import is a compile error here where it is not in a project — each
        // one lands in a file of its own there.
        for import in *imports {
            if !seen.contains(import) {
                seen.push(import);
                boxes.push_str(import);
                boxes.push('\n');
            }
        }
        boxes.push_str(&format!(
            "#[allow(dead_code)]\nfn a_{name}(window: &mut Window, cx: &mut App) {{\n        {body}\n}}\n"
        ));
    }
    std::fs::write(out.join("boxes.rs"), boxes).expect("the boxes must be written");

    // The sub-tree templates, each as the expression it is. Same reason as the
    // boxes: a call gpui-component drops has to fail here rather than in a
    // project, on a line maxx wrote.
    let mut subtrees = String::new();
    let mut seen: Vec<&str> = Vec::new();
    for (id, imports, source) in SUBTREES {
        for import in *imports {
            if !seen.contains(import) {
                seen.push(import);
                subtrees.push_str(import);
                subtrees.push('\n');
            }
        }
        subtrees.push_str(&format!(
            "#[allow(dead_code)]\nfn a_{id}() -> impl IntoElement {{\n    {source}\n}}\n"
        ));
    }
    std::fs::write(out.join("subtrees.rs"), subtrees).expect("the templates must be written");

    // The component library, each brick in a module of its own — the shape it
    // has in a project, so that a `use super::…` that only works when the files
    // are flattened fails here rather than there.
    let mut components = String::new();
    for (name, body) in COMPONENTS {
        components.push_str(&format!("pub mod {name} {{\n"));
        components.push_str(&as_module_body(body));
        components.push_str("}\n");
    }
    std::fs::write(out.join("components.rs"), components).expect("the components must be written");
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
