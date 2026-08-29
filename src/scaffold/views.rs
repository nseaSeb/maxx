//! Views: creating one, renaming one, and saying which one the window opens on.

use std::io;
use std::path::{Path, PathBuf};

use super::modules::{header_end, joined};
use super::to_type_name;
/// Declares `module` in `src/ui/mod.rs`, if it is not there already.
///
/// By textual insertion so the rest of the file — comments, ordering, anything
/// the developer put there — is untouched.
pub(super) fn declare_ui_module(root: &Path, module: &str) -> io::Result<()> {
    let mod_path = root.join("src/ui/mod.rs");
    let mut source = std::fs::read_to_string(&mod_path).unwrap_or_default();
    let line = format!("pub mod {module};\n");
    if source.contains(&line) {
        return Ok(());
    }
    if !source.is_empty() && !source.ends_with('\n') {
        source.push('\n');
    }
    source.push_str(&line);
    std::fs::write(&mod_path, source)
}

/// Adds a view to an existing project and registers it in `src/ui/mod.rs`.
pub fn create_view(root: &Path, module: &str) -> io::Result<()> {
    let type_name = to_type_name(module);
    let file = root.join(format!("src/ui/{module}.rs"));
    if file.exists() {
        return Err(io::Error::new(
            io::ErrorKind::AlreadyExists,
            format!("{} already exists", file.display()),
        ));
    }
    std::fs::write(&file, view_rs(&type_name, module))?;
    declare_ui_module(root, module)
}

/// Makes the view in `module` the one the window opens on.
///
/// Two writes that have to agree: `src/main.rs`, which is what actually opens
/// the window, and `maxx.toml`, which is where maxx reads it back. The code is
/// written first — a `maxx.toml` naming an entry the code does not open would
/// be worse than no record at all.
///
/// The construction site is the truth here, not the `use` line: `main.rs` may
/// import several views, and only one of them is handed to `Root`. What is
/// hard about it is that maxx has to be sure it has found the right one, and
/// [`entry_site`] refuses rather than guesses.
pub fn set_entry_view(root: &Path, module: &str) -> io::Result<()> {
    // Before `main.rs` is touched, not after: `maxx.toml` may be unreadable —
    // one missing bracket in a hand-written `[run]` — and a code that opens the
    // new view while the file still names the old one is the very thing the
    // order above exists to prevent.
    crate::projectfile::check(root)?;

    let file = root.join(format!("src/ui/{module}.rs"));
    let view_source = std::fs::read_to_string(&file)?;
    let Some(type_name) = declared_type(&view_source) else {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("{}: no view type to open the window on", file.display()),
        ));
    };

    let main_path = root.join("src/main.rs");
    let source = std::fs::read_to_string(&main_path)?;
    let mut lines: Vec<String> = source.lines().map(str::to_string).collect();

    let Some((built, current)) = entry_site(&lines) else {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "src/main.rs: maxx cannot tell which view the window opens on — \
             change it by hand",
        ));
    };

    // The call first, and the imports after: inserting a line above `built`
    // would move the very line about to be rewritten. `current` is the path as
    // `main.rs` writes it, which may be qualified in full — replacing only its
    // last segment would leave `crate::ui::home::Other::new(…)`, an old module
    // holding a new type.
    lines[built] = lines[built].replace(&format!("{current}::new("), &format!("{type_name}::new("));

    let import = format!("use crate::ui::{module}::{type_name};");
    let current_type = last_segment(&current);
    let existing = lines.iter().position(|line| is_import_of(line, current_type));
    let already = lines.iter().any(|line| line.trim() == import);
    match (existing, already) {
        // The view was already the entry: nothing to add, and nothing to take
        // away.
        (Some(index), true) if lines[index].trim() == import => {}
        // The new view was imported already, by a `main.rs` that names several:
        // the old import goes, and writing it a second time would be `E0252`.
        (Some(index), true) => {
            lines.remove(index);
        }
        (Some(index), false) => lines[index] = import,
        (None, true) => {}
        // Already there inside a braced import, which `is_import_of` does not
        // touch: writing it again as a line of its own is `E0252`, and taking
        // the group apart is the developer's layout to rewrite.
        (None, false) if imported_in_group(&lines, module, &type_name) => {}
        // A `main.rs` that names the view in full, or imports it through a
        // `use crate::ui::*`: the import goes in rather than being guessed at.
        (None, false) => lines.insert(header_end(&lines), import),
    }

    std::fs::write(&main_path, joined(&lines, &source))?;
    crate::projectfile::set_entry(root, module)
}

