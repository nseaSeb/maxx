//! One view file, loaded and saved.
//!
//! The file text is kept whole in memory. Saving re-renders only the managed
//! region and splices it back, then makes sure the imports and the input-state
//! fields the tree needs exist — again by textual insertion, never by rewriting
//! the surrounding code.

use std::path::{Path, PathBuf};

use rust_i18n::t;

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
    ("state_type.text", "SharedString", "\"\".into()"),
    ("state_type.integer", "usize", "0"),
    ("state_type.decimal", "f32", "0.0"),
    ("state_type.boolean", "bool", "false"),
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
    /// `use` lines this view owes to something the catalogue does not know.
    ///
    /// The catalogue's own imports are worked out from the tree at every save,
    /// because a `Spec` carries them. A component of the project carries none —
    /// maxx read it out of the developer's own source — so the line naming it is
    /// put here when it is dropped.
    ///
    /// Kept, but **filtered against the tree at every save**: remembering alone
    /// meant that dropping a brick and pressing `⌘Z` still wrote its `use` line
    /// into a file with nothing to use it. `checkpoint` snapshots the tree and
    /// only the tree, and `ensure_imports` only ever adds — so nothing would
    /// have taken that line back out.
    pub extra_imports: Vec<String>,
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
            extra_imports: Vec::new(),
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
            fields.push(StateField { name: name.to_string(), ty: ty.trim().to_string() });
        }
        fields
    }

    /// The fields that can back a component needing `ty`.
    ///
    /// Filtered on the type: a dropdown cannot be bound to the field of a text
    /// input, and proposing it would be proposing something that will not
    /// compile.
    pub fn state_fields_of_type(&self, ty: &str) -> Vec<String> {
        self.state_fields()
            .into_iter()
            .filter(|field| field.ty == ty)
            .map(|field| field.name)
            .collect()
    }

    /// Adds a field to the view's struct and initializes it in `new`.
    pub fn add_state_field(&mut self, name: &str, ty: &str, initial: &str) -> Result<(), String> {
        if !name.chars().next().is_some_and(|first| first == '_' || first.is_alphabetic())
            || !name.chars().all(|c| c == '_' || c.is_alphanumeric())
        {
            return Err(t!("error.bad_field_name", name = name).into_owned());
        }
        if self.state_fields().iter().any(|field| field.name == name) {
            return Err(t!("error.field_exists", name = name).into_owned());
        }

        // The same gate `write_view` applies: this writes the file too.
        if self.disk_changed() {
            return Err(crate::tr("error.changed_on_disk_reload").to_string());
        }

        let mut source = std::mem::take(&mut self.source);
        let Some(type_name) = view_type_name(&source) else {
            self.source = source;
            return Err(t!("error.no_render_impl").into_owned());
        };
        // Both anchors, or neither: initializing a field that was never
        // declared reports success and leaves the project unbuildable.
        let (Some(_), Some(_)) =
            (struct_brace(&source, &type_name), self_brace(&source, &type_name))
        else {
            self.source = source;
            return Err(t!("error.view_shape", name = type_name).into_owned());
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
        self.source.lines().position(|line| line.contains(&needle)).map(|index| index + 1)
    }

    /// The file name, for tabs and the status bar.
    pub fn name(&self) -> String {
        self.path.file_name().map(|name| name.to_string_lossy().into_owned()).unwrap_or_default()
    }

    /// The selected node, falling back to the root.
    pub fn selected(&self) -> &Node {
        self.root.at(&self.selected).unwrap_or(&self.root)
    }

    /// The file as [`Self::save`] would write it, without writing it.
    ///
    /// Split out of `save` so the code reader can show the same text: what the
    /// canvas is about to produce, rather than what the disk still holds. The
    /// whole pipeline was already textual and pure — only the `fs::write` had
    /// to come out.
    pub fn render_source(&self) -> Result<String, String> {
        // `splice` re-indents every line by the marker's own indentation, so
        // the block is rendered flush left and only the width budget knows
        // about that offset.
        // `syn` throws comments away, so rendering the region back would delete
        // one written inside it. Refusing is better than losing it quietly.
        if parser::region_has_comment(&self.source) {
            return Err(crate::tr("error.comment_in_region").to_string());
        }
        let region = parser::locate(&self.source).map_err(|error| error.to_string())?;
        let block = codegen::render_for_splice(&self.root, region.width());

        let mut source = parser::splice(&self.source, &block).map_err(|error| error.to_string())?;
        let mut needed = registry::imports(&self.root);
        let owed: Vec<&str> = self
            .extra_imports
            .iter()
            .filter(|line| tree_names(&self.root, line))
            .map(String::as_str)
            .collect();
        needed.extend(owed);
        source = ensure_imports(source, &needed);
        for (field, state) in state_fields_needed(&self.root) {
            source = ensure_state_field(source, &field, &state);
        }
        for (handler, shape) in registry::handlers(&self.root) {
            source = ensure_handler(source, &handler, shape);
        }
        // Last, once every import this save owes has been written: the duplicate
        // worth saying something about may be one maxx has just made.
        Ok(flag_duplicate_imports(source))
    }

    /// Renders the tree, splices it into the file and writes it to disk.
    pub fn save(&mut self) -> Result<(), String> {
        let source = self.render_source()?;
        std::fs::write(&self.path, &source).map_err(|error| error.to_string())?;
        self.source = source;
        self.saved = self.root.clone();
        Ok(())
    }
}

