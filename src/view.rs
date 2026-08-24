//! One view file, loaded and saved.
//!
//! The file text is kept whole in memory. Saving re-renders only the managed
//! region and splices it back, then makes sure the imports and the input-state
//! fields the tree needs exist — again by textual insertion, never by rewriting
//! the surrounding code.

use std::path::{Path, PathBuf};

use crate::model::{Base, Node};
use crate::parser::matching_brace;
use crate::{codegen, parser, registry};

/// A field declared on the view's struct.
#[derive(Clone, Debug, PartialEq)]
pub struct StateField {
    /// Field name.
    pub name: String,
    /// Type as written in the source.
    pub ty: String,
}

impl StateField {
    /// How a property reads this field: `SharedString` and `String` clone,
    /// anything else is rendered.
    pub fn read_expression(&self) -> String {
        match self.ty.as_str() {
            "SharedString" | "String" => format!("self.{}.clone()", self.name),
            _ => format!("self.{}.to_string()", self.name),
        }
    }
}

/// The kinds of field the state panel can declare.
pub const STATE_TYPES: &[(&str, &str, &str)] = &[
    ("Texte", "SharedString", "\"\".into()"),
    ("Nombre entier", "usize", "0"),
    ("Nombre décimal", "f32", "0.0"),
    ("Booléen", "bool", "false"),
];

/// A view file open in the workshop.
pub struct View {
    /// Absolute path of the `.rs` file.
    pub path: PathBuf,
    /// Full text of the file, markers included.
    pub source: String,
    /// The tree parsed from the managed region.
    pub root: Node,
    /// Path of the selected node, empty for the root.
    pub selected: crate::model::Path,
    /// The tree as it stands on disk, to tell whether there is anything to save.
    saved: Node,
    /// Undo stack for this view: whole-tree snapshots.
    pub past: Vec<Node>,
    /// Redo stack for this view.
    pub future: Vec<Node>,
}

impl View {
    /// Reads and parses a view file.
    pub fn load(path: &Path) -> Result<Self, String> {
        let source = std::fs::read_to_string(path).map_err(|error| error.to_string())?;
        let (root, _) = parser::parse(&source).map_err(|error| error.to_string())?;
        Ok(Self {
            path: path.to_path_buf(),
            source,
            saved: root.clone(),
            root,
            selected: Vec::new(),
            past: Vec::new(),
            future: Vec::new(),
        })
    }

    /// Whether the tree differs from what is on disk. Derived rather than
    /// flagged, so undoing back to the saved state reports clean again.
    pub fn dirty(&self) -> bool {
        self.root != self.saved
    }

    /// The fields declared on the view's struct, read from the source.
    ///
    /// Parsed by scanning rather than through `syn`, for the same reason the
    /// managed region is: the file must come back out as it went in.
    pub fn state_fields(&self) -> Vec<StateField> {
        let mut fields = Vec::new();
        let Some(type_name) = view_type_name(&self.source) else {
            return fields;
        };
        let Some(open) = struct_brace(&self.source, &type_name) else {
            return fields;
        };
        let Some(close) = matching_brace(&self.source, open) else {
            return fields;
        };

        for line in self.source[open + 1..close].lines() {
            let line = line.trim().trim_end_matches(',');
            if line.is_empty() || line.starts_with("//") || line.starts_with("#[") {
                continue;
            }
            let declaration = line.strip_prefix("pub ").unwrap_or(line);
            let Some((name, ty)) = declaration.split_once(':') else {
                continue;
            };
            let name = name.trim();
            if name.is_empty() || !name.chars().all(|c| c == '_' || c.is_alphanumeric()) {
                continue;
            }
            fields.push(StateField {
                name: name.to_string(),
                ty: ty.trim().to_string(),
            });
        }
        fields
    }

    /// The fields that can back a text input.
    pub fn input_state_fields(&self) -> Vec<String> {
        self.state_fields()
            .into_iter()
            .filter(|field| field.ty == "Entity<InputState>")
            .map(|field| field.name)
            .collect()
    }

