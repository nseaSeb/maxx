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
    /// How many arguments `new` takes, all of them text.
    pub arity: usize,
    /// Whether `mod.rs` re-exports the type, which decides the `use` line.
    reexported: bool,
    /// The builder methods the inspector can offer, in declaration order.
    pub props: Vec<BrickProp>,
}

/// One `pub fn x(mut self, …) -> Self` of a component.
///
/// The shape *is* the property, which is the whole reason this works without a
/// declaration: a builder method taking one string is a text field, one taking
/// nothing is a switch — the call is there or it is not. Anything else is not
/// offered, for the same reason a constructor maxx cannot fill is not.
#[derive(Clone, Debug, PartialEq)]
pub struct BrickProp {
    /// The method's name, which is also the call written on the node.
    pub method: String,
    /// Whether it takes a string. `false` is the switch.
    pub text: bool,
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
    /// Read from `mod.rs` rather than assumed. maxx's own bricks are
    /// re-exported, so `use crate::components::Card;` is right for them — but a
    /// developer who wrote `mod badge;` and no re-export would have received a
    /// line that does not resolve, spliced into their view by maxx, and their
    /// project would stop compiling on it.
    pub fn import(&self) -> String {
        if self.reexported {
            format!("use crate::components::{};", self.type_name)
        } else {
            format!("use crate::components::{}::{};", self.module, self.type_name)
        }
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
    // What `mod.rs` says, because it decides how a view names each type — and
    // whether it is reachable at all.
    let declarations = std::fs::read_to_string(directory.join("mod.rs")).unwrap_or_default();

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
        let Some(mut brick) = read_one(&module, &source) else {
            continue;
        };
        // A file nothing declares is not part of the project: it is not
        // compiled, and a view naming it would not build.
        if !declares(&declarations, &module) {
            continue;
        }
        brick.reexported = reexports(&declarations, &module, &brick.type_name);
        // A name the catalogue already answers to is worse than no name.
        // `Badge::new(..)` would match maxx's own `Badge`, so the save would
        // write both `use gpui_component::badge::Badge;` and the project's own
        // — E0252, on two lines maxx wrote — while the canvas drew the wrong
        // component and the inspector offered properties the type has not.
        if crate::registry::by_path(&format!("{}::new", brick.type_name)).is_some() {
            continue;
        }
        out.push(brick);
    }
    // Read from a directory, so in whatever order the filesystem answers:
    // sorted, or the palette would reshuffle itself between two launches.
    out.sort_by(|a, b| a.type_name.cmp(&b.type_name));
    out
}

/// The component one file holds, when it holds one maxx can offer.
fn read_one(module: &str, source: &str) -> Option<Brick> {
    let file = syn::parse_file(source).ok()?;

    let public: Vec<String> = file
        .items
        .iter()
        .filter_map(|item| match item {
            syn::Item::Struct(item) if matches!(item.vis, syn::Visibility::Public(_)) => {
                Some(item.ident.to_string())
            }
            _ => None,
        })
        .collect();

    // The public struct that *has* the constructor, and not the first one the
    // file declares: a `pub struct CardStyle` written above `pub struct Card`
    // would otherwise be what the palette offered.
    let (type_name, arity, props) = file.items.iter().find_map(|item| {
        let syn::Item::Impl(block) = item else {
            return None;
        };
        if block.trait_.is_some() {
            return None;
        }
        let name = public.iter().find(|name| impl_of(block, name))?;
        let props = block
            .items
            .iter()
            .filter_map(|item| {
                let syn::ImplItem::Fn(function) = item else {
                    return None;
                };
                read_prop(function)
            })
            .collect();
        let arity = block.items.iter().find_map(|item| match item {
            syn::ImplItem::Fn(function)
                if function.sig.ident == "new"
                    && matches!(function.vis, syn::Visibility::Public(_)) =>
            {
                // Every argument has to be one maxx can write, and all it knows
                // how to write is a string. `pub fn new(window: &mut Window, cx:
                // &mut Context<Self>)` — the commonest constructor in all of
                // GPUI — would otherwise be offered and dropped as
                // `Foo::new("Text", "Text")`: a node that parses, so nothing
                // complains, and a type error the developer finds in Zed.
                function.sig.inputs.iter().all(is_text).then_some(function.sig.inputs.len())
            }
            _ => None,
        })?;
        Some((name.clone(), arity, props))
    })?;

    Some(Brick {
        type_name,
        module: module.to_string(),
        doc: first_doc_line(source),
        arity,
        reexported: false,
        props,
    })
}

/// The property a builder method is, when it is one.
///
/// `pub fn x(mut self, …) -> Self` and nothing else. The receiver has to be
/// `mut self` and the return `Self`: `pub fn label(&self) -> &str` is a reader,
/// and writing `.label()` on the node would be nonsense.
fn read_prop(function: &syn::ImplItemFn) -> Option<BrickProp> {
    if !matches!(function.vis, syn::Visibility::Public(_)) || function.sig.ident == "new" {
        return None;
    }
    let mut inputs = function.sig.inputs.iter();
    // Taken by value, so the builder consumes and returns itself. A `&self`
    // reads rather than builds, and writing `.label()` on the node for it would
    // be nonsense.
    let syn::FnArg::Receiver(receiver) = inputs.next()? else {
        return None;
    };
    if quote::quote!(#receiver).to_string().contains('&') {
        return None;
    }
    let syn::ReturnType::Type(_, returned) = &function.sig.output else {
        return None;
    };
    if quote::quote!(#returned).to_string() != "Self" {
        return None;
    }

    match (inputs.next(), inputs.next()) {
        (None, _) => Some(BrickProp { method: function.sig.ident.to_string(), text: false }),
        (Some(argument), None) if is_text(argument) => {
            Some(BrickProp { method: function.sig.ident.to_string(), text: true })
        }
        _ => None,
    }
}

