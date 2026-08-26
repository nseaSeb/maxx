//! Embeds the project's own files into the binary. maxx:assets
//!
//! Walks the directories of `ROOTS` and writes `assets.rs` into `OUT_DIR`,
//! which `src/assets.rs` includes. The contract between the two is this file's
//! output: `pub static ASSETS: &[(&str, &[u8])]`, keyed by the path relative
//! to the project root — the very string the code hands to `img(…)`.
//!
//! Written by maxx, yours from here. Add a directory to `ROOTS` and it travels
//! inside the binary too.

use std::fmt::Write as _;
use std::path::{Path, PathBuf};

/// The directories whose contents travel inside the binary.
const ROOTS: &[&str] = &["assets", "icons"];

fn main() {
    let mut table = String::from(
        "// Written by build.rs. Do not edit.\npub static ASSETS: &[(&str, &[u8])] = &[\n",
    );
    for root in ROOTS {
        println!("cargo::rerun-if-changed={root}");
        collect(Path::new(root), &mut table);
    }
    table.push_str("];\n");

    let out = PathBuf::from(std::env::var("OUT_DIR").expect("cargo sets OUT_DIR"));
    std::fs::write(out.join("assets.rs"), table).expect("the asset table must be written");
}

/// Adds every file under `directory` to the table, recursively.
///
/// A directory that is not there is not a failure: a project keeps its
/// pictures where it likes, and `icons/` is only there once someone wants the
/// gpui-component icons.
fn collect(directory: &Path, table: &mut String) {
    let Ok(entries) = std::fs::read_dir(directory) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let name = path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or(".");
        // Dotfiles belong to the system, not to the project: `.DS_Store` inside
        // the binary is bytes nobody asked for.
        if name.starts_with('.') {
            continue;
        }
        if path.is_dir() {
            collect(&path, table);
            continue;
        }
        // Forward slashes, whatever the system: the key has to match the string
        // written in the source, and that one is written once.
        let key = path.to_string_lossy().replace('\\', "/");
        println!("cargo::rerun-if-changed={key}");
        // `{key:?}` twice, and not once: a quote or a backslash in a file name
        // has to be escaped in the key and in the path just the same.
        let _ = writeln!(
            table,
            "    ({key:?}, include_bytes!(concat!(env!(\"CARGO_MANIFEST_DIR\"), \"/\", {key:?}))),"
        );
    }
}