/// Renames a view: its file, its module line, its type, and the entry when it
/// was the one the window opens on.
///
/// Answers the files that still name the old view and that maxx does not own —
/// said rather than rewritten. maxx knows three places by construction: the
/// file itself, `src/ui/mod.rs`, and the entry site in `main.rs`. Everything
/// else is the developer's code, where the old name may be a field, a comment
/// or a string, and a blind replacement there is how a tool loses trust.
///
/// Every view maxx creates is called `view_1`, `view_2`, … until it is renamed,
/// so this is not a convenience: it is the step between a view being made and
/// a view being named.
pub fn rename_view(root: &Path, module: &str, renamed: &str) -> io::Result<Vec<PathBuf>> {
    let file = root.join(format!("src/ui/{module}.rs"));
    if renamed == module {
        return Ok(Vec::new());
    }
    let refuse = |reason: String| Err(io::Error::new(io::ErrorKind::InvalidInput, reason));
    if !is_module_name(renamed) {
        return refuse(format!("{renamed} is not a module name"));
    }
    // A keyword passes every character test and then writes `pub mod match;`,
    // which is a project that stops compiling — after the file has moved.
    if RUST_KEYWORDS.contains(&renamed) {
        return refuse(format!("{renamed} is a Rust keyword"));
    }
    let renamed_type = to_type_name(renamed);
    // `_` and `__` are module names Rust accepts, and `to_type_name` answers the
    // empty string for them: the type would be erased rather than renamed.
    if renamed_type.is_empty() {
        return refuse(format!("{renamed} gives no type name"));
    }

    let target = root.join(format!("src/ui/{renamed}.rs"));
    if target.exists() {
        return Err(io::Error::new(
            io::ErrorKind::AlreadyExists,
            format!("{} already exists", target.display()),
        ));
    }

    let source = std::fs::read_to_string(&file)?;
    let Some(type_name) = declared_type(&source) else {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("{}: no view type to rename", file.display()),
        ));
    };

    // The entry is read before anything moves: `entry` names a module, and once
    // the file is renamed there is no longer a way to tell whether it was the
    // one the window opened on.
    let was_entry = crate::projectfile::entry(root).as_deref() == Some(module);

    // `src/ui/mod.rs` is read before anything is written, and its rewrite is
    // required rather than attempted: a rename that cannot reach it would leave
    // a module declared under a name whose file is gone, and report success.
    let mod_path = root.join("src/ui/mod.rs");
    let declarations = std::fs::read_to_string(&mod_path)?;
    let Some(rewritten) = redeclared(&declarations, module, renamed) else {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("{}: no `pub mod {module};` to rewrite", mod_path.display()),
        ));
    };

    // The file first, and the declaration after: a `pub mod` pointing at a file
    // that is not there yet is a project that does not compile, and the window
    // may be closed between the two writes.
    std::fs::write(&target, replace_identifier(&source, &type_name, &renamed_type))?;

    // The entry before the old file goes, and its failure undoes the one write
    // made so far. `set_entry_view` refuses a `maxx.toml` that does not parse
    // and a `main.rs` whose entry site it cannot find — both reachable on a
    // hand-written project — and a rename that stopped halfway there would
    // leave the file moved, the declaration stale and the window still opening
    // on a type that is no longer declared anywhere.
    if was_entry && let Err(error) = set_entry_view(root, renamed) {
        let _ = std::fs::remove_file(&target);
        return Err(error);
    }

    std::fs::write(&mod_path, rewritten)?;
    std::fs::remove_file(&file)?;

    Ok(mentions(root, module, &type_name))
}

/// Whether `name` is spelled the way a module is.
fn is_module_name(name: &str) -> bool {
    name.chars().next().is_some_and(|first| first == '_' || first.is_alphabetic())
        && name.chars().all(|c| c == '_' || c.is_alphanumeric())
}

/// `src/ui/mod.rs` with one module renamed, or `None` when it declares no such
/// module.
///
/// The declaration is matched as the **start** of the trimmed line, not as the
/// whole of it: `pub mod home; // the landing view` is a line the developer is
/// entitled to write, and leaving it alone would leave a module declared under
/// a name whose file has moved — while `mentions` deliberately does not report
/// `mod.rs`, so nothing would say so.
fn redeclared(source: &str, module: &str, renamed: &str) -> Option<String> {
    let declaration = format!("pub mod {module};");
    let mut found = false;
    let lines: Vec<String> = source
        .lines()
        .map(|line| {
            if !found && line.trim_start().starts_with(&declaration) {
                found = true;
                line.replacen(&declaration, &format!("pub mod {renamed};"), 1)
            } else {
                line.to_string()
            }
        })
        .collect();
    if !found {
        return None;
    }
    let mut out = lines.join("\n");
    if source.ends_with('\n') {
        out.push('\n');
    }
    Some(out)
}

