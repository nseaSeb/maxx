//! Every module of `gpui-component` is either offered, set aside, or on the
//! list — and nothing else.
//!
//! The catalogue is written against one version of the crate. A version bump
//! adds modules, and nothing says so: maxx keeps compiling, the palette keeps
//! offering exactly what it offered yesterday, and a component that would have
//! cost an entry in a table goes unnoticed for a year. The same gesture
//! `tests/locales.rs` makes for the translation keys — refuse what is neither
//! used nor declared useless — applied to the library maxx draws with.
//!
//! So this test reads the crate's own sources, out of the cargo registry, at
//! the version `Cargo.lock` pins. Not the documentation, not a list copied by
//! hand: the directory the compiler itself reads.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use maxx::registry::CATALOGUE;

/// What the palette drops, deduced rather than listed.
///
/// A catalogue entry names its module in the `use` line it writes into the
/// generated file, so the covered set is a fact of the table, not a second
/// table to keep in step with it.
///
/// Two exceptions, and they are the reason this constant exists at all: a
/// component re-exported from the crate root names no module in its import,
/// and a property is not a component.
const OFFERED_BESIDE_THE_IMPORTS: &[&str] = &[
    // `use gpui_component::{Icon, IconName};` — the icon is offered, but the
    // crate re-exports it from the root, so its module never appears.
    "icon",
    // Not an entry but a property, `Target::Tooltip`: the closure it writes
    // brings `use gpui_component::tooltip::Tooltip;` with it, from
    // `registry::imports` rather than from a spec's own table.
    "tooltip",
];

/// What the roadmap turned down, and why.
///
/// The reason is the point of the list. A module lands here because a decision
/// was taken about it, and the decision is what a later reader needs — not the
/// name, which they can read in the crate.
const SET_ASIDE: &[&str] = &[
    // Delegates: their content comes from a trait the developer implements,
    // which is Rust to write, not a tree to draw.
    "table",
    "tree",
    "list",
    "virtual_list",
    // Data: a chart is a series, and maxx has no way to hold one — the view
    // holds a tree of components, not a dataset.
    "chart",
    "plot",
    // A dependency of its own: `webview` pulls `wry` into the generated
    // project, which no palette drop should ever decide.
    "webview",
    // Multiple slots: a header and a body, two children that are not siblings.
    // Chantier 2 measured what that costs the model, and the answer stands.
    "accordion",
    "collapsible",
    "form",
    "description_list",
    "popover",
    // A constructor asking for a frame maxx does not have there:
    // `TextView::markdown` takes a `&mut Window` and a `&mut App`, and both
    // ends refuse it. The generated `render` leaves its window unused under an
    // underscore, and the canvas draws a node from a function that has no
    // window at all. The rule the catalogue follows — nothing that needs a
    // closure, a named slot or a `&mut Window` — names this case exactly.
    "text",
    // Covered by the manager rather than by the tree: these are opened from
    // code, on a window, not laid down in a view.
    "dialog",
    "sheet",
    "notification",
    // Infrastructure — maxx's own, or the shell the generated project gets
    // from `scaffold`. Real modules, used, but never a brick to drop on a
    // canvas.
    "root",
    "theme",
    "styled",
    "actions",
    "animation",
    "event",
    "geometry",
    "global_state",
    "history",
    "index_path",
    "inspector",
    "highlighter",
    "resizable",
    "dock",
    "sidebar",
    "title_bar",
    "window_border",
    "menu",
    "setting",
];

/// What the roadmap names as coming, in `BACKLOG.md`, under *Les composants
/// gratuits*.
///
/// The difference with [`SET_ASIDE`] is a promise: a name here is one the next
/// cycle adds to the catalogue, and moving it into the deduced set is what
/// closes the point.
const TO_LOOK_AT: &[&str] = &[];

/// The modules the crate publishes under another name.
///
/// `pub use time::{calendar, date_picker};` — the two names a `use` line can
/// spell are not modules of `src/`, and the module that does hold them appears
/// in no import at all. Without this table the two would be reported as
/// classified but gone, and `time` as never decided about: three failures for
/// one component that is very much offered.
const REEXPORTED: &[(&str, &str)] = &[("calendar", "time"), ("date_picker", "time")];

/// The version `Cargo.lock` pins `name` to.
///
/// Copied from `build.rs` rather than shared: a build script is a program of
/// its own, and nothing in the crate can import it. The same reason
/// `src/scaffold/templates.rs` depends on nothing.
fn locked_version(lock: &str, name: &str) -> Option<String> {
    let needle = format!("name = \"{name}\"");
    let mut lines = lock.lines().skip_while(|line| line.trim() != needle);
    lines.next()?;
    lines
        .next()
        .and_then(|line| line.trim().strip_prefix("version = "))
        .map(|version| version.trim_matches('"').to_string())
}

