//! `maxx new` writes a project without opening a window.
//!
//! The binary is launched for real here rather than `cli::parse` called, since
//! what is being checked is exactly what a script sees: files on the disk, a
//! line on stdout, and an exit code. The project is not built — that costs
//! minutes and `tests/project.rs` already pays them.

use std::path::PathBuf;
use std::process::{Command, Output};

fn scratch(name: &str) -> PathBuf {
    let dir =
        std::env::var("MAXX_SCRATCH").map(PathBuf::from).unwrap_or_else(|_| std::env::temp_dir());
    // The pid is not decoration: this removes the directory before writing in
    // it, so two overlapping `cargo test` runs — a second checkout, a CI job
    // beside a local one — would delete each other's project mid-test.
    let path = dir.join(format!("maxx-cli-{}-{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&path);
    path
}

fn maxx(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_maxx")).args(args).output().expect("maxx must run")
}

fn stdout(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).into_owned()
}

fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

#[test]
fn new_writes_a_project_and_says_where() {
    let root = scratch("maxx_cli_new");
    let output = maxx(&["new", &root.to_string_lossy()]);

    assert!(output.status.success(), "{}", stderr(&output));
    assert!(root.join("Cargo.toml").exists());
    assert!(root.join("maxx.toml").exists());
    assert!(root.join("src/main.rs").exists());

    let said = stdout(&output);
    assert!(said.contains("shape: empty"), "the default shape is named: {said}");
    assert!(said.contains("cargo run"), "the next step is spelled out: {said}");
}

#[test]
fn a_shape_writes_its_shell() {
    let root = scratch("maxx_cli_shape");
    let output = maxx(&["new", &root.to_string_lossy(), "--shape", "sidebar"]);

    assert!(output.status.success(), "{}", stderr(&output));
    assert!(root.join("src/ui/shell.rs").exists(), "the sidebar shape writes a shell");
    assert!(stdout(&output).contains("shape: sidebar"));
}

/// The refusal names every shape, and the list is the one `Template::ALL`
/// holds — so a shape added there is a shape offered here without a line
/// changing.
///
/// The shape asked for has to be one maxx can never have: `dashboard` stood
/// here until the day it became a real one, which is exactly the way a test
/// written against a fixed list goes wrong.
#[test]
fn an_unknown_shape_is_refused_with_the_list() {
    let root = scratch("maxx_cli_unknown_shape");
    let output = maxx(&["new", &root.to_string_lossy(), "--shape", "no-such-shape"]);

    assert_ne!(output.status.code(), Some(0), "an unknown shape is an error");
    let said = stderr(&output);
    for template in maxx::scaffold::Template::ALL {
        assert!(said.contains(template.name()), "the valid shapes are listed: {said}");
    }
    assert!(!root.exists(), "nothing is written when the shape is refused");
}

#[test]
fn a_second_new_on_the_same_path_is_refused() {
    let root = scratch("maxx_cli_twice");
    assert!(maxx(&["new", &root.to_string_lossy()]).status.success());

    let output = maxx(&["new", &root.to_string_lossy()]);
    assert_ne!(output.status.code(), Some(0), "an existing crate is never written over");
    assert!(stderr(&output).contains("Cargo.toml"), "{}", stderr(&output));
}

#[test]
fn help_and_version_answer_without_a_window() {
    let help = maxx(&["--help"]);
    assert!(help.status.success());
    let said = stdout(&help);
    assert!(said.contains("maxx new"), "the usage names every way in: {said}");
    assert!(said.contains("maxx [<path>]"));

    let version = maxx(&["--version"]);
    assert!(version.status.success());
    assert!(stdout(&version).contains(env!("CARGO_PKG_VERSION")));
}