/// The words Rust keeps for itself, which a module cannot be called.
///
/// The 2024 list, strict and reserved together: a reserved word is refused by
/// the compiler just as flatly, and a rename that only fails on the next build
/// has already moved the file.
const RUST_KEYWORDS: &[&str] = &[
    "as", "async", "await", "become", "box", "break", "const", "continue", "crate", "do", "dyn",
    "else", "enum", "extern", "false", "final", "fn", "for", "gen", "if", "impl", "in", "let",
    "loop", "macro", "match", "mod", "move", "mut", "override", "priv", "pub", "ref", "return",
    "self", "static", "struct", "super", "trait", "true", "try", "type", "typeof", "unsafe",
    "unsized", "use", "virtual", "where", "while", "yield",
];

/// The project's own files that still name a view maxx has just renamed.
///
/// `src/ui/mod.rs` and `src/main.rs` are left out: those two are the ones maxx
/// has already rewritten, and naming them would be maxx reporting its own work
/// as unfinished.
fn mentions(root: &Path, module: &str, type_name: &str) -> Vec<PathBuf> {
    let mut found = Vec::new();
    let known = [root.join("src/ui/mod.rs"), root.join("src/main.rs")];
    let mut stack = vec![root.join("src")];
    while let Some(directory) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&directory) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
                continue;
            }
            if path.extension().is_none_or(|extension| extension != "rs") || known.contains(&path) {
                continue;
            }
            let Ok(source) = std::fs::read_to_string(&path) else {
                continue;
            };
            if source.contains(&format!("ui::{module}"))
                || replace_identifier(&source, type_name, "\u{0}") != source
            {
                found.push(path);
            }
        }
    }
    found.sort();
    found
}

/// Replaces `old` by `new` wherever it stands as a whole identifier.
///
/// Whole, and not as a substring: renaming `Home` to `Start` must not turn
/// `HomePage` into `StartPage`, and a project holds both often enough.
fn replace_identifier(source: &str, old: &str, new: &str) -> String {
    let boundary = |character: Option<char>| {
        !character.is_some_and(|character| character.is_alphanumeric() || character == '_')
    };

    let mut out = String::with_capacity(source.len());
    let mut rest = source;
    while let Some(index) = rest.find(old) {
        let before = rest[..index].chars().next_back();
        let after = rest[index + old.len()..].chars().next();
        out.push_str(&rest[..index]);
        if boundary(before) && boundary(after) {
            out.push_str(new);
        } else {
            out.push_str(old);
        }
        rest = &rest[index + old.len()..];
    }
    out.push_str(rest);
    out
}

/// The type a view file declares.
///
/// The one that implements `Render`, and its first `pub struct` failing that:
/// read rather than derived from the module name, because a view adopted from
/// a project maxx did not write is called whatever its author called it, and
/// `to_type_name` would answer a type that does not exist. A file may well
/// declare a helper struct above the view — `impl Render for` is what tells
/// them apart.
fn declared_type(source: &str) -> Option<String> {
    let rendered = source
        .lines()
        .find_map(|line| leading_identifier(line.trim_start().strip_prefix("impl Render for ")?));
    rendered.or_else(|| {
        source
            .lines()
            .find_map(|line| leading_identifier(line.trim_start().strip_prefix("pub struct ")?))
    })
}

/// The line that builds the view the window opens on, and the type it names.
///
/// One candidate is the answer. Several — a `main.rs` that builds a toolbar
/// before its root view — and only the entity handed to `Root::new` is the
/// entry: rewriting the first `::new(window, cx)` in the file would change a
/// line nobody opens a window with, and `maxx.toml` would then claim an entry
/// the code does not open. When that cannot be read either, nothing is
/// touched.
fn entry_site(lines: &[String]) -> Option<(usize, String)> {
    let mut candidates = lines
        .iter()
        .enumerate()
        .filter_map(|(index, line)| entry_type(line).map(|found| (index, found)));

    let first = candidates.next()?;
    let rest: Vec<(usize, String)> = candidates.collect();
    if rest.is_empty() {
        return Some(first);
    }

    let root_view = lines.iter().find_map(|line| root_argument(line))?;
    std::iter::once(first).chain(rest).find(|(index, _)| binds(&lines[*index], &root_view))
}

