//! The components the project itself holds, read back out of its source.
//!
//! The catalogue maxx ships is a table compiled into maxx. It cannot know about
//! the `Card` a developer wrote on their third day — and that is the day maxx
//! starts being a spectator, because their own components are exactly what a
//! project is made of once it has any.
//!
//! So they are read. Not declared in a file of maxx's own, which would be a
//! catalogue to keep in step by hand, and the format of its own that the
//! project refuses to have; read out of the Rust, the way a view is. What is
//! looked for is the shape `src/components/` already has, because maxx wrote
//! it:
//!
//! ```ignore
//! pub struct Card { … }
//! impl Card {
//!     pub fn new(title: impl Into<SharedString>) -> Self { … }
//!     pub fn subtitle(mut self, subtitle: impl Into<SharedString>) -> Self { … }
//! }
//! ```
//!
//! Bounded on purpose, and bounded by what maxx itself writes: the library
//! shipped by `scaffold::components` is the first thing this has to read, so
//! there is a working example of every shape it accepts before anyone else's
//! code is involved. A component written differently is simply not offered —
//! the same bargain an unrecognised expression gets on the canvas.

use std::path::Path;

/// One component of the project, as the palette needs it.
#[derive(Clone, Debug, PartialEq)]
pub struct Brick {
    /// The type, e.g. `Card`.
    pub type_name: String,
    /// The module it lives in, e.g. `card`.
    pub module: String,
    /// The first line of the file's own `//!`, for the palette to show.
    pub doc: String,
    /// How many arguments `new` takes.
    pub arity: usize,
}

impl Brick {
    /// The expression dropping this brick writes.
    ///
    /// A placeholder per argument rather than an empty call: what lands on the
    /// canvas has to be something you can see, and `Card::new("")` draws a card
    /// with no title, which reads as a broken card.
    pub fn expression(&self) -> String {
        let args = (0..self.arity).map(|_| "\"Text\"").collect::<Vec<_>>().join(", ");
        format!("{}::new({args})", self.type_name)
    }

    /// The `use` line a view needs to name it.
    ///
    /// Through the re-export and not the module: `use crate::components::Card;`
    /// is what `src/components/mod.rs` offers and what a developer would have
    /// written.
    pub fn import(&self) -> String {
        format!("use crate::components::{};", self.type_name)
    }
}

/// Reads the project's components.
///
/// Empty when the project has no library, which is the ordinary case — it is
/// added on demand — rather than an error.
pub fn read(root: &Path) -> Vec<Brick> {
    let directory = root.join("src/components");
    let Ok(entries) = std::fs::read_dir(&directory) else {
        return Vec::new();
    };

    let mut out = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().is_none_or(|extension| extension != "rs") {
            continue;
        }
        let Some(module) = path.file_stem().map(|stem| stem.to_string_lossy().into_owned()) else {
            continue;
        };
        // `mod.rs` declares the others and is not one of them.
        if module == "mod" {
            continue;
        }
        let Ok(source) = std::fs::read_to_string(&path) else {
            continue;
        };
        if let Some(brick) = read_one(&module, &source) {
            out.push(brick);
        }
    }
    // Read from a directory, so in whatever order the filesystem answers:
    // sorted, or the palette would reshuffle itself between two launches.
    out.sort_by(|a, b| a.type_name.cmp(&b.type_name));
    out
}

/// The component one file holds, when it holds one maxx can offer.
fn read_one(module: &str, source: &str) -> Option<Brick> {
    let file = syn::parse_file(source).ok()?;

    // The public struct of the file. Not "the struct named after the file": a
    // developer is free to have renamed one without renaming the other, and the
    // type is what a view will write.
    let type_name = file.items.iter().find_map(|item| match item {
        syn::Item::Struct(item) if matches!(item.vis, syn::Visibility::Public(_)) => {
            Some(item.ident.to_string())
        }
        _ => None,
    })?;

    // And its `new`. Without one there is nothing to write on the canvas, so
    // there is nothing to offer either.
    let arity = file.items.iter().find_map(|item| {
        let syn::Item::Impl(block) = item else {
            return None;
        };
        if block.trait_.is_some() || !impl_of(block, &type_name) {
            return None;
        }
        block.items.iter().find_map(|item| match item {
            syn::ImplItem::Fn(function)
                if function.sig.ident == "new"
                    && matches!(function.vis, syn::Visibility::Public(_)) =>
            {
                Some(function.sig.inputs.len())
            }
            _ => None,
        })
    })?;

    Some(Brick { type_name, module: module.to_string(), doc: first_doc_line(source), arity })
}

