//! The repository's demo is the reference: what maxx has to be able to read
//! back, and what it has to be able to rewrite without damaging anything.
//!
//! It lives in `demo/`, versioned, at a path relative to the repository — the
//! old reference was an absolute path into a personal folder, and the test
//! stopped without failing when it was missing: on anybody else's machine, the
//! coverage was nil and silent.

use std::path::{Path, PathBuf};

use maxx::menufile::MenuFile;
use maxx::view::View;

fn demo() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("demo")
}

#[test]
fn every_demo_view_reads_back() {
    let ui = demo().join("src/ui");
    let mut seen = 0;

    for entry in std::fs::read_dir(&ui).expect("the demo must have views") {
        let path = entry.unwrap().path();
        if path.file_name().is_some_and(|name| name == "mod.rs") {
            continue;
        }
        let view = View::load(&path)
            .unwrap_or_else(|error| panic!("{} does not read back: {error}", path.display()));
        assert!(
            !view.root.children.is_empty(),
            "{}: empty tree, the managed region is badly located",
            path.display()
        );
        seen += 1;
    }

    assert!(seen >= 2, "the demo must keep at least two views");
}

#[test]
fn rewriting_a_demo_view_changes_nothing() {
    // The property that matters, and its exact wording: reading back and then
    // rewriting without having changed anything has to give the file back byte
    // for byte — *up to rustfmt*.
    //
    // The nuance is not an admission, it is the description of the system.
    // `codegen` does not write what rustfmt would write, and a Rust editor
    // formats on save; maxx therefore runs rustfmt after itself, and it is the
    // composition of the two that has to be stable. A formatted demo file is
    // also what a real project would be.
    let path = demo().join("src/ui/home.rs");
    let before = std::fs::read_to_string(&path).unwrap();

    let view = View::load(&path).expect("the view must read back");
    let spliced = maxx::parser::splice(&before, &maxx::codegen::render(&view.root, 0))
        .expect("the managed region must be found again");

    let temporary = std::env::temp_dir().join("maxx_demo_round_trip.rs");
    std::fs::write(&temporary, &spliced).unwrap();
    match maxx::run::format_rust(&temporary) {
        Ok(_) => {
            let after = std::fs::read_to_string(&temporary).unwrap();
            assert_eq!(before, after, "rewriting followed by rustfmt is not neutral");
        }
        // The only acceptable failure is rustfmt not being installed. Tested on
        // the `PATH` rather than on the message, which is translated.
        Err(error) => assert!(!maxx::tools::on_path("rustfmt"), "{error}"),
    }
}

#[test]
fn the_demo_uses_the_components_it_advertises() {
    let path = demo().join("src/ui/home.rs");
    let view = View::load(&path).expect("the view must read back");

    let mut bases = Vec::new();
    collect(&view.root, &mut bases);

    for expected in [
        "v_flex",
        "h_flex",
        "Label::new",
        "Input::new",
        "Button::new",
        "Checkbox::new",
        "Switch::new",
        "GroupBox::new",
        "Divider::horizontal",
        "img",
    ] {
        assert!(
            bases.iter().any(|base| base == expected),
            "{expected} has disappeared from the demo: {bases:?}"
        );
    }
}

#[test]
fn the_demo_input_is_bound_to_a_field() {
    let path = demo().join("src/ui/home.rs");
    let view = View::load(&path).expect("the view must read back");

    let fields = view.state_fields();
    assert!(
        fields.iter().any(|field| field.name == "name"),
        "the demo's text input must be bound to a field of the view"
    );
    assert!(view.method_line("on_open").is_some(), "the button's handler must exist");
}

#[test]
fn the_demo_menu_bar_reads_back() {
    let path = demo().join("src/menus.rs");
    let menus = MenuFile::load(&path).expect("the menu bar must read back");

    assert_eq!(menus.menus.len(), 3, "app, Edit, Window");

    let window = menus
        .menus
        .iter()
        .find(|menu| menu.name == "Window")
        .expect("the Window menu must be there");
    assert!(
        window.items.iter().any(|item| item.label() == "Open the inspector"),
        "the entry that opens a window is what the demo exists to show"
    );
    assert!(menus.handler_line("OpenInspector").is_some(), "its action must have a handler");
}

/// Collects the base of every node of the tree.
fn collect(node: &maxx::model::Node, out: &mut Vec<String>) {
    if let Some(path) = node.base.path() {
        out.push(path.to_string());
    }
    for child in &node.children {
        collect(child, out);
    }
}