/// Whether this parameter is one maxx knows how to write: a string.
///
/// Spelled the four ways a constructor takes one. Anything else — a number, a
/// list, a `&mut Window` — is a value maxx has nothing to put in, so the
/// component holding it is not offered rather than offered wrongly.
fn is_text(argument: &syn::FnArg) -> bool {
    let syn::FnArg::Typed(typed) = argument else {
        return false;
    };
    let rendered = quote::quote!(#typed).to_string().replace(' ', "");
    rendered.ends_with("implInto<SharedString>")
        || rendered.ends_with(":SharedString")
        || rendered.ends_with(":String")
        || rendered.ends_with(":&str")
        || rendered.ends_with("implInto<String>")
}

/// Whether `mod.rs` declares this module.
fn declares(declarations: &str, module: &str) -> bool {
    let needle = format!("mod {module};");
    declarations.lines().any(|line| line.trim().trim_start_matches("pub ") == needle)
}

/// Whether `mod.rs` re-exports the type, so a view can name it directly.
fn reexports(declarations: &str, module: &str, type_name: &str) -> bool {
    let needle = format!("pub use {module}::{type_name};");
    declarations.lines().any(|line| line.trim() == needle)
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

    /// The builder methods of maxx's own bricks are read as properties.
    ///
    /// The other half of the contract between the library and the reader: a
    /// brick whose builder drifted out of the shape read here loses its
    /// properties from the inspector, silently, and this is what says so.
    #[test]
    fn the_builders_of_maxx_s_bricks_are_read_as_properties() {
        let read = |name: &str| {
            let (module, body) = crate::scaffold::templates::COMPONENTS
                .iter()
                .find(|(module, _)| *module == name)
                .expect(name);
            read_one(module, body).expect(name)
        };
        let card = read("card");
        assert_eq!(
            card.props,
            vec![BrickProp { method: "subtitle".into(), text: true }],
            "a builder taking one string is a text field"
        );
        let toolbar = read("toolbar");
        assert_eq!(
            toolbar.props,
            vec![BrickProp { method: "separated".into(), text: false }],
            "a builder taking nothing is a switch"
        );
    }

    /// What is not a builder is not a property.
    #[test]
    fn a_reader_is_not_a_property() {
        let source = "\
pub struct Card;
impl Card {
    pub fn new() -> Self { Card }
    pub fn label(&self) -> &str { \"\" }
    fn hidden(mut self, v: String) -> Self { self }
    pub fn size(mut self, v: usize) -> Self { self }
    pub fn both(mut self, a: String, b: String) -> Self { self }
    pub fn title(mut self, v: impl Into<SharedString>) -> Self { self }
}
";
        let brick = read_one("card", source).expect("Card");
        assert_eq!(
            brick.props,
            vec![BrickProp { method: "title".into(), text: true }],
            "only `pub fn x(mut self, one string) -> Self` is a property"
        );
    }

    /// A constructor maxx cannot fill is not offered.
    ///
    /// The commonest constructor in all of GPUI takes a window and a context.
    /// Offered, it would have been dropped as `Foo::new("Text", "Text")` — a
    /// node that parses, so nothing complains, and a type error the developer
    /// finds in Zed on a line maxx wrote.
    #[test]
    fn a_constructor_maxx_cannot_fill_is_not_offered() {
        assert!(
            read_one(
                "screen",
                "pub struct Screen;\nimpl Screen { pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self { Screen } }\n"
            )
            .is_none(),
            "a window and a context are not text"
        );
        assert!(
            read_one("counter", "pub struct Counter;\nimpl Counter { pub fn new(count: usize) -> Self { Counter } }\n")
                .is_none(),
            "a number is not text"
        );
        // And the four spellings of a string that are.
        for argument in [
            "title: impl Into<SharedString>",
            "title: SharedString",
            "title: String",
            "title: &str",
        ] {
            let source = format!(
                "pub struct Card;\nimpl Card {{ pub fn new({argument}) -> Self {{ Card }} }}\n"
            );
            assert_eq!(read_one("card", &source).map(|b| b.arity), Some(1), "{argument}");
        }
    }

    /// The struct that holds the constructor, not the first one in the file.
    #[test]
    fn the_offered_type_is_the_one_with_the_constructor() {
        let source = "pub struct CardStyle;\npub struct Card;\nimpl Card { pub fn new() -> Self { Card } }\n";
        assert_eq!(read_one("card", source).map(|b| b.type_name), Some("Card".to_string()));
    }

    /// The `use` line follows what `mod.rs` actually offers.
    #[test]
    fn the_import_follows_the_declaration() {
        let reexported = Brick {
            type_name: "Card".into(),
            module: "card".into(),
            doc: String::new(),
            arity: 0,
            reexported: true,
            props: Vec::new(),
        };
        assert_eq!(reexported.import(), "use crate::components::Card;");
        // Theirs, declared but not re-exported: naming it directly would not
        // resolve, and maxx would have written that line into their view.
        let theirs = Brick { reexported: false, ..reexported };
        assert_eq!(theirs.import(), "use crate::components::card::Card;");
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
