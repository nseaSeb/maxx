//! The `components` module: the bricks a project starts with.
//!
//! Not the sub-trees of the palette, and the difference is what this module is
//! for. A sub-tree is pasted into a view, so ten cards are ten copies and
//! changing the look of a card is ten edits in ten files. A component is a
//! type, so ten cards are ten calls and changing the look is one file — the
//! file the developer owns, in their project, in ordinary Rust.
//!
//! It brings the palette module with it, the way `settings` brings `system`:
//! the bricks are painted with the project's own roles, and a brick that
//! carried its own colours would ignore the palette the developer just chose.

use std::io;
use std::path::Path;

use super::modules::{header_end, joined, module_version};
use super::templates::COMPONENTS;
use super::theme::add_theme_module;

/// Copies the component library into the project and declares it.
///
/// Each component is one file of `src/components/`, with `src/components/mod.rs`
/// declaring them. A file already there is left alone: it is the developer's
/// from the moment they open it, and a library that overwrites what it finds is
/// a library nobody dares to edit.
pub fn add_components_module(root: &Path) -> io::Result<()> {
    // The bricks paint with the project's roles, so the roles have to exist.
    add_theme_module(root)?;

    let main_path = root.join("src/main.rs");
    let source = std::fs::read_to_string(&main_path)?;
    let mut lines: Vec<String> = source.lines().map(str::to_string).collect();
    if !lines.iter().any(|line| line.trim() == "mod components;") {
        lines.insert(header_end(&lines), "mod components;".into());
    }

    let directory = root.join("src/components");
    std::fs::create_dir_all(&directory)?;
    for (name, body) in COMPONENTS {
        let path = directory.join(format!("{name}.rs"));
        if !path.exists() {
            std::fs::write(&path, body)?;
        }
    }

    let declarations = components_mod_rs();
    let mod_path = directory.join("mod.rs");
    if !mod_path.exists() {
        std::fs::write(&mod_path, &declarations)?;
    }

    // One fingerprint for the whole library, over the text of every file in the
    // order the table gives them. A library is updated or it is not; offering
    // to update one brick out of three would leave a project holding two
    // versions of the same idea.
    crate::projectfile::record(
        root,
        "components",
        module_version("components").unwrap_or(1),
        &module_body(),
    )?;

    std::fs::write(&main_path, joined(&lines, &source))
}

/// The text the fingerprint is taken over: every component, then the module
/// that declares them.
pub(super) fn module_body() -> String {
    let mut out = String::new();
    for (_, body) in COMPONENTS {
        out.push_str(body);
    }
    out.push_str(&components_mod_rs());
    out
}

/// `src/components/mod.rs`: one declaration and one re-export per component.
///
/// Re-exported so a view writes `use crate::components::Card;` rather than
/// `use crate::components::card::Card;` — the shorter one is what a developer
/// would have written, and what maxx writes has to be that.
fn components_mod_rs() -> String {
    let mut out = String::from(
        "//! The project's own components.\n\
         //!\n\
         //! Written by maxx, yours from here. Each one is an ordinary GPUI\n\
         //! component: a builder, and a `RenderOnce` that draws it with the\n\
         //! roles of `crate::theme`.\n\
         //!\n\
         //! A brick nobody has used yet is not dead code, it is a brick. Without\n\
         //! this, adding the library and building would answer with a warning\n\
         //! for every component of it — on files that had just arrived.\n\
         #![allow(dead_code)]\n\n",
    );
    for (name, _) in COMPONENTS {
        out.push_str(&format!("mod {name};\n"));
    }
    out.push('\n');
    for (name, _) in COMPONENTS {
        out.push_str(&format!("pub use {name}::{};\n", type_name(name)));
    }
    out
}

/// `empty_state` as `EmptyState`.
fn type_name(module: &str) -> String {
    module
        .split('_')
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
                None => String::new(),
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_module_name_becomes_its_type_name() {
        assert_eq!(type_name("card"), "Card");
        assert_eq!(type_name("empty_state"), "EmptyState");
        assert_eq!(type_name("toolbar"), "Toolbar");
    }

    /// Every component declares the type its file is expected to hold.
    ///
    /// `mod.rs` is generated from the file names, and the source is written by
    /// hand: a `card.rs` holding `struct Panel` would produce a `mod.rs` that
    /// does not compile, in the developer's project, on a line maxx wrote.
    #[test]
    fn every_component_holds_the_type_its_name_promises() {
        for (name, body) in COMPONENTS {
            let expected = format!("pub struct {}", type_name(name));
            assert!(body.contains(&expected), "{name}.rs must declare {expected}");
        }
    }

    /// Every role a component names is one the palette actually declares.
    ///
    /// The two are written apart — the bricks here, the roles in
    /// `scaffold::theme` — so nothing but this says that `theme::PANEL` exists.
    /// A brick reaching for a role that was never declared is a project that
    /// stops compiling on a file maxx wrote.
    #[test]
    fn every_role_a_component_names_is_declared_by_the_palette() {
        let palette = super::super::theme::theme_rs();
        for (name, body) in COMPONENTS {
            let mut rest = *body;
            while let Some(at) = rest.find("theme::") {
                rest = &rest[at + "theme::".len()..];
                let role: String =
                    rest.chars().take_while(|c| c.is_ascii_uppercase() || *c == '_').collect();
                assert!(!role.is_empty(), "{name}.rs: theme:: followed by nothing");
                assert!(
                    palette.contains(&format!("pub const {role}: Role")),
                    "{name}.rs names {role}, which the palette does not declare"
                );
            }
        }
    }

    /// And every one of them paints with the project's palette.
    ///
    /// A brick carrying a colour of its own would ignore the palette the
    /// developer chose, which is the one thing this library must not do.
    #[test]
    fn every_component_paints_with_the_project_s_roles() {
        for (name, body) in COMPONENTS {
            assert!(body.contains("use crate::theme;"), "{name}.rs must read the palette");
            assert!(!body.contains("rgb(0x"), "{name}.rs must not carry a colour of its own");
        }
    }
}
