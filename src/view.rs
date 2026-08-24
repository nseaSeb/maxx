//! One view file, loaded and saved.
//!
//! The file text is kept whole in memory. Saving re-renders only the managed
//! region and splices it back, then makes sure the imports and the input-state
//! fields the tree needs exist — again by textual insertion, never by rewriting
//! the surrounding code.

use std::path::{Path, PathBuf};

use crate::model::{Base, Node};
use crate::{codegen, parser, registry};

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
        })
    }

    /// Whether the tree differs from what is on disk. Derived rather than
    /// flagged, so undoing back to the saved state reports clean again.
    pub fn dirty(&self) -> bool {
        self.root != self.saved
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
        let indent = parser::locate(&self.source)
            .map_err(|error| error.to_string())?
            .indent;
        let block = codegen::render_for_splice(&self.root, indent);

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
        .filter(|line| !source.contains(line))
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

    if let Some(offset) = source.find("pub struct ")
        && let Some(brace) = source[offset..].find('{').map(|index| offset + index)
    {
        let declaration = format!("    pub {field}: Entity<InputState>,\n");
        source = insert_into_block(source, brace, &declaration);
    }

    if let Some(offset) = source.find("        Self {")
        && let Some(brace) = source[offset..].find('{').map(|index| offset + index)
    {
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
    if source.contains(&format!("fn {name}(")) {
        return source;
    }

    // `Context` and `Window` already come from the template's braced import;
    // adding them again would be a duplicate-import error.
    let mut source = ensure_imports(source, &["use gpui::ClickEvent;"]);

    // `cx.listener(..)` needs the parameter the template leaves unused.
    source = source.replace(
        "fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>)",
        "fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>)",
    );

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

/// The name of the type the file declares.
fn view_type_name(source: &str) -> Option<String> {
    let offset = source.find("pub struct ")? + "pub struct ".len();
    let rest = &source[offset..];
    let end = rest.find(|character: char| !character.is_alphanumeric() && character != '_')?;
    Some(rest[..end].to_string())
}

/// The offset of the `}` closing the `{` at `open`.
fn matching_brace(source: &str, open: usize) -> Option<usize> {
    let mut depth = 0usize;
    for (offset, character) in source[open..].char_indices() {
        match character {
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    return Some(open + offset);
                }
            }
            _ => {}
        }
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