/// Whether this `impl` block is the inherent one of `type_name`.
fn impl_of(block: &syn::ItemImpl, type_name: &str) -> bool {
    let syn::Type::Path(path) = block.self_ty.as_ref() else {
        return false;
    };
    path.path.segments.last().is_some_and(|segment| segment.ident == type_name)
}

/// The first line of the file's own `//!`, without it.
fn first_doc_line(source: &str) -> String {
    source
        .lines()
        .find_map(|line| line.trim().strip_prefix("//!"))
        .map(|text| text.trim().to_string())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every brick maxx ships is one maxx reads back.
    ///
    /// The point of shipping the library first: it is the worked example of the
    /// shape this reader accepts, so the reader has a customer before anyone
    /// else's code is involved — and a brick whose shape drifts out of what can
    /// be read fails here rather than going quietly missing from the palette.
    #[test]
    fn every_component_maxx_writes_is_one_maxx_reads() {
        for (module, body) in crate::scaffold::templates::COMPONENTS {
            let brick = read_one(module, body)
                .unwrap_or_else(|| panic!("{module}.rs must be readable as a component"));
            assert_eq!(brick.module, *module);
            assert!(!brick.doc.is_empty(), "{module}.rs must say what it is");
        }
    }

    /// `Card::new(title)` takes one argument, `Toolbar::new()` none.
    #[test]
    fn the_arity_of_new_is_what_the_drop_writes() {
        let read = |name: &str| {
            let (module, body) = crate::scaffold::templates::COMPONENTS
                .iter()
                .find(|(module, _)| *module == name)
                .expect(name);
            read_one(module, body).expect(name)
        };
        assert_eq!(read("card").expression(), "Card::new(\"Text\")");
        assert_eq!(read("toolbar").expression(), "Toolbar::new()");
        assert_eq!(read("card").import(), "use crate::components::Card;");
    }

    /// What the expression writes is something the parser reads back.
    ///
    /// A drop that produced an opaque node would put a component on the canvas
    /// that maxx cannot move, name or delete — worse than not offering it.
    #[test]
    fn what_a_drop_writes_parses_as_a_node() {
        for (module, body) in crate::scaffold::templates::COMPONENTS {
            let brick = read_one(module, body).expect(module);
            let node = crate::parser::parse_expr(&brick.expression())
                .unwrap_or_else(|error| panic!("{module}: {error}"));
            assert!(!node.is_opaque(), "{module}: {}", brick.expression());
            assert_eq!(node.base.path(), Some(format!("{}::new", brick.type_name).as_str()));
        }
    }

    /// A file with no public struct, or no `new`, is not offered.
    #[test]
    fn what_cannot_be_written_is_not_offered() {
        assert!(read_one("helper", "pub fn helper() {}\n").is_none(), "no struct");
        assert!(
            read_one("private", "struct Card;\nimpl Card { pub fn new() -> Self { Card } }\n")
                .is_none(),
            "not public"
        );
        assert!(
            read_one("no_new", "pub struct Card;\nimpl Card { pub fn draw(&self) {} }\n").is_none(),
            "no constructor"
        );
        // A `new` on a trait implementation is not the constructor.
        assert!(
            read_one(
                "trait_new",
                "pub struct Card;\nimpl Default for Card { fn new() -> Self { Card } }\n"
            )
            .is_none(),
            "a trait's method is not the type's constructor"
        );
    }
}