/// Inserts the `use` lines that are missing, after the last existing one.
/// Whether the tree still holds something the `use` line names.
///
/// The type is the last segment of the path — `use crate::components::Card;`
/// names `Card` — and a tree holds it when some node is built by its `new`.
fn tree_names(root: &Node, line: &str) -> bool {
    let Some(type_name) = line.trim_end_matches(';').rsplit("::").next() else {
        return false;
    };
    let constructor = format!("{type_name}::new");
    let mut found = false;
    root.walk(&mut |_, node| {
        if node.base.path() == Some(constructor.as_str()) {
            found = true;
        }
    });
    found
}

/// `ensure_imports`, reachable from the test suite.
///
/// The rule it carries is about files maxx did not write, so it is checked on
/// hand-written shapes rather than only through a whole save.
pub fn ensure_imports_for_test(source: String, lines: &[&str]) -> String {
    ensure_imports(source, lines)
}

/// `flag_duplicate_imports`, reachable from the test suite, for the same reason.
pub fn flag_duplicate_imports_for_test(source: String) -> String {
    flag_duplicate_imports(source)
}

fn ensure_imports(source: String, lines: &[&str]) -> String {
    let missing: Vec<String> =
        lines.iter().filter_map(|line| what_is_missing(&source, line)).collect();
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
    for line in &missing {
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
/// The mark maxx leaves above a line it has something to say about.
///
/// Distinct from [`parser::BEGIN`] and [`parser::END`], which are matched
/// whole: this one is a prefix, and nothing looks for it but the code that
/// writes it.
const NOTE: &str = "// maxx: ";

/// Points at an import written twice, and takes the mark back when it is not.
///
/// maxx **adds**; it does not take away what it did not write. A duplicate
/// import is not valid Rust and the file will not build, so the temptation is to
/// merge the two lines and be done — but one of them may be the developer's, and
/// removing their line is a border this program does not cross. A comment says
/// what is wrong, on the line it is wrong on, and leaves the choice where it
/// belongs. Rust ignores it; Zed shows it; the compiler would say the same thing
/// later, and this says it at the save.
///
/// Written once and only once: the mark is recognised on the next pass, so a
/// hundred saves leave one line, and it is taken back — by maxx, from maxx —
/// as soon as the duplicate is gone.
fn flag_duplicate_imports(source: String) -> String {
    // Asked of `syn`, never of the text. A `use a::b::C;` at column zero inside
    // a raw string is not an import, and scanning for one put maxx's comment
    // *inside the string literal* — changing what the developer's code means,
    // silently. `syn` sees items; a string is not one. It settles the inner
    // modules for free: a `use` in a `mod tests` is not a top-level item.
    //
    // A file that does not parse is left exactly as it is, marks included. It
    // is broken for a reason of its own, and taking away a standing warning
    // that cannot be written back is churn in the file and in the diff, at the
    // moment the file needs it most.
    let Ok(parsed) = syn::parse_file(&source) else {
        return source;
    };
    // The `use` keyword's own line, not the item's: a span covers the
    // attributes above it, so `#[allow(unused_imports)]` or a doc comment moved
    // the line one up — the import went unread, and a real duplicate under it
    // went unflagged with it.
    let tops: Vec<usize> = parsed
        .items
        .iter()
        .filter_map(|item| match item {
            syn::Item::Use(item) => Some(item.use_token.span.start().line),
            _ => None,
        })
        .collect();

    // Lines with their own terminators, so an untouched line comes back out
    // byte for byte. Splitting on `\n` and joining on one ending rewrote every
    // line of a file whose endings were mixed — a whole-file diff for a change
    // that did not happen.
    let lines: Vec<&str> = source.split_inclusive('\n').collect();

    let mut seen: Vec<(String, String)> = Vec::new();
    let mut out = String::with_capacity(source.len() + 96);
    for (index, line) in lines.iter().enumerate() {
        let number = index + 1;
        // A mark is maxx's when maxx's sentence stands immediately above a
        // top-level import, which is the only place maxx writes one. The
        // sentence alone was not enough: it was taken out of a raw string as
        // readily as out of the import block.
        if is_our_note(line) && tops.contains(&(number + 1)) {
            continue;
        }
        let names = if tops.contains(&number) { imported_names(line) } else { Vec::new() };
        let duplicate = names.iter().find(|pair| seen.contains(pair)).cloned();
        for pair in names {
            if !seen.contains(&pair) {
                seen.push(pair);
            }
        }
        if let Some((_, name)) = duplicate {
            let ending = if line.ends_with("\r\n") { "\r\n" } else { "\n" };
            out.push_str(&note(&name));
            out.push_str(ending);
        }
        out.push_str(line);
    }
    out
}

/// The end of the sentence maxx writes, held once.
///
/// Written twice — once to build the line, once to recognise it — the two drift
/// apart at the first rewording, and the mark stops being recognised: a new one
/// then stacks above the same import at every save, without a test failing.
const NOTE_TAIL: &str = " is imported twice — one of these two lines has to go.";

/// The sentence maxx writes above an import it has something to say about.
fn note(name: &str) -> String {
    format!("{NOTE}{name}{NOTE_TAIL}")
}

/// Whether this line carries that sentence.
fn is_our_note(line: &str) -> bool {
    let trimmed = line.trim_end_matches(['\n', '\r']);
    trimmed.starts_with(NOTE) && trimmed.ends_with(NOTE_TAIL)
}

/// The `(path, name)` pairs one `use` line imports.
///
/// Only the shapes maxx itself writes and reads: `use a::b::C;` and
/// `use a::b::{C, D};`. A glob, an `as` rename or a nested brace is somebody
/// else's spelling, and saying nothing about it is better than saying something
/// wrong.
fn imported_names(line: &str) -> Vec<(String, String)> {
    let Some(item) = line.trim().strip_prefix("use ").and_then(|rest| rest.strip_suffix(';'))
    else {
        return Vec::new();
    };
    if item.contains(" as ") || item.contains('*') {
        return Vec::new();
    }
    let Some((path, names)) = item.rsplit_once("::") else {
        return Vec::new();
    };
    match names.strip_prefix('{').and_then(|rest| rest.strip_suffix('}')) {
        Some(list) if !list.contains('{') => list
            .split(',')
            .map(str::trim)
            .filter(|name| !name.is_empty())
            .map(|name| (path.to_string(), name.to_string()))
            .collect(),
        Some(_) => Vec::new(),
        None => vec![(path.to_string(), names.to_string())],
    }
}

/// The `use` line to add so that everything `line` names is imported, if any.
///
/// `None` when it all already is. The interesting answer is the third one: a
/// braced needle whose names are only PARTLY there gives back a line for the
/// rest, never itself.
///
/// The failure that asks for it arrived in a real project. An earlier save had
/// written `use gpui_component::button::Button;`; a `.primary()` then made maxx
/// owe `use gpui_component::button::{Button, ButtonVariants};`. Not every name
/// of that needle was present — `ButtonVariants` was not — so it counted as
/// missing and the whole statement was written, leaving the file importing
/// `Button` twice. `E0252`, in the developer's project, on a line maxx wrote,
/// and the view stopped opening.
fn what_is_missing(source: &str, line: &str) -> Option<String> {
    if already_imported(source, line) {
        return None;
    }
    let Some((path, names)) = line
        .trim()
        .strip_prefix("use ")
        .and_then(|rest| rest.strip_suffix(';'))
        .and_then(|item| item.rsplit_once("::"))
    else {
        return Some(line.to_string());
    };
    let Some(list) = names.strip_prefix('{').and_then(|rest| rest.strip_suffix('}')) else {
        return Some(line.to_string());
    };

    let rest: Vec<&str> = list
        .split(',')
        .map(str::trim)
        .filter(|name| !name.is_empty())
        .filter(|name| !already_imported(source, &format!("use {path}::{name};")))
        .collect();
    match rest.len() {
        0 => None,
        // One name left is written plainly: `use a::{B};` is not what anyone
        // would have typed, and rustfmt would take the braces off anyway.
        1 => Some(format!("use {path}::{};", rest[0])),
        _ => Some(format!("use {path}::{{{}}};", rest.join(", "))),
    }
}

fn already_imported(source: &str, line: &str) -> bool {
    if source.contains(line) {
        return true;
    }
    let Some(item) = line.trim().strip_prefix("use ").and_then(|rest| rest.strip_suffix(';'))
    else {
        return false;
    };
    let Some((path, names)) = item.rsplit_once("::") else {
        return false;
    };

    // The needle may be braced itself — `use gpui_component::{Icon, IconName};`
    // — and then every one of its names has to be found, not the brace list as
    // a whole: read as one name, it matches nothing and maxx writes the
    // statement a second time, which is `E0252` in the developer's project.
    let names: Vec<&str> = match names.strip_prefix('{').and_then(|rest| rest.strip_suffix('}')) {
        Some(list) => list.split(',').map(str::trim).collect(),
        None => vec![names],
    };

    names.iter().all(|name| {
        // Statements rather than lines: rustfmt wraps a long braced import over
        // several of them, and a line-by-line scan misses it.
        use_statements(source).iter().any(|statement| {
            let Some(rest) = statement.strip_prefix(&format!("use {path}::")) else {
                return false;
            };
            match rest.strip_prefix('{').and_then(|r| r.strip_suffix('}')) {
                Some(list) => list.split(',').any(|entry| entry.trim() == *name),
                None => rest == *name,
            }
        })
    })
}

/// The `use …;` statements of a file, whitespace collapsed.
fn use_statements(source: &str) -> Vec<String> {
    let mut statements = Vec::new();
    let mut search = 0;
    while let Some(index) = source[search..].find("use ") {
        let start = search + index;
        let at_line_start = source[..start].chars().last().is_none_or(|c| c == '\n' || c == ' ');
        let Some(end) = source[start..].find(';').map(|offset| start + offset) else {
            break;
        };
        if at_line_start {
            // Whitespace collapsed inside the path, but the `use ` kept: the
            // caller matches on `use <path>::`.
            let body: String =
                source[start + "use ".len()..end].split_whitespace().collect::<Vec<_>>().join("");
            statements.push(format!("use {body}"));
        }
        search = end + 1;
    }
    statements
}

/// Field names referenced by `Input::new(&self.<field>)` in the tree.
fn state_fields_needed(root: &Node) -> Vec<(String, registry::StateSpec)> {
    let mut fields: Vec<(String, registry::StateSpec)> = Vec::new();
    root.walk(&mut |_, node| {
        let Some(state) = registry::of(node).and_then(|spec| spec.state) else {
            return;
        };
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
            && !fields.iter().any(|(existing, _)| *existing == name)
        {
            fields.push((name, state));
        }
    });
    fields
}

/// Makes sure the view struct carries `<field>: <type>` and builds it in `new`.
///
/// Only the two anchors are touched. If either is missing — a hand-restructured
/// file — nothing is inserted and the developer gets a plain compile error
/// naming the field, which is more useful than a mangled file.
fn ensure_state_field(source: String, field: &str, state: &registry::StateSpec) -> String {
    if source.contains(&format!("{field}: {}", state.ty)) {
        return source;
    }

    let mut source = ensure_imports(source, state.imports);

    let Some(type_name) = view_type_name(&source) else {
        return source;
    };
    // Declaring the field without initializing it, or the reverse, leaves a
    // file that does not compile. Both anchors, or neither.
    let (Some(_), Some(_)) = (struct_brace(&source, &type_name), self_brace(&source, &type_name))
    else {
        return source;
    };

    if let Some(brace) = struct_brace(&source, &type_name) {
        let declaration = format!("    pub {field}: {},\n", state.ty);
        source = insert_into_block(source, brace, &declaration);
    }

    if let Some(brace) = self_brace(&source, &type_name) {
        let initializer = format!("            {field}: {},\n", state.initializer);
        source = insert_into_block(source, brace, &initializer);
        // The template's `new` ignores its arguments; the field needs them.
        // `cx` always — an entity is built with it — but `window` only when
        // this initializer asks for one: a slider's state does not, and a
        // parameter renamed for nothing leaves an unused-variable warning in
        // the project maxx just wrote.
        // Both are renamed only if this initializer asks for them: a
        // `ScrollHandle` is a plain value, built with neither, and a parameter
        // renamed for nothing leaves an unused-variable warning in the project
        // maxx just wrote.
        if state.initializer.contains("cx") {
            source = source.replace(
                "pub fn new(_window: &mut Window, _cx: &mut Context<Self>)",
                "pub fn new(_window: &mut Window, cx: &mut Context<Self>)",
            );
        }
        if state.initializer.contains("window") {
            source = source
                .replace("pub fn new(_window: &mut Window,", "pub fn new(window: &mut Window,");
        }
    }

    source
}

/// Makes sure the view has a `fn <name>(&mut self, ..)` stub, inserted at the
/// end of the view's own `impl` block.
///
/// `shape` says what the component hands the method — a `&ClickEvent` from a
/// button, the new state from a switch — because a stub that does not match
/// leaves a project that will not compile, on a line maxx wrote.
///
/// An existing method is never touched — the stub is a starting point, and what
/// the developer wrote in it is the whole point of the file.
fn ensure_handler(source: String, name: &str, shape: registry::HandlerSpec) -> String {
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
    source = ensure_imports(source, shape.imports);

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
        "\n    /// Written by maxx; the body is yours.\n         \x20   pub fn {name}(\n         \x20       &mut self,\n         \x20       {},\n         \x20       _window: &mut Window,\n         \x20       _cx: &mut Context<Self>,\n         \x20   ) {{\n    }}\n",
        shape.argument
    );
    source.insert_str(close, &stub);
    source
}

/// Fills an empty handler with the body that opens one of the boxes.
///
/// The boxes gpui-component presents — a dialog, a sheet, a notification — are
/// given imperatively and are never children of a view, so they cannot be a
/// node on the canvas. Their place is the other end of the same gesture: this
/// button opens that box.
///
/// Only an **empty** body is filled. The rule `ensure_handler` follows holds
/// here too and for a stronger reason: this is not a stub being placed, it is a
/// method already on disk, and what a developer wrote in it is the file.
pub fn fill_handler(source: &str, name: &str, kind: &str) -> Result<String, String> {
    let Some((_, imports, body)) =
        crate::scaffold::templates::BOXES.iter().find(|(this, _, _)| *this == kind)
    else {
        return Err(t!("error.no_such_box", name = kind).into_owned());
    };

    let Some(offset) = source.find(&format!("fn {name}(")) else {
        return Err(t!("message.handler_unwritten", name = name).into_owned());
    };
    // The brace of the body, not one of the parameters': a signature holds
    // `&mut Context<Self>` and no braces, but a future one might.
    let Some(close_paren) = source[offset..].find(") {").map(|index| offset + index) else {
        return Err(t!("error.handler_unreadable", name = name).into_owned());
    };
    let open = close_paren + 2;
    let Some(close) = matching_brace(source, open) else {
        return Err(t!("error.handler_unreadable", name = name).into_owned());
    };
    if !source[open + 1..close].trim().is_empty() {
        return Err(t!("error.handler_not_empty", name = name).into_owned());
    }

    let mut filled = String::with_capacity(source.len() + body.len());
    filled.push_str(&source[..open + 1]);
    filled.push_str("\n        ");
    filled.push_str(body);
    filled.push_str("\n    ");
    filled.push_str(&source[close..]);

    // The two parameters the body uses are the ones the stub left unused. Only
    // within this signature: another handler's are still its own.
    let signature = &filled[offset..open];
    let used = signature
        .replace("_window: &mut Window", "window: &mut Window")
        .replace("_cx: &mut Context<Self>", "cx: &mut Context<Self>");
    filled.replace_range(offset..open, &used);

    Ok(ensure_imports(filled, imports))
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
        let boundary =
            source[after..].chars().next().is_none_or(|c| !c.is_alphanumeric() && c != '_');
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