/// Where cargo unpacked the sources of `gpui-component`.
///
/// A failure here is a failure of the test, never a pass: an unpopulated
/// registry would turn this whole file into a silent no-op, which is exactly
/// the kind of thing it exists to catch elsewhere. The CI has the sources — it
/// cannot have built the crate without them.
fn crate_sources() -> PathBuf {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let lock = std::fs::read_to_string(manifest.join("Cargo.lock")).expect("Cargo.lock");
    let version = locked_version(&lock, "gpui-component")
        .expect("Cargo.lock must pin gpui-component: maxx does not build without it");

    let home = std::env::var("CARGO_HOME").map(PathBuf::from).unwrap_or_else(|_| {
        let home = std::env::var("HOME").expect("HOME must be set to find ~/.cargo");
        PathBuf::from(home).join(".cargo")
    });
    let registry = home.join("registry/src");

    // One directory per registry index, named after a hash nobody can guess.
    let indexes = std::fs::read_dir(&registry).unwrap_or_else(|error| {
        panic!(
            "{}: {error}. Build the crate once so cargo unpacks its sources.",
            registry.display()
        )
    });
    for index in indexes.flatten() {
        // Exactly the directory, not a prefix: `gpui-component-macros-0.5.1`
        // sits right beside `gpui-component-0.5.1` and would match a glob.
        let candidate = index.path().join(format!("gpui-component-{version}")).join("src");
        if candidate.is_dir() {
            return candidate;
        }
    }
    panic!(
        "gpui-component-{version}/src found nowhere under {}. \
         Run `cargo build` so cargo unpacks the sources this test reads.",
        registry.display()
    )
}

/// The crate's top-level modules: `src/*.rs` and `src/*/mod.rs`.
fn modules(src: &Path) -> BTreeSet<String> {
    let mut names = BTreeSet::new();
    for entry in std::fs::read_dir(src).expect("the crate's src").flatten() {
        let path = entry.path();
        let Some(name) = path.file_stem().and_then(|stem| stem.to_str()) else { continue };
        if path.is_dir() {
            if path.join("mod.rs").is_file() {
                names.insert(name.to_string());
            }
        } else if path.extension().is_some_and(|ext| ext == "rs") && name != "lib" {
            names.insert(name.to_string());
        }
    }
    assert!(!names.is_empty(), "{}: no module read", src.display());
    names
}

/// The modules the catalogue writes `use` lines for.
fn offered() -> BTreeSet<String> {
    let mut names: BTreeSet<String> =
        OFFERED_BESIDE_THE_IMPORTS.iter().map(|name| (*name).to_string()).collect();
    for spec in CATALOGUE {
        let extra = spec.extra_imports.iter().map(|(_, line)| *line);
        let state = spec.state.iter().flat_map(|state| state.imports.iter().copied());
        for line in spec.imports.iter().copied().chain(extra).chain(state) {
            // `use gpui_component::<module>::…` — anything shorter is a
            // re-export from the crate root, which names no module.
            let Some(rest) = line.trim_start_matches("use ").strip_prefix("gpui_component::")
            else {
                continue;
            };
            let Some((module, _)) = rest.split_once("::") else { continue };
            let module = REEXPORTED
                .iter()
                .find(|(name, _)| *name == module)
                .map_or(module, |(_, real)| *real);
            names.insert(module.to_string());
        }
    }
    names
}

/// Every icon the crate names is one the palette offers.
///
/// The generated pair is only as good as what it was generated from: a parse
/// that quietly read half the enum would leave the inspector with half the
/// icons and nothing to say so. This reads the enum a second time, the plain
/// way, and holds the offered list to it.
#[test]
fn every_icon_of_the_crate_is_offered() {
    let source = crate_sources().join("icon.rs");
    let text = std::fs::read_to_string(&source).expect("gpui-component's icon.rs");
    let offset = text.find("pub enum IconName {").expect("the enum") + "pub enum IconName {".len();
    let variants: BTreeSet<String> = text[offset..]
        .lines()
        .take_while(|line| line.trim() != "}")
        .filter_map(|line| line.trim().strip_suffix(','))
        .filter(|name| name.chars().all(|c| c.is_ascii_alphanumeric()))
        .map(|name| format!("IconName::{name}"))
        .collect();
    assert!(variants.len() > 50, "{}: {} variants read", source.display(), variants.len());

    let offered: BTreeSet<String> = CATALOGUE
        .iter()
        .flat_map(|spec| spec.props.iter())
        .filter_map(|prop| match prop.target {
            maxx::registry::Target::VariantArg(_, values) if prop.label == "prop.icon" => {
                Some(values)
            }
            _ => None,
        })
        .flatten()
        .map(|value| (*value).to_string())
        .collect();

    let missing: Vec<&String> = variants.difference(&offered).collect();
    let extra: Vec<&String> = offered.difference(&variants).collect();
    assert!(
        missing.is_empty() && extra.is_empty(),
        "the crate names icons the palette does not offer: {missing:?}\n\
         and the palette offers names the crate does not have: {extra:?}\n\
         Both come from build.rs — remove target/debug/build/maxx-* and build again."
    );
}

#[test]
fn every_module_of_gpui_component_is_classified() {
    let src = crate_sources();
    let modules = modules(&src);
    let offered = offered();

    let mut classified: BTreeSet<String> = offered.clone();
    for name in SET_ASIDE.iter().chain(TO_LOOK_AT) {
        assert!(
            classified.insert((*name).to_string()),
            "{name}: classified twice in tests/components.rs"
        );
    }

    // A module nobody has looked at — the one thing this test is for — and a
    // name kept in a list after the crate dropped it, which is a decision about
    // nothing and hides the next real one. Reported together on purpose: a
    // renamed module is both halves at once, and telling only one of them turns
    // a rename into two runs.
    let unclassified: Vec<&String> =
        modules.iter().filter(|name| !classified.contains(*name)).collect();
    let stale: Vec<&String> = classified.iter().filter(|name| !modules.contains(*name)).collect();
    assert!(
        unclassified.is_empty() && stale.is_empty(),
        "gpui-component {version} has modules maxx has never decided about: {unclassified:?} — \
         classe-le dans tests/components.rs.\n\
         And these are classified here but no longer exist in the crate: {stale:?} — \
         the list is stale, drop them.",
        version = src.parent().and_then(|dir| dir.file_name()).unwrap_or_default().display()
    );
}
