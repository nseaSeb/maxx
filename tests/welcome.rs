//! The welcome screen, minus its window: what it reads before it draws.
//!
//! Two questions, and both are answered off the disk rather than off gpui —
//! which is what makes them testable at all. Is the repository's demo reachable
//! from this binary, and what tree does a recent project's entry view hold? The
//! drawing itself is the canvas's, and the canvas is already covered elsewhere.

use std::path::{Path, PathBuf};

use maxx::project::{demo_beside, entry_tree};

fn scratch(name: &str) -> PathBuf {
    let dir =
        std::env::var("MAXX_SCRATCH").map(PathBuf::from).unwrap_or_else(|_| std::env::temp_dir());
    let path = dir.join(name);
    let _ = std::fs::remove_dir_all(&path);
    std::fs::create_dir_all(&path).unwrap();
    path
}

fn repository() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).to_path_buf()
}

/// A project with a `maxx.toml` naming `home`, and whatever `home.rs` holds.
fn project(root: &Path, home: &str) {
    std::fs::create_dir_all(root.join("src/ui")).unwrap();
    std::fs::write(root.join("maxx.toml"), "[project]\nentry = \"home\"\n").unwrap();
    std::fs::write(root.join("src/ui/home.rs"), home).unwrap();
}

#[test]
fn the_demo_is_found_from_the_binary_that_runs() {
    // The layout of a checkout: the binary sits in `target/debug`, the demo at
    // the root, three levels up.
    let root = scratch("welcome-demo");
    std::fs::create_dir_all(root.join("target/debug")).unwrap();
    std::fs::create_dir_all(root.join("demo")).unwrap();
    std::fs::write(root.join("demo/maxx.toml"), "").unwrap();
    // The manifest is half of what makes this the demo: three levels up from
    // an installed `~/.cargo/bin/maxx` is the home directory, and a stray
    // `~/demo` there must not answer.
    std::fs::write(root.join("Cargo.toml"), "[package]\nname = \"maxx\"\n").unwrap();

    let executable = root.join("target/debug/maxx");
    assert_eq!(demo_beside(&executable), Some(root.join("demo")));
}

#[test]
fn a_demo_beside_the_binary_is_found_too() {
    // What an installed maxx would look like if someone shipped the demo with
    // it: the folder next to the executable, no climbing needed.
    let root = scratch("welcome-demo-beside");
    std::fs::create_dir_all(root.join("demo")).unwrap();
    std::fs::write(root.join("demo/maxx.toml"), "").unwrap();
    std::fs::write(root.join("Cargo.toml"), "[package]\nname = \"maxx\"\n").unwrap();

    assert_eq!(demo_beside(&root.join("maxx")), Some(root.join("demo")));
}

#[test]
fn a_folder_called_demo_is_not_the_demo() {
    // `maxx.toml` is what makes it a project. Opening a folder that only shares
    // the name would give an empty window with nothing to explain it.
    let root = scratch("welcome-demo-impostor");
    std::fs::create_dir_all(root.join("demo/src")).unwrap();

    assert_eq!(demo_beside(&root.join("maxx")), None);
}

#[test]
fn the_search_stops_at_the_root_of_the_checkout() {
    // The bound, and what it is for: a checkout living beside somebody else's
    // `demo/` must not open theirs. Three levels up from the binary is the root
    // of the checkout and no further.
    let root = scratch("welcome-demo-too-far");
    std::fs::create_dir_all(root.join("checkout/target/debug")).unwrap();
    std::fs::create_dir_all(root.join("demo")).unwrap();
    std::fs::write(root.join("demo/maxx.toml"), "").unwrap();

    assert_eq!(demo_beside(&root.join("checkout/target/debug/maxx")), None);
}

#[test]
fn an_installed_binary_finds_no_demo() {
    // The case the button exists for: `cargo install` copies the binary and
    // nothing else, so there is nothing to open and no button to draw.
    let root = scratch("welcome-demo-installed");
    assert_eq!(demo_beside(&root.join("bin/maxx")), None);
}

#[test]
fn the_entry_view_of_a_real_project_reads_back() {
    // The repository's own demo, which is the reference project: its
    // `maxx.toml` names `home`, and `home.rs` holds a tree.
    let started = std::time::Instant::now();
    let tree = entry_tree(&repository().join("demo")).expect("the demo's entry view must read");
    let elapsed = started.elapsed();

    assert!(!tree.children.is_empty(), "the demo's entry view is not an empty tree");
    // Printed rather than asserted at the millisecond: this is what says whether
    // reading one card is cheap, and the ceiling is loose on purpose — the
    // number is the interesting part, and a machine under load must not fail a
    // test about correctness.
    eprintln!("one entry view read and parsed in {elapsed:?}");
    assert!(elapsed.as_secs() < 1, "reading one entry view took {elapsed:?}");
}

#[test]
fn a_view_that_does_not_parse_is_no_tree() {
    // No managed region, so nothing to read: the card shows an empty frame
    // rather than an error nobody asked for.
    let root = scratch("welcome-unreadable");
    project(&root, "fn main() {}\n");

    assert!(entry_tree(&root).is_none());
}

#[test]
fn a_project_that_names_no_view_is_no_tree() {
    let root = scratch("welcome-no-entry");
    std::fs::create_dir_all(root.join("src/ui")).unwrap();
    std::fs::write(root.join("maxx.toml"), "").unwrap();

    assert!(entry_tree(&root).is_none());
}

#[test]
fn a_project_that_is_gone_is_no_tree() {
    // The ordinary case, and the one that must not panic: the recent list
    // outlives the folders it names.
    let root = scratch("welcome-vanished");
    let gone = root.join("moved-away");

    assert!(entry_tree(&gone).is_none());
}

#[test]
fn a_demo_in_the_home_directory_is_not_maxx_s_demo() {
    // The failure the level count alone allowed. From `~/.cargo/bin/maxx` the
    // three steps land on the home directory, so somebody's own `~/demo` —
    // a real maxx project, `maxx.toml` and all — was offered on the welcome
    // screen as *the* demo. What settles it is the manifest beside it: this
    // one belongs to whoever wrote it, not to maxx.
    let home = scratch("welcome-demo-home");
    std::fs::create_dir_all(home.join(".cargo/bin")).unwrap();
    std::fs::create_dir_all(home.join("demo")).unwrap();
    std::fs::write(home.join("demo/maxx.toml"), "").unwrap();
    std::fs::write(home.join("Cargo.toml"), "[package]\nname = \"leur_projet\"\n").unwrap();

    assert_eq!(demo_beside(&home.join(".cargo/bin/maxx")), None);
}
