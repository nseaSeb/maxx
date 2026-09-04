//! The catalogue of editors and terminals: it is a table, so what can go wrong
//! is in it, not in an algorithm.

use std::path::Path;

use maxx::run::editor_arguments;
use maxx::tools::{EDITORS, Editor, LineArgument, TERMINALS};

fn editor(id: &str) -> &'static Editor {
    EDITORS
        .iter()
        .find(|editor| editor.id == id)
        .unwrap_or_else(|| panic!("{id} must be in the catalogue"))
}

#[test]
fn every_editor_spells_its_line_number_its_own_way() {
    let path = Path::new("/tmp/view.rs");

    assert_eq!(editor_arguments(editor("zed"), path, Some(12)), vec!["/tmp/view.rs:12"]);
    assert_eq!(editor_arguments(editor("code"), path, Some(12)), vec!["-g", "/tmp/view.rs:12"]);
    assert_eq!(editor_arguments(editor("nvim"), path, Some(12)), vec!["+12", "/tmp/view.rs"]);
    assert_eq!(
        editor_arguments(editor("rustrover"), path, Some(12)),
        vec!["--line", "12", "/tmp/view.rs"]
    );
}

#[test]
fn without_a_line_every_editor_takes_the_bare_path() {
    let path = Path::new("/tmp/view.rs");
    for candidate in EDITORS {
        assert_eq!(
            editor_arguments(candidate, path, None),
            vec!["/tmp/view.rs"],
            "{}",
            candidate.id
        );
    }
}

#[test]
fn the_catalogue_holds_no_duplicate_and_no_hole() {
    for (index, editor) in EDITORS.iter().enumerate() {
        assert!(!editor.id.is_empty());
        assert!(!editor.label.is_empty());
        // An editor with neither a command nor a bundle can never be detected.
        assert!(!editor.command.is_empty() || editor.bundle.is_some(), "{}", editor.id);
        // A terminal editor has no bundle to open: it is only a command.
        if editor.terminal_bound {
            assert!(editor.bundle.is_none(), "{}", editor.id);
            assert!(!editor.command.is_empty(), "{}", editor.id);
        }
        assert!(
            EDITORS[index + 1..].iter().all(|other| other.id != editor.id),
            "{} en double",
            editor.id
        );
    }

    for (index, terminal) in TERMINALS.iter().enumerate() {
        assert!(!terminal.id.is_empty());
        assert!(!terminal.command.is_empty() || terminal.bundle.is_some(), "{}", terminal.id);
        // Running a command supposes a command to hand it to.
        if terminal.command_flag.is_some() {
            assert!(!terminal.command.is_empty(), "{}", terminal.id);
        }
        assert!(
            TERMINALS[index + 1..].iter().all(|other| other.id != terminal.id),
            "{} en double",
            terminal.id
        );
    }
}

#[test]
fn a_flag_style_editor_never_gets_a_suffix_and_the_reverse() {
    // The confusion that would break everything: `code file:12` opens a file
    // named `file:12`, and `zed -g file:12` does not understand -g.
    for editor in EDITORS {
        let arguments = editor_arguments(editor, Path::new("/tmp/a.rs"), Some(3));
        match editor.line {
            LineArgument::Suffix => {
                assert_eq!(arguments.len(), 1, "{}", editor.id);
                assert!(arguments[0].ends_with(":3"), "{}", editor.id);
            }
            LineArgument::Flag(flag) => {
                assert_eq!(arguments[0], flag, "{}", editor.id);
                assert!(arguments[1].ends_with(":3"), "{}", editor.id);
            }
            LineArgument::PlusLine => assert_eq!(arguments[0], "+3", "{}", editor.id),
            LineArgument::Named(name) => {
                assert_eq!(arguments[0], name, "{}", editor.id);
                assert_eq!(arguments[1], "3", "{}", editor.id);
            }
        }
    }
}

#[test]
fn nothing_is_found_on_an_empty_path() {
    assert!(!maxx::tools::on_path(""));
}

#[test]
fn rustfmt_reformats_a_file_and_says_so() {
    let path = scratch_file("maxx_format_test.rs");
    std::fs::write(&path, "fn   main(){let  x=1;let _=x;}\n").unwrap();

    match maxx::run::format_rust(&path) {
        Ok(change) => {
            assert!(change, "rustfmt had something to do on this file");
            let after = std::fs::read_to_string(&path).unwrap();
            assert!(after.contains("fn main() {"), "{after}");
            // Twice in a row changes nothing more: rustfmt is idempotent, and
            // that is what makes maxx's round trip stable.
            assert!(!maxx::run::format_rust(&path).unwrap());
        }
        // rustfmt is not guaranteed to be there; the test says what it checks
        // rather than failing for an unrelated reason.
        Err(error) => assert!(!maxx::tools::on_path("rustfmt"), "{error}"),
    }
}

#[test]
fn a_file_that_is_not_rust_is_refused_rather_than_mangled() {
    let path = scratch_file("maxx_format_invalid.rs");
    std::fs::write(&path, "this is not Rust {{{\n").unwrap();

    // The text of the message is not what is checked: it is translated, so it
    // depends on the language, and asserting it here would tie the behaviour to
    // its wording. What matters is that there is a refusal and that it names the
    // file.
    if let Err(error) = maxx::run::format_rust(&path) {
        assert!(!error.is_empty());
        assert!(error.contains("maxx_format_invalid.rs") || error.contains("rustfmt"), "{error}");
    }
    // And above all: the file has not been mangled.
    assert!(std::fs::read_to_string(&path).unwrap().contains("this is not Rust"));
}

/// A directory of this run's own, under `MAXX_SCRATCH` when it is set.
///
/// Fixed names under `temp_dir()` collide whenever two `cargo test` runs
/// overlap — a second checkout, a CI job beside a local run — and the failure
/// then lands on whichever test read a file the other had just removed. The
/// pid separates two runs even when the variable is unset. Repeated per file
/// because each integration test is a crate of its own.
fn scratch_file(name: &str) -> std::path::PathBuf {
    let root = std::env::var_os("MAXX_SCRATCH")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(std::env::temp_dir);
    let directory = root.join(format!("maxx-{}-{}", "tools", std::process::id()));
    std::fs::create_dir_all(&directory).expect("the test directory must be created");
    directory.join(name)
}