    /// Adds a field to the view's struct and initializes it in `new`.
    pub fn add_state_field(&mut self, name: &str, ty: &str, initial: &str) -> Result<(), String> {
        if !name
            .chars()
            .next()
            .is_some_and(|first| first == '_' || first.is_alphabetic())
            || !name.chars().all(|c| c == '_' || c.is_alphanumeric())
        {
            return Err(format!("« {name} » n'est pas un nom de champ valide"));
        }
        if self.state_fields().iter().any(|field| field.name == name) {
            return Err(format!("le champ « {name} » existe déjà"));
        }

        // The same gate `write_view` applies: this writes the file too.
        if self.disk_changed() {
            return Err("fichier modifié en dehors de maxx — rechargez d'abord".into());
        }

        let mut source = std::mem::take(&mut self.source);
        let Some(type_name) = view_type_name(&source) else {
            self.source = source;
            return Err("aucun « impl Render for » dans ce fichier".into());
        };
        // Both anchors, or neither: initializing a field that was never
        // declared reports success and leaves the project unbuildable.
        let (Some(_), Some(_)) = (
            struct_brace(&source, &type_name),
            self_brace(&source, &type_name),
        ) else {
            self.source = source;
            return Err(format!(
                "« {type_name} » n'a pas la forme attendue — struct et « Self {{ … }} » introuvables"
            ));
        };
        if let Some(brace) = struct_brace(&source, &type_name) {
            source = insert_into_block(source, brace, &format!("    {name}: {ty},\n"));
        }
        if let Some(brace) = self_brace(&source, &type_name) {
            source = insert_into_block(source, brace, &format!("            {name}: {initial},\n"));
        }
        if ty == "SharedString" {
            source = ensure_imports(source, &["use gpui::SharedString;"]);
        }

        self.source = source;
        std::fs::write(&self.path, &self.source).map_err(|error| error.to_string())?;
        Ok(())
    }

    /// Whether the file on disk differs from the copy this view was built from.
    pub fn disk_changed(&self) -> bool {
        match std::fs::read_to_string(&self.path) {
            Ok(text) => text != self.source,
            // Unreadable is not "changed": refusing to save would not help.
            Err(_) => false,
        }
    }

    /// Re-reads the file, dropping whatever the designer held.
    pub fn reload(&mut self) -> Result<(), String> {
        let reloaded = View::load(&self.path)?;
        self.source = reloaded.source;
        self.root = reloaded.root;
        self.saved = reloaded.saved;
        self.selected.clear();
        self.past.clear();
        self.future.clear();
        Ok(())
    }

    /// The line where a method is declared, for jumping to it in an editor.
    pub fn method_line(&self, name: &str) -> Option<usize> {
        let needle = format!("fn {name}(");
        self.source
            .lines()
            .position(|line| line.contains(&needle))
            .map(|index| index + 1)
    }

    /// The file name, for tabs and the status bar.
    pub fn name(&self) -> String {
        self.path
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_default()
    }

    /// The selected node, falling back to the root.
    pub fn selected(&self) -> &Node {
        self.root.at(&self.selected).unwrap_or(&self.root)
    }

    /// Renders the tree, splices it into the file and writes it to disk.
    pub fn save(&mut self) -> Result<(), String> {
        // `splice` re-indents every line by the marker's own indentation, so
        // the block is rendered flush left and only the width budget knows
        // about that offset.
        // `syn` throws comments away, so rendering the region back would delete
        // one written inside it. Refusing is better than losing it quietly.
        if parser::region_has_comment(&self.source) {
            return Err(
                "un commentaire se trouve dans la zone gérée — l'enregistrement le perdrait"
                    .into(),
            );
        }
        let region = parser::locate(&self.source).map_err(|error| error.to_string())?;
        let block = codegen::render_for_splice(&self.root, region.width());

        let mut source = parser::splice(&self.source, &block).map_err(|error| error.to_string())?;
        source = ensure_imports(source, &registry::imports(&self.root));
        for field in input_fields(&self.root) {
            source = ensure_input_field(source, &field);
        }
        for handler in registry::handlers(&self.root) {
            source = ensure_handler(source, &handler);
        }

        std::fs::write(&self.path, &source).map_err(|error| error.to_string())?;
        self.source = source;
        self.saved = self.root.clone();
        Ok(())
    }
}

/// Inserts the `use` lines that are missing, after the last existing one.
fn ensure_imports(source: String, lines: &[&str]) -> String {
    let missing: Vec<&str> = lines
        .iter()
        .copied()
        .filter(|line| !already_imported(&source, line))
        .collect();
    if missing.is_empty() {
        return source;
    }

    // After the last `use` line of the header, so the block stays together.
    let anchor = source
        .match_indices("\nuse ")
        .last()
        .and_then(|(offset, _)| source[offset + 1..].find('\n').map(|end| offset + 1 + end + 1))
        .unwrap_or(0);

    let mut out = String::with_capacity(source.len() + missing.len() * 48);
    out.push_str(&source[..anchor]);
    for line in missing {
        out.push_str(line);
        out.push('\n');
    }
    out.push_str(&source[anchor..]);
    out
}