/// The name `Root::new` is handed as its view.
fn root_argument(line: &str) -> Option<String> {
    let (_, after) = line.split_once("Root::new(")?;
    let argument = after.split([',', ')']).next()?.trim();
    leading_identifier(argument)
}

/// Whether `line` is the `let` that binds `name`.
fn binds(line: &str, name: &str) -> bool {
    let Some(rest) = line.trim_start().strip_prefix("let ") else {
        return false;
    };
    let rest = rest.trim_start().strip_prefix("mut ").unwrap_or(rest);
    leading_identifier(rest).is_some_and(|bound| bound == name)
}

/// The type `line` builds the window's view from, if it does.
///
/// The whole path as written, `crate::ui::home::Home` as readily as `Home`:
/// what is replaced has to be replaced whole.
fn entry_type(line: &str) -> Option<String> {
    let (before, _) = line.split_once("::new(window, cx)")?;
    let name: String = before
        .chars()
        .rev()
        .take_while(|c| c.is_alphanumeric() || *c == '_' || *c == ':')
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect();
    let name = name.trim_start_matches(':').to_string();
    // `Self::new(window, cx)` inside an `impl` is not a view being opened, and
    // a bare `::new(` belongs to something else entirely.
    (!name.is_empty() && last_segment(&name) != "Self").then_some(name)
}

/// The last segment of a path, which is the type it names.
fn last_segment(path: &str) -> &str {
    path.rsplit("::").next().unwrap_or(path)
}

/// The identifier `text` starts with, if it starts with one.
///
/// A prefix, where [`identifier`] validates a whole name: what is read here is
/// the head of a line — `Home {`, `home::Home;` — and what follows it is not
/// the caller's business.
fn leading_identifier(text: &str) -> Option<String> {
    let name: String = text.chars().take_while(|c| c.is_alphanumeric() || *c == '_').collect();
    (!name.is_empty()).then_some(name)
}

/// Whether a braced `use crate::ui::…` already brings `module::type_name` in.
///
/// Braces and spaces are dropped before the search, so the three spellings a
/// developer or a `rustfmt` may leave — `{home::Home, second::Second}`,
/// `second::{Second}`, one name per line — all read the same.
fn imported_in_group(lines: &[String], module: &str, type_name: &str) -> bool {
    let needle = format!("{module}::{type_name}");
    lines
        .iter()
        .filter(|line| line.trim_start().starts_with("use crate::ui::") && line.contains('{'))
        .any(|line| {
            let flattened: String =
                line.chars().filter(|c| !c.is_whitespace() && *c != '{' && *c != '}').collect();
            flattened.contains(&needle)
        })
}

/// Whether `line` is the `use` that brings `type_name` in from `crate::ui`.
fn is_import_of(line: &str, type_name: &str) -> bool {
    let line = line.trim();
    line.starts_with("use crate::ui::")
        && line.ends_with(&format!("::{type_name};"))
        // `use crate::ui::home::Home;` and nothing braced: a grouped import
        // would need to be taken apart rather than replaced whole.
        && !line.contains('{')
}

/// The view template.
///
/// The root scrolls, and that is not a flourish: a window is 900 by 600, and a
/// view taller than that was cut with no way down — one image at its natural
/// size is enough. `id` is what gpui needs to keep a scroll offset between
/// frames, and `size_full` is what gives the view the window to fill.
///
/// The three calls are ordinary Rust that maxx carries as data: they show in
/// the inspector, and whoever does not want them removes them there.
pub(super) fn view_rs(type_name: &str, module: &str) -> String {
    format!(
        r#"use gpui::{{Context, Window, prelude::*}};
use gpui_component::label::Label;
use gpui_component::v_flex;

pub struct {type_name} {{}}

impl {type_name} {{
    pub fn new(_window: &mut Window, _cx: &mut Context<Self>) -> Self {{
        Self {{}}
    }}
}}

impl Render for {type_name} {{
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {{
        // maxx:begin
        v_flex()
            .id("{module}")
            .size_full()
            .overflow_y_scroll()
            .gap_2()
            .p_4()
            .child(Label::new("Welcome"))
        // maxx:end
    }}
}}
"#
    )
}
