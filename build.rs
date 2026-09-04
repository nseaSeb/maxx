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
    write_icons();

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

    // The pages the shapes bring with them, each in a module of its own — the
    // shape it has in a project, where every page is one file of `src/ui/`.
    // These are views and not hand-written screens, so what is checked here is
    // the half maxx does not rewrite at a save: the struct, the fields the tree
    // binds, and the methods the tree names.
    let mut pages = String::new();
    for (module, source) in SHAPE_PAGES {
        pages.push_str(&format!("pub mod {module} {{\n"));
        pages.push_str(&as_module_body(&source()));
        pages.push_str("}\n");
    }
    std::fs::write(out.join("pages.rs"), pages).expect("the pages must be written");

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

    // The sub-tree templates, each inside the one place it can land: a view.
    // Same reason as the boxes — a call gpui-component drops has to fail here
    // rather than in a project, on a line maxx wrote — but a template may bind
    // `&self.field` and name `Self::on_click`, and neither of those means
    // anything in a free function. So each one is compiled as the body of a
    // `Render`, with the fields the table declares and a stub for every handler
    // it names: exactly what `ensure_state_field` and `ensure_handler` write
    // into the developer's file at the save.
    let mut subtrees = String::new();
    let mut seen: Vec<&str> = Vec::new();
    for (id, imports, state, source) in SUBTREES {
        for import in *imports {
            if !seen.contains(import) {
                seen.push(import);
                subtrees.push_str(import);
                subtrees.push('\n');
            }
        }
        // Paths are written out in full below so that the wrapper owes no `use`
        // line: an import added for a check and unused by the next template is
        // a warning in a build that refuses them.
        let name = camel_case(id);
        subtrees.push_str(&format!("#[allow(dead_code)]\nstruct {name} {{\n"));
        for field in *state {
            subtrees.push_str(&format!("    {field},\n"));
        }
        subtrees.push_str("}\n");
        let cx = if source.contains("cx.") { "cx" } else { "_cx" };
        subtrees.push_str(&format!(
            "impl gpui::Render for {name} {{\n    fn render(&mut self, _window: &mut gpui::Window, {cx}: &mut gpui::Context<Self>) -> impl gpui::IntoElement {{\n        {source}\n    }}\n}}\n"
        ));
        // A `&ClickEvent` because every handler a template writes today sits on
        // a button. One posed on a switch would take the new state instead, and
        // this is where that would have to be told apart.
        for handler in handler_names(source) {
            subtrees.push_str(&format!(
                "#[allow(dead_code)]\nimpl {name} {{\n    fn {handler}(&mut self, _event: &gpui::ClickEvent, _window: &mut gpui::Window, _cx: &mut gpui::Context<Self>) {{\n    }}\n}}\n"
            ));
        }
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

/// Writes the icon tables into `OUT_DIR`, read from `gpui-component`'s own
/// `IconName`.
///
/// Eighty-six variants, and every one of them has to appear twice: once in the
/// list the inspector offers, once in the match the canvas draws from. Kept by
/// hand, that pair drifted the moment the crate added an icon — twenty-two were
/// offered out of eighty-six, and the missing ones were not missing on purpose.
///
/// `IconName` has no `FromStr`, no `Display` and no way to enumerate itself in
/// 0.5.1, so the enum is read where the compiler reads it: the sources cargo
/// unpacked, at the version `Cargo.lock` pins. The same lookup `tests/components.rs`
/// makes, and copied for the same reason — a build script cannot import the
/// crate it builds.
fn write_icons() {
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR"));
    let lock = std::fs::read_to_string(manifest.join("Cargo.lock")).expect("Cargo.lock");
    let version = locked_version(&lock, "gpui-component")
        .expect("Cargo.lock must pin gpui-component: maxx does not build without it");
    let source = icon_source(&version);
    println!("cargo::rerun-if-changed={}", source.display());

    let text = std::fs::read_to_string(&source)
        .unwrap_or_else(|error| panic!("{}: {error}", source.display()));
    let names = icon_variants(&text);
    // A parse that reads nothing would leave the palette with an empty list and
    // say nothing about it — the silent hole this whole table exists to close.
    assert!(
        names.len() > 50,
        "{}: {} variants read, expected the whole enum",
        source.display(),
        names.len()
    );

    let out = std::path::PathBuf::from(std::env::var("OUT_DIR").expect("cargo sets OUT_DIR"));

    let mut list = String::from(
        "// Written by build.rs from gpui-component's own `IconName`. Do not edit.\n\
         /// Every icon the crate names, offered whole.\n\
         const ICONS: &[&str] = &[\n",
    );
    for name in &names {
        list.push_str(&format!("    \"IconName::{name}\",\n"));
    }
    list.push_str("];\n");
    std::fs::write(out.join("icons.rs"), list).expect("the icon list must be written");

    let mut table = String::from(
        "// Written by build.rs from gpui-component's own `IconName`. Do not edit.\n\
         /// The variant a `IconName::…` path names, when the crate has one.\n\
         ///\n\
         /// `None` for a name written by hand that the crate does not carry: the\n\
         /// canvas draws its fallback rather than a wrong icon.\n\
         pub fn icon_name(source: &str) -> Option<gpui_component::IconName> {\n\
         \x20   use gpui_component::IconName;\n\
         \x20   Some(match source {\n",
    );
    for name in &names {
        table.push_str(&format!("        \"IconName::{name}\" => IconName::{name},\n"));
    }
    table.push_str("        _ => return None,\n    })\n}\n");
    std::fs::write(out.join("icon_name.rs"), table).expect("the icon table must be written");
}

/// Where cargo unpacked `gpui-component`'s `icon.rs`.
fn icon_source(version: &str) -> std::path::PathBuf {
    let home = std::env::var("CARGO_HOME").map(std::path::PathBuf::from).unwrap_or_else(|_| {
        let home = std::env::var("HOME").expect("HOME must be set to find ~/.cargo");
        std::path::PathBuf::from(home).join(".cargo")
    });
    let registry = home.join("registry/src");
    let indexes = std::fs::read_dir(&registry)
        .unwrap_or_else(|error| panic!("{}: {error}", registry.display()));
    for index in indexes.flatten() {
        // Exactly the directory, not a prefix: `gpui-component-macros-0.5.1`
        // sits right beside `gpui-component-0.5.1` and would match a glob.
        let candidate = index.path().join(format!("gpui-component-{version}")).join("src/icon.rs");
        if candidate.is_file() {
            return candidate;
        }
    }
    panic!("gpui-component-{version}/src/icon.rs found nowhere under {}", registry.display())
}

/// The variants of `enum IconName`, in the order the crate declares them.
///
/// Read by scanning rather than parsed: a build script that pulled `syn` in
/// would make every build of maxx wait for it, to answer a question one brace
/// and one indent already answer.
fn icon_variants(source: &str) -> Vec<String> {
    let Some(offset) = source.find("pub enum IconName {") else {
        return Vec::new();
    };
    let body = &source[offset + "pub enum IconName {".len()..];
    let mut names = Vec::new();
    for line in body.lines() {
        let line = line.trim();
        if line == "}" {
            break;
        }
        // A variant is a bare name and a comma; anything else — an attribute, a
        // comment, a variant carrying data — is not one to write.
        let Some(name) = line.strip_suffix(',') else { continue };
        if !name.is_empty()
            && name.starts_with(|c: char| c.is_ascii_uppercase())
            && name.chars().all(|c| c.is_ascii_alphanumeric())
        {
            names.push(name.to_string());
        }
    }
    names
}

/// `form_field` as the type name a wrapper can wear: `FormField`.
fn camel_case(id: &str) -> String {
    id.split('_')
        .map(|word| {
            let mut characters = word.chars();
            match characters.next() {
                Some(first) => first.to_ascii_uppercase().to_string() + characters.as_str(),
                None => String::new(),
            }
        })
        .collect()
}

/// The methods a template names, read off its `Self::…` as the inspector reads
/// them off the tree.
fn handler_names(source: &str) -> Vec<String> {
    let mut names = Vec::new();
    for (index, _) in source.match_indices("Self::") {
        let rest = &source[index + "Self::".len()..];
        let name: String =
            rest.chars().take_while(|c| *c == '_' || c.is_ascii_alphanumeric()).collect();
        if !name.is_empty() && !names.contains(&name) {
            names.push(name);
        }
    }
    names
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