/// Whether the item a `use` line brings in is already in scope.
///
/// Comparing whole lines misses `use gpui::{Context, px};`, and appending the
/// line anyway is a duplicate-import error the user then has to fix by hand.
fn already_imported(source: &str, line: &str) -> bool {
    if source.contains(line) {
        return true;
    }
    let Some(item) = line
        .trim()
        .strip_prefix("use ")
        .and_then(|rest| rest.strip_suffix(';'))
    else {
        return false;
    };
    let Some((path, name)) = item.rsplit_once("::") else {
        return false;
    };

    // Statements rather than lines: rustfmt wraps a long braced import over
    // several of them, and a line-by-line scan misses it.
    use_statements(source).iter().any(|statement| {
        let Some(rest) = statement.strip_prefix(&format!("use {path}::")) else {
            return false;
        };
        match rest.strip_prefix('{').and_then(|r| r.strip_suffix('}')) {
            Some(list) => list.split(',').any(|entry| entry.trim() == name),
            None => rest == name,
        }
    })
}

/// The `use …;` statements of a file, whitespace collapsed.
fn use_statements(source: &str) -> Vec<String> {
    let mut statements = Vec::new();
    let mut search = 0;
    while let Some(index) = source[search..].find("use ") {
        let start = search + index;
        let at_line_start = source[..start]
            .chars()
            .last()
            .is_none_or(|c| c == '\n' || c == ' ');
        let Some(end) = source[start..].find(';').map(|offset| start + offset) else {
            break;
        };
        if at_line_start {
            // Whitespace collapsed inside the path, but the `use ` kept: the
            // caller matches on `use <path>::`.
            let body: String = source[start + "use ".len()..end]
                .split_whitespace()
                .collect::<Vec<_>>()
                .join("");
            statements.push(format!("use {body}"));
        }
        search = end + 1;
    }
    statements
}

/// Field names referenced by `Input::new(&self.<field>)` in the tree.
fn input_fields(root: &Node) -> Vec<String> {
    let mut fields = Vec::new();
    root.walk(&mut |_, node| {
        if node.base.path() != Some("Input::new") {
            return;
        }
        let Base::Known { args, .. } = &node.base else {
            return;
        };
        if let Some(name) = args
            .first()
            .map(|arg| arg.to_source())
            .and_then(|source| source.strip_prefix("&self.").map(str::to_string))
            // A name that is not an identifier would be written into the struct
            // as it stands and would not compile.
            && name.chars().next().is_some_and(|first| first == '_' || first.is_alphabetic())
            && name.chars().all(|character| character == '_' || character.is_alphanumeric())
            && !fields.contains(&name)
        {
            fields.push(name);
        }
    });
    fields
}

/// Makes sure the view struct carries `<field>: Entity<InputState>` and builds
/// it in `new`.
///
/// Only the two anchors are touched. If either is missing — a hand-restructured
/// file — nothing is inserted and the developer gets a plain compile error
/// naming the field, which is more useful than a mangled file.
fn ensure_input_field(source: String, field: &str) -> String {
    if source.contains(&format!("{field}: Entity<InputState>")) {
        return source;
    }

    let mut source = ensure_imports(
        source,
        &[
            "use gpui::Entity;",
            "use gpui_component::input::InputState;",
        ],
    );

    let Some(type_name) = view_type_name(&source) else {
        return source;
    };
    // Declaring the field without initializing it, or the reverse, leaves a
    // file that does not compile. Both anchors, or neither.
    let (Some(_), Some(_)) = (
        struct_brace(&source, &type_name),
        self_brace(&source, &type_name),
    ) else {
        return source;
    };

    if let Some(brace) = struct_brace(&source, &type_name) {
        let declaration = format!("    pub {field}: Entity<InputState>,\n");
        source = insert_into_block(source, brace, &declaration);
    }

    if let Some(brace) = self_brace(&source, &type_name) {
        let initializer =
            format!("            {field}: cx.new(|cx| InputState::new(window, cx)),\n");
        source = insert_into_block(source, brace, &initializer);
        // The template's `new` ignores its arguments; the field needs them.
        source = source.replace(
            "pub fn new(_window: &mut Window, _cx: &mut Context<Self>)",
            "pub fn new(window: &mut Window, cx: &mut Context<Self>)",
        );
    }

    source
}

/// Makes sure the view has a `fn <name>(&mut self, _: &ClickEvent, ..)` stub,
/// inserted at the end of the view's own `impl` block.
///
/// An existing method is never touched — the stub is a starting point, and what
/// the developer wrote in it is the whole point of the file.
fn ensure_handler(source: String, name: &str) -> String {
    // `cx.listener(..)` needs the parameter the template leaves unused. Done
    // before the early return below: a handler written by hand still needs the
    // signature fixed, or the generated call does not compile.
    let mut source = source.replace(
        "fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>)",
        "fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>)",
    );

    if source.contains(&format!("fn {name}(")) {
        return source;
    }

    // `Context` and `Window` already come from the template's braced import;
    // adding them again would be a duplicate-import error.
    source = ensure_imports(source, &["use gpui::ClickEvent;"]);

    let Some(type_name) = view_type_name(&source) else {
        return source;
    };
    // The view's own `impl`, not `impl Render for ..`.
    let Some(offset) = source.find(&format!("impl {type_name} {{")) else {
        return source;
    };
    let Some(open) = source[offset..].find('{').map(|index| offset + index) else {
        return source;
    };
    let Some(close) = matching_brace(&source, open) else {
        return source;
    };

    let stub = format!(
        "\n    /// Écrit par maxx ; à toi de le remplir.\n         \x20   pub fn {name}(\n         \x20       &mut self,\n         \x20       _event: &ClickEvent,\n         \x20       _window: &mut Window,\n         \x20       _cx: &mut Context<Self>,\n         \x20   ) {{\n    }}\n"
    );
    source.insert_str(close, &stub);
    source
}

/// The name of the type whose `render` carries the managed region.
///
/// Taking the first `pub struct` of the file instead sent handler stubs and
/// state fields into a helper type declared above the view.
fn view_type_name(source: &str) -> Option<String> {
    let offset = source.find("impl Render for ")? + "impl Render for ".len();
    let rest = &source[offset..];
    let end = rest.find(|character: char| !character.is_alphanumeric() && character != '_')?;
    let name = rest[..end].to_string();
    (!name.is_empty()).then_some(name)
}

/// The `{` opening the declaration of `struct <name>`, whatever its
/// visibility.
///
/// The match is anchored on a word boundary: a prefix match would find
/// `pub struct AppConfig` when looking for `App`, which is the very bug this
/// anchoring was meant to remove.
fn struct_brace(source: &str, name: &str) -> Option<usize> {
    let needle = format!("struct {name}");
    let mut search = 0;
    while let Some(index) = source[search..].find(&needle) {
        let at = search + index;
        let after = at + needle.len();
        let boundary = source[after..]
            .chars()
            .next()
            .is_none_or(|c| !c.is_alphanumeric() && c != '_');
        if boundary {
            return source[at..].find('{').map(|offset| at + offset);
        }
        search = after;
    }
    None
}

/// The `{` opening the `Self {` of that type's `new`.
fn self_brace(source: &str, name: &str) -> Option<usize> {
    let offset = source.find(&format!("impl {name} {{"))?;
    let close = matching_brace(source, source[offset..].find('{').map(|i| offset + i)?)?;
    let block = &source[offset..close];
    // `-> Self {` is the signature, not the struct literal.
    let mut search = 0;
    while let Some(index) = block[search..].find("Self {") {
        let at = search + index;
        if !block[..at].trim_end().ends_with("->") {
            return Some(offset + at + "Self ".len());
        }
        search = at + "Self ".len();
    }
    None
}

/// Inserts `line` just inside the block opening at `brace`, collapsing `{}`
/// into a real block when needed.
fn insert_into_block(mut source: String, brace: usize, line: &str) -> String {
    let rest = &source[brace + 1..];
    let empty = rest.trim_start().starts_with('}');
    let insertion = if empty {
        let close = brace + 1 + rest.find('}').expect("just checked");
        source.replace_range(brace..=close, &format!("{{\n{line}}}"));
        return source;
    } else {
        format!("\n{}", line.trim_end_matches('\n'))
    };
    source.insert_str(brace + 1, &insertion);
    source
}
