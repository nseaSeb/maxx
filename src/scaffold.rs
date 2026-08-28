//! Project and view templates.
//!
//! Everything written here is ordinary Rust that compiles and runs without
//! `maxx`. The only trace `maxx` leaves is a pair of marker comments around the
//! expression it owns.

use rust_i18n::t;
use std::io;
use std::path::{Path, PathBuf};

pub mod templates;
pub use templates::{settings_screen_rs, shell_rs};

/// Where an image dropped on a view is kept.
///
/// Under `assets/`, because that is the name every framework gives it and the
/// one a developer opening the project will look for; under `images/`, because
/// fonts and icons will want their own place beside it.
pub const IMAGE_DIRECTORY: &str = "assets/images";

/// Copies `file` into the project and answers the path to write, relative to
/// the root, with forward slashes.
///
/// Relative and inside the project, or the image shows on one machine only:
/// `img(PathBuf::from(..))` is read from the directory the binary starts in,
/// which is the project root. Picking a file from the desktop and writing its
/// absolute path would draw on the canvas and nowhere else.
///
/// A file already inside the project is left exactly where it is — that is the
/// developer's layout, and moving it would break whatever else points at it.
///
/// A name already taken by a *different* file is numbered rather than
/// overwritten; the same file imported twice is recognised by its bytes and
/// imported once.
pub fn import_asset(root: &Path, file: &Path) -> Result<String, String> {
    if !crate::project::is_image(file) {
        return Err(crate::tr("error.not_an_image").to_string());
    }
    let extension =
        file.extension().and_then(|value| value.to_str()).unwrap_or_default().to_lowercase();

    let resolved = file.canonicalize().unwrap_or_else(|_| file.to_path_buf());
    // The same ceiling the reader holds, and before the shortcut below: a file
    // already in the project is not a file maxx can draw, and accepting one it
    // would then refuse to open is the two paths disagreeing about the same
    // picture.
    // Une taille qu'on ne sait pas lire n'est pas zéro : la traiter ainsi
    // laisserait passer la lecture entière que ce plafond existe pour éviter.
    let size = std::fs::metadata(&resolved).map_err(|error| error.to_string())?.len();
    if size > crate::project::MAX_IMAGE_BYTES {
        return Err(t!("error.file_too_large", size = size / 1024).into_owned());
    }

    let inside = root.canonicalize().unwrap_or_else(|_| root.to_path_buf());
    if let Ok(relative) = resolved.strip_prefix(&inside) {
        return Ok(slashed(relative));
    }

    let stem = file.file_stem().and_then(|value| value.to_str()).unwrap_or("image").to_string();
    let directory = root.join(IMAGE_DIRECTORY);
    std::fs::create_dir_all(&directory).map_err(|error| error.to_string())?;

    let bytes = std::fs::read(&resolved).map_err(|error| error.to_string())?;
    let mut name = format!("{stem}.{extension}");
    let mut index = 2;
    // A file that cannot be read is a file that differs, not a file to write
    // over: `is_ok_and` answered the same thing for "same bytes" and "could not
    // look", and the second case overwrote what it had not seen.
    while directory.join(&name).exists()
        && !std::fs::read(directory.join(&name)).is_ok_and(|existing| existing == bytes)
    {
        name = format!("{stem}-{index}.{extension}");
        index += 1;
    }

    std::fs::write(directory.join(&name), &bytes).map_err(|error| error.to_string())?;
    Ok(format!("{IMAGE_DIRECTORY}/{name}"))
}

/// A path written the one way `PathBuf` reads everywhere.
fn slashed(path: &Path) -> String {
    path.components()
        .map(|component| component.as_os_str().to_string_lossy().into_owned())
        .collect::<Vec<_>>()
        .join("/")
}

/// The shape a new project is given.
///
/// Not screens to pick from a gallery: the three answer the question a desktop
/// project asks on its first day, which is *what holds what*. A sidebar and a
/// settings screen arrive here rather than in the component palette for that
/// reason — they are not elements to drop on a canvas, they are what a canvas
/// hangs from.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum Template {
    /// One window, one view, and nothing else to unlearn.
    #[default]
    Empty,
    /// A sidebar on the left, the view of the moment on the right.
    Sidebar,
    /// The same, with the settings module and a screen that reads and writes
    /// it.
    Settings,
}

impl Template {
    /// The name this shape carries in `maxx.toml` and in the tests.
    pub fn name(self) -> &'static str {
        match self {
            Self::Empty => "empty",
            Self::Sidebar => "sidebar",
            Self::Settings => "settings",
        }
    }
}

/// Creates a runnable GPUI project at `root`, in the shape `template` asks for.
pub fn create_project(root: &Path, name: &str, template: Template) -> io::Result<()> {
    // Never write over an existing crate: `src/ui/mod.rs` and `src/main.rs`
    // would go with it.
    if root.join("Cargo.toml").exists() {
        return Err(io::Error::new(
            io::ErrorKind::AlreadyExists,
            format!("{} already holds a Cargo.toml", root.display()),
        ));
    }
    std::fs::create_dir_all(root.join("src/ui"))?;
    std::fs::create_dir_all(root.join(".cargo"))?;
    std::fs::write(root.join("Cargo.toml"), cargo_toml(&crate_name(name)))?;
    std::fs::write(root.join(".cargo/config.toml"), cargo_config())?;
    // `maxx.toml` carries the project, so it starts with the project: the view
    // the window opens on. What was copied from maxx joins it later, module by
    // module. Versioned with the project, both halves.
    std::fs::write(root.join(".gitignore"), "/target\n/.cargo\n")?;
    // A shape with a shell draws its own title bar; the empty one keeps the
    // system's, having nothing to put in a bar of its own.
    std::fs::write(root.join("src/main.rs"), main_rs(template != Template::Empty))?;
    std::fs::write(root.join("src/menus.rs"), menus_rs())?;
    std::fs::write(root.join("src/ui/mod.rs"), "pub mod home;\n")?;
    std::fs::write(root.join("src/ui/home.rs"), view_rs("Home", "home"))?;
    crate::projectfile::set_entry(root, "home")?;

    match template {
        Template::Empty => Ok(()),
        Template::Sidebar => add_shell(root, &[("library", "Library", "Library")]),
        Template::Settings => {
            // The module first: the screen is written against it, and a screen
            // whose module failed to arrive would not compile.
            add_settings_module(root)?;
            std::fs::write(
                root.join("src/ui/settings_screen.rs"),
                crate::scaffold::settings_screen_rs(),
            )?;
            declare_ui_module(root, "settings_screen")?;
            add_shell(root, &[("settings_screen", "SettingsScreen", "Settings")])
        }
    }
}

/// Gives the project a shell: a sidebar, `home`, and whatever `pages` adds.
///
/// The window then opens on the shell rather than on a view — which is a fact
/// about the project, so `set_entry_view` records it in `maxx.toml` on the way
/// through.
fn add_shell(root: &Path, pages: &[(&str, &str, &str)]) -> io::Result<()> {
    for (module, type_name, _) in pages {
        // A page maxx designs is created as a view; one it wrote whole, like
        // the settings screen, is already on disk.
        if !root.join(format!("src/ui/{module}.rs")).exists() {
            std::fs::write(root.join(format!("src/ui/{module}.rs")), view_rs(type_name, module))?;
            declare_ui_module(root, module)?;
        }
    }

    let mut all = vec![("home", "Home", "Home")];
    all.extend_from_slice(pages);
    std::fs::write(root.join("src/ui/shell.rs"), crate::scaffold::shell_rs(&all))?;
    declare_ui_module(root, "shell")?;
    set_entry_view(root, "shell")
}

/// Declares `module` in `src/ui/mod.rs`, if it is not there already.
///
/// By textual insertion so the rest of the file — comments, ordering, anything
/// the developer put there — is untouched.
fn declare_ui_module(root: &Path, module: &str) -> io::Result<()> {
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

/// Turns a folder name into a name cargo accepts: lowercase, `_` for anything
/// that is not alphanumeric, and never starting with a digit.
pub fn crate_name(name: &str) -> String {
    let mut out: String =
        name.chars()
            .map(|character| {
                if character.is_ascii_alphanumeric() { character.to_ascii_lowercase() } else { '_' }
            })
            .collect();
    if out.is_empty() || out.starts_with(|character: char| character.is_ascii_digit()) {
        out.insert(0, '_');
    }
    out
}

/// `home` becomes `Home`, `my_screen` becomes `MyScreen`.
pub fn to_type_name(module: &str) -> String {
    module
        .split('_')
        .filter(|part| !part.is_empty())
        .map(|part| {
            let mut chars = part.chars();
            match chars.next() {
                Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
                None => String::new(),
            }
        })
        .collect()
}

fn cargo_toml(name: &str) -> String {
    format!(
        r#"[package]
name = "{name}"
version = "0.1.0"
edition = "2024"

[dependencies]
# `runtime_shaders` compiles the Metal shaders at startup instead of at build
# time. Xcode 26 ships the Metal toolchain as a separate downloadable
# component, and without this feature the build fails on a missing `metal`
# tool. Remove it only once that component is installed.
gpui = {{ version = "0.2.2", features = ["runtime_shaders"] }}
gpui-component = "0.5.1"

[profile.dev.package."*"]
opt-level = 2
"#
    )
}

/// Points the project at the cache every maxx project shares.
///
/// The path is absolute, so it is machine-local — hence the `.gitignore` entry.
/// Losing it costs a rebuild, nothing more.
fn cargo_config() -> String {
    format!(
        r#"# Written by maxx. Every maxx project builds into the same directory: gpui
# and gpui-component are about 750 crates, and a project with a `target/` of
# its own rebuilds all of them. This file is local to this machine, hence its
# entry in .gitignore.
[build]
target-dir = "{}"
"#,
        // A basic TOML string treats `\` as an escape, and `C:\Users\…` holds
        // no valid one: the file becomes unreadable and `cargo` refuses to
        // start before it even compiles.
        crate::run::shared_target_dir()
            .display()
            .to_string()
            .replace('\\', "\\\\")
            .replace('"', "\\\"")
    )
}

/// The modules maxx knows how to copy into a project, and their versions.
///
/// A version is bumped whenever the template changes. `tests/scaffold.rs`
/// holds the fingerprint of each one and fails when a template moves without
/// its version following — the guard against a fix that never reaches the
/// projects carrying the old copy.
pub const MODULES: &[(&str, u32)] =
    &[("system", 1), ("settings", 1), ("theme", 1), ("assets", 1), ("window", 1)];

/// The name each module carried before it was renamed to English.
///
/// A project written by an older maxx has `src/systeme.rs` and `mod systeme;`,
/// which the new names do not match: adding the module again would write a
/// second, near-identical file and declare it alongside the first, leaving the
/// developer to guess which one their code calls.
const LEGACY: &[(&str, &str)] = &[("system", "systeme"), ("settings", "reglages")];

/// The error to answer when `module` is already in the project under its old
/// name.
fn legacy_copy(root: &Path, module: &str) -> Option<io::Error> {
    let (_, old) = LEGACY.iter().find(|(current, _)| *current == module)?;
    root.join(format!("src/{old}.rs")).exists().then(|| {
        io::Error::new(
            io::ErrorKind::AlreadyExists,
            format!(
                "src/{old}.rs is this module under its former name — rename it to \
                 src/{module}.rs, and its `mod {old};` with it, before adding it again"
            ),
        )
    })
}

/// The version of `module`, if maxx knows it.
pub fn module_version(module: &str) -> Option<u32> {
    MODULES.iter().find(|(name, _)| *name == module).map(|(_, version)| *version)
}

/// The current text of a module's template.
pub fn module_body(module: &str) -> Option<String> {
    match module {
        "system" => Some(system_rs()),
        "settings" => Some(settings_rs()),
        "theme" => Some(theme_rs()),
        "assets" => Some(assets_rs()),
        "window" => Some(window_rs()),
        _ => None,
    }
}

/// The modules a project carries in a version older than maxx's, and that it
/// has not modified since.
///
/// A file the developer has touched is never listed: it is theirs now, and
/// maxx has no business replacing it.
pub fn outdated_modules(root: &Path) -> Vec<String> {
    let file = crate::projectfile::load(root);
    let mut outdated = Vec::new();

    for (module, current) in MODULES {
        let Some(recorded) = file.modules.get(*module) else {
            continue;
        };
        if recorded.version >= *current {
            continue;
        }
        let path = root.join(format!("src/{module}.rs"));
        let Ok(body) = std::fs::read_to_string(&path) else {
            continue;
        };
        if recorded.holds(&body) {
            outdated.push((*module).to_string());
        }
    }
    outdated
}

/// Replaces a module with maxx's current version.
///
/// Refuses when the file no longer matches what maxx wrote: the developer's
/// edits are not maxx's to discard.
pub fn update_module(root: &Path, module: &str) -> io::Result<()> {
    let (Some(version), Some(body)) = (module_version(module), module_body(module)) else {
        return Err(io::Error::new(io::ErrorKind::NotFound, format!("unknown module: {module}")));
    };
    let file = crate::projectfile::load(root);
    let Some(recorded) = file.modules.get(module) else {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            format!("maxx.toml does not mention {module}"),
        ));
    };

    let path = root.join(format!("src/{module}.rs"));
    let current = std::fs::read_to_string(&path)?;
    if !recorded.holds(&current) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("src/{module}.rs has been modified — maxx does not replace it"),
        ));
    }

    std::fs::write(&path, &body)?;
    // The build script is the other half of the assets module, and `maxx.toml`
    // does not track it: a new `assets.rs` on top of a stale `build.rs` is a
    // project that stops compiling. Rewritten only while it is still maxx's
    // own — a script the developer has taken over is theirs.
    if module == "assets" {
        let build_path = root.join("build.rs");
        if std::fs::read_to_string(&build_path).is_ok_and(|body| body.contains(BUILD_MARKER)) {
            std::fs::write(&build_path, assets_build_rs())?;
        }
    }
    crate::projectfile::record(root, module, version, &body)
}

/// Adds the system module to an existing project and declares it.
///
/// A copied module, not a dependency: a generated project owes nothing to
/// maxx, and this one owes nothing to gpui either — it is plain `std`.
pub fn add_system_module(root: &Path) -> io::Result<()> {
    if let Some(error) = legacy_copy(root, "system") {
        return Err(error);
    }
    let main_path = root.join("src/main.rs");
    let source = std::fs::read_to_string(&main_path)?;
    let mut lines: Vec<String> = source.lines().map(str::to_string).collect();

    if !lines.iter().any(|line| line.trim() == "mod system;") {
        lines.insert(header_end(&lines), "mod system;".into());
    }

    let path = root.join("src/system.rs");
    let body = system_rs();
    if !path.exists() {
        std::fs::write(&path, &body)?;
        crate::projectfile::record(root, "system", module_version("system").unwrap_or(1), &body)?;
    }

    std::fs::write(&main_path, joined(&lines, &source))
}

/// Drops `mod <module>;` from `src/main.rs`.
///
/// Called when a module file goes to the Trash: leaving the declaration behind
/// stops the project from compiling, which is the opposite of what deleting a
/// file is meant to achieve.
pub fn remove_module(root: &Path, module: &str) -> io::Result<()> {
    let main_path = root.join("src/main.rs");
    let source = std::fs::read_to_string(&main_path)?;
    let declaration = format!("mod {module};");
    let wiring = WIRING.iter().find(|(name, _, _)| *name == module);
    let (statements, fragments) = wiring.map_or((&[][..], &[][..]), |(_, s, f)| (*s, *f));

    let mut changed = false;
    let mut kept: Vec<String> = Vec::new();
    for line in source.lines() {
        let trimmed = line.trim();
        if trimmed == declaration
            || trimmed == format!("pub {declaration}")
            || statements.iter().any(|call| trimmed.contains(call))
        {
            changed = true;
            continue;
        }
        let mut line = line.to_string();
        let mut cut = false;
        for fragment in fragments {
            if line.contains(fragment) {
                line = line.replace(fragment, "");
                changed = true;
                cut = true;
            }
        }
        // A fragment rustfmt had given a line of its own leaves an empty one
        // behind, in the middle of a call chain.
        if cut && line.trim().is_empty() {
            continue;
        }
        kept.push(line);
    }

    // Deleting a file that was never declared must not rewrite `main.rs` at
    // all: `lines()` and `join` would quietly turn CRLF into LF, which is a
    // whole-file diff for a change that did not happen.
    if !changed {
        return Ok(());
    }

    std::fs::write(&main_path, joined(&kept, &source))
}

/// What maxx wrote into `main.rs` to reach a module: the whole statements that
/// go when the module goes, and the fragments that are only cut out of the line
/// carrying them.
///
/// Whole lines wherever the wiring allows it, and that is why it has the shape
/// it has: a call written as an argument to another one would leave a hole where
/// a value is expected. Deleting a module has to leave a file that still
/// compiles — `system`, `settings` and `theme` are only declared, these two are
/// called.
const WIRING: &[(&str, &[&str], &[&str])] =
    &[("assets", &[], &[WITH_ASSETS]), ("window", &["window::bounds(", "window::remember("], &[])];

/// `lines` joined the way `source` ends its own.
///
/// `str::lines` drops the `\r` of a CRLF file, so joining with `\n` would turn
/// every line of a `main.rs` into a change nobody asked for — a whole-file diff
/// for two inserted lines, and now on an ordinary view save, since the assets
/// module adds itself.
fn joined(lines: &[String], source: &str) -> String {
    let ending = if source.contains("\r\n") { "\r\n" } else { "\n" };
    let mut out = lines.join(ending);
    out.push_str(ending);
    out
}

/// The first line an item may be inserted before.
///
/// An inner doc comment or an inner attribute has to stay ahead of every item,
/// or the crate stops compiling.
fn header_end(lines: &[String]) -> usize {
    let mut index = 0;
    let mut in_block = false;
    while index < lines.len() {
        let line = lines[index].trim_start();
        if in_block {
            if line.contains("*/") {
                in_block = false;
            }
            index += 1;
            continue;
        }
        // `/*! … */` is an inner doc comment too, and just as fatal to jump
        // over: an item may not precede one.
        if line.starts_with("/*") {
            in_block = !line.contains("*/");
            index += 1;
            continue;
        }
        if line.is_empty() || line.starts_with("//") || line.starts_with("#![") {
            index += 1;
            continue;
        }
        break;
    }
    index
}

/// The system module of a generated project.
///
/// Only what actually differs from one system to the next *and* is not already
/// in gpui. The clipboard, opening a URL, revealing a file, the file pickers:
/// gpui has all of them (`cx.write_to_clipboard`, `cx.open_url`,
/// `cx.reveal_path`, `cx.open_with_system`, `cx.prompt_for_paths`), and
/// wrapping them would be noise. What is left is where a system puts an
/// application's files, and what it calls its trash.
fn system_rs() -> String {
    r#"//! What every system does its own way.
//!
//! Written by maxx, yours from here. Nothing in it depends on maxx or on gpui:
//! it is plain `std`, and copies elsewhere as it stands.
//!
//! This module deliberately leaves out the clipboard, opening a URL, revealing
//! a file in the file manager and the file pickers: gpui already has them —
//! `cx.write_to_clipboard`, `cx.read_from_clipboard`, `cx.open_url`,
//! `cx.reveal_path`, `cx.open_with_system`, `cx.prompt_for_paths`. Wrapping
//! them would be noise and nothing else.

// A module added before it is needed: without this, every function not yet
// called raises a warning, and seven warnings on the day you add it teach you
// to stop reading them. Drop it once everything is in use.
#![allow(dead_code)]

use std::path::{Path, PathBuf};

/// Where the settings live: what the user edits.
///
/// `XDG_CONFIG_HOME` when it is set, `APPDATA` on Windows,
/// `Library/Application Support` on macOS, `.config` elsewhere.
pub fn config_dir(application: &str) -> Option<PathBuf> {
    if cfg!(target_os = "windows") {
        return Some(PathBuf::from(std::env::var("APPDATA").ok()?).join(application));
    }
    if let Ok(xdg) = std::env::var("XDG_CONFIG_HOME") {
        if !xdg.is_empty() {
            return Some(PathBuf::from(xdg).join(application));
        }
    }
    let home = PathBuf::from(std::env::var("HOME").ok()?);
    Some(if cfg!(target_os = "macos") {
        home.join("Library/Application Support").join(application)
    } else {
        home.join(".config").join(application)
    })
}

/// Where to put what the application remembers on its own.
pub fn data_dir(application: &str) -> Option<PathBuf> {
    if cfg!(target_os = "windows") {
        return Some(PathBuf::from(std::env::var("LOCALAPPDATA").ok()?).join(application));
    }
    if let Ok(xdg) = std::env::var("XDG_DATA_HOME") {
        if !xdg.is_empty() {
            return Some(PathBuf::from(xdg).join(application));
        }
    }
    let home = PathBuf::from(std::env::var("HOME").ok()?);
    Some(if cfg!(target_os = "macos") {
        home.join("Library/Application Support").join(application)
    } else {
        home.join(".local/share").join(application)
    })
}

/// Where to put what can be rebuilt.
pub fn cache_dir(application: &str) -> Option<PathBuf> {
    if cfg!(target_os = "windows") {
        return Some(PathBuf::from(std::env::var("LOCALAPPDATA").ok()?).join(application));
    }
    if let Ok(xdg) = std::env::var("XDG_CACHE_HOME") {
        if !xdg.is_empty() {
            return Some(PathBuf::from(xdg).join(application));
        }
    }
    let home = PathBuf::from(std::env::var("HOME").ok()?);
    Some(if cfg!(target_os = "macos") {
        home.join("Library/Caches").join(application)
    } else {
        home.join(".cache").join(application)
    })
}

/// Writes `body` to `path`, through a temporary file.
///
/// A direct write interrupted halfway leaves a truncated file, which the next
/// read will take for a corrupt one.
pub fn write_atomically(path: &Path, body: &str) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    // The name, not the extension: `settings.json` and `settings.toml` would
    // otherwise both write to `settings.tmp` and overwrite each other.
    let name = path.file_name().unwrap_or_default().to_string_lossy();
    let temporary = path.with_file_name(format!("{name}.tmp"));
    std::fs::write(&temporary, body)?;
    if let Err(error) = std::fs::rename(&temporary, path) {
        let _ = std::fs::remove_file(&temporary);
        return Err(error);
    }
    Ok(())
}

/// Moves `path` to the trash and answers where it landed.
///
/// Never a deletion: an unlucky click should cost a round trip through the file
/// manager, not the day.
///
/// Three different trashes. `~/.Trash` on macOS. On Linux the freedesktop
/// specification, `.trashinfo` included — without it the desktop does not know
/// where the file came from and cannot restore it. On Windows, a trash of the
/// application's own: the real one is only reachable through the shell API,
/// which would cost a dependency and a block of `unsafe`.
pub fn move_to_trash(path: &Path, application: &str) -> Result<PathBuf, String> {
    let trash = trash_dir(application)?;
    std::fs::create_dir_all(&trash).map_err(|error| error.to_string())?;

    let name = path
        .file_name()
        .ok_or_else(|| String::from("path with no file name"))?
        .to_string_lossy()
        .into_owned();
    let (stem, extension) = match name.rsplit_once('.') {
        Some((stem, extension)) if !stem.is_empty() => (stem.to_string(), format!(".{extension}")),
        _ => (name.clone(), String::new()),
    };

    // The trash may already hold a file by that name.
    let mut target = trash.join(&name);
    let mut index = 1;
    while target.exists() {
        target = trash.join(format!("{stem} {index}{extension}"));
        index += 1;
    }

    // Across volumes `rename` fails with EXDEV: copy, then delete. In Rust and
    // not through `mv` or `cmd /C move`: `move` refuses to carry a folder from
    // one disk to another, which is exactly the case that leads here on
    // Windows.
    if std::fs::rename(path, &target).is_err() {
        copy_all(path, &target).map_err(|error| error.to_string())?;
        // Only once the copy is complete: deleting first would turn a failed
        // copy into a deletion.
        let removed = if path.is_dir() {
            std::fs::remove_dir_all(path)
        } else {
            std::fs::remove_file(path)
        };
        removed.map_err(|error| error.to_string())?;
    }

    write_trashinfo(&target, path);
    Ok(target)
}

/// Copies a file, or a folder and everything in it.
fn copy_all(source: &Path, target: &Path) -> std::io::Result<()> {
    if !source.is_dir() {
        std::fs::copy(source, target)?;
        return Ok(());
    }
    std::fs::create_dir_all(target)?;
    for entry in std::fs::read_dir(source)? {
        let entry = entry?;
        copy_all(&entry.path(), &target.join(entry.file_name()))?;
    }
    Ok(())
}

/// The folder this system keeps trashed files in.
fn trash_dir(application: &str) -> Result<PathBuf, String> {
    if cfg!(target_os = "macos") {
        let home = std::env::var("HOME").map_err(|_| String::from("HOME is not set"))?;
        return Ok(PathBuf::from(home).join(".Trash"));
    }
    if cfg!(target_os = "windows") {
        return data_dir(application)
            .map(|dir| dir.join("trash"))
            .ok_or_else(|| String::from("LOCALAPPDATA is not set"));
    }
    let data = match std::env::var("XDG_DATA_HOME") {
        Ok(data) if !data.is_empty() => PathBuf::from(data),
        _ => {
            let home = std::env::var("HOME").map_err(|_| String::from("HOME is not set"))?;
            PathBuf::from(home).join(".local/share")
        }
    };
    Ok(data.join("Trash/files"))
}

/// Writes the record a Linux desktop needs to offer “Restore”.
fn write_trashinfo(target: &Path, original: &Path) {
    if cfg!(target_os = "macos") || cfg!(target_os = "windows") {
        return;
    }
    let Some(files) = target.parent() else { return };
    let Some(trash) = files.parent() else { return };
    let Some(name) = target.file_name() else { return };

    let info = trash.join("info");
    if std::fs::create_dir_all(&info).is_err() {
        return;
    }
    let absolute = std::fs::canonicalize(original.parent().unwrap_or(original))
        .map(|parent| match original.file_name() {
            Some(name) => parent.join(name),
            None => parent,
        })
        .unwrap_or_else(|_| original.to_path_buf());

    // The specification asks for both keys, and for an encoded path: a file
    // named `100%.rs` would otherwise be decoded wrong and restored somewhere
    // else, or nowhere.
    let body = format!(
        "[Trash Info]\nPath={}\nDeletionDate={}\n",
        encode(&absolute.to_string_lossy()),
        deletion_date()
    );
    let _ = std::fs::write(
        info.join(format!("{}.trashinfo", name.to_string_lossy())),
        body,
    );
}

/// Encodes a path the way the trash specification asks for.
fn encode(path: &str) -> String {
    let mut out = String::with_capacity(path.len());
    for byte in path.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' | b'/' => {
                out.push(byte as char)
            }
            _ => out.push_str(&format!("%{byte:02X}")),
        }
    }
    out
}

/// The moment of the deletion, in the expected shape.
///
/// In UTC where the specification asks for local time: `std` has no time zone,
/// and taking a dependency for one line of a file nobody reads by hand would be
/// paying dearly.
fn deletion_date() -> String {
    let seconds = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|elapsed| elapsed.as_secs())
        .unwrap_or(0);

    let days = (seconds / 86_400) as i64;
    let time = seconds % 86_400;
    let (year, month, day) = civil_date(days);
    format!(
        "{year:04}-{month:02}-{day:02}T{:02}:{:02}:{:02}",
        time / 3600,
        (time % 3600) / 60,
        time % 60
    )
}

/// Turns a number of days since 1970-01-01 into a civil date.
///
/// Howard Hinnant's algorithm: it shifts the year to start in March, which puts
/// the leap day at the end and saves making a case of it.
fn civil_date(days: i64) -> (i64, u32, u32) {
    let shifted = days + 719_468;
    let era = if shifted >= 0 { shifted } else { shifted - 146_096 } / 146_097;
    let day_of_era = shifted - era * 146_097;
    let year_of_era =
        (day_of_era - day_of_era / 1460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let year = year_of_era + era * 400;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let shifted_month = (5 * day_of_year + 2) / 153;
    let day = (day_of_year - (153 * shifted_month + 2) / 5 + 1) as u32;
    let month = if shifted_month < 10 {
        shifted_month + 3
    } else {
        shifted_month - 9
    } as u32;
    (if month <= 2 { year + 1 } else { year }, month, day)
}
"#
    .to_string()
}

/// Adds the settings module to an existing project, with what it needs.
///
/// Pulls the system module in with it: the settings need to know where this
/// system puts an application's files, and that is exactly what `system.rs`
/// answers. And declares `serde` and `serde_json_lenient`, both already
/// compiled in the tree through gpui, so the build does not grow.
pub fn add_settings_module(root: &Path) -> io::Result<()> {
    if let Some(error) = legacy_copy(root, "settings") {
        return Err(error);
    }
    add_system_module(root)?;
    add_dependencies(
        root,
        &[
            ("serde", "{ version = \"1\", features = [\"derive\"] }"),
            ("serde_json_lenient", "\"0.2\""),
        ],
    )?;

    let main_path = root.join("src/main.rs");
    let source = std::fs::read_to_string(&main_path)?;
    let mut lines: Vec<String> = source.lines().map(str::to_string).collect();
    if !lines.iter().any(|line| line.trim() == "mod settings;") {
        lines.insert(header_end(&lines), "mod settings;".into());
    }

    let path = root.join("src/settings.rs");
    let body = settings_rs();
    if !path.exists() {
        std::fs::write(&path, &body)?;
        crate::projectfile::record(
            root,
            "settings",
            module_version("settings").unwrap_or(1),
            &body,
        )?;
    }

    std::fs::write(&main_path, joined(&lines, &source))
}

/// Declares crates in the project's `Cargo.toml`, under `[dependencies]`.
///
/// Textual, like everything else maxx adds: the file is the developer's, and
/// rewriting it from a template would throw away whatever they put in it.
fn add_dependencies(root: &Path, crates: &[(&str, &str)]) -> io::Result<()> {
    let path = root.join("Cargo.toml");
    let source = std::fs::read_to_string(&path)?;
    let mut lines: Vec<String> = source.lines().map(str::to_string).collect();

    let section = dependencies_section(&source)?;
    // The end of the section, not the end of the file: a `[profile]` block
    // after it must stay after it. And before the blank line that separates the
    // two — inserting after it glues the new crates to the next header and
    // leaves the gap in the middle of the list.
    let mut end = lines[section + 1..]
        .iter()
        .position(|line| line.trim_start().starts_with('['))
        .map(|offset| section + 1 + offset)
        .unwrap_or(lines.len());
    while end > section + 1 && lines[end - 1].trim().is_empty() {
        end -= 1;
    }

    let mut inserted = 0;
    for (name, requirement) in crates {
        let declared = lines[section + 1..end]
            .iter()
            .any(|line| line.split('=').next().is_some_and(|left| left.trim() == *name));
        if declared {
            continue;
        }
        lines.insert(end + inserted, format!("{name} = {requirement}"));
        inserted += 1;
    }
    if inserted == 0 {
        return Ok(());
    }

    let ending = if source.contains("\r\n") { "\r\n" } else { "\n" };
    let mut out = lines.join(ending);
    out.push_str(ending);
    std::fs::write(&path, out)
}

/// The line `[dependencies]` sits on, or the refusal to say so.
///
/// Looked up on its own so a caller can ask before it writes anything: a module
/// that declares crates has to know it can, or it leaves the project with half
/// of itself added.
fn dependencies_section(source: &str) -> io::Result<usize> {
    source.lines().position(|line| line.trim() == "[dependencies]").ok_or_else(|| {
        io::Error::new(io::ErrorKind::InvalidData, "Cargo.toml: no [dependencies] section")
    })
}

/// The settings module of a generated project.
///
/// The same discipline maxx applies to its own: JSON with comments, only the
/// changed key rewritten, a documented default file. It is a copy, and a copy
/// is a debt — a defect found on one side has to be carried to the other.
fn settings_rs() -> String {
    r##"//! The application's settings: what it remembers from one run to the next.
//!
//! Written by maxx, yours from here. Add your fields to `Settings`, a line to
//! `documented_defaults`, and that is all.
//!
//! JSON with comments, because a file the user is invited to open has to accept
//! being annotated — and **only the key that changes is rewritten**. Your
//! comments and your layout survive a save, which serialising the whole struct
//! never allows.
//!
//! The reading principle: a file that is missing, partial or damaged is never
//! worse than no file at all. `serde(default)` drops a missing key back on its
//! default instead of failing the whole read, and an unreadable file is
//! reported and then left alone — overwriting it would lose whatever the user
//! was in the middle of writing.

#![allow(dead_code)]

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

/// What the application remembers. Add your fields here.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    /// Example: replace it with your own.
    pub dark_theme: bool,
    /// Example: replace it with your own.
    pub text_size: f32,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            dark_theme: true,
            text_size: 14.0,
        }
    }
}

/// The application's folder name, under the configuration directory.
const APPLICATION: &str = env!("CARGO_PKG_NAME");

/// Where the file lives.
pub fn path() -> Option<PathBuf> {
    crate::system::config_dir(APPLICATION).map(|folder| folder.join("settings.json"))
}

/// Reads the settings, tolerating comments and trailing commas.
pub fn load() -> Settings {
    let Some(path) = path() else {
        return Settings::default();
    };
    let Ok(source) = std::fs::read_to_string(&path) else {
        return Settings::default();
    };
    match serde_json_lenient::from_str_lenient(&source) {
        Ok(settings) => settings,
        Err(error) => {
            eprintln!("{} is unreadable: {error}", path.display());
            Settings::default()
        }
    }
}

/// Writes the settings, touching only the keys whose value has changed.
pub fn save(settings: &Settings) -> std::io::Result<()> {
    let Some(path) = path() else {
        return Ok(());
    };
    let source = match std::fs::read_to_string(&path) {
        Ok(source) if source.contains('{') => source,
        Ok(_) => documented_defaults(),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => documented_defaults(),
        // Unreadable for another reason — a permission, an incident: writing
        // would replace what the user wrote with the defaults, which is the one
        // outcome worse than not saving at all.
        Err(error) => return Err(error),
    };
    crate::system::write_atomically(&path, &patch(&source, settings))
}

/// The file written when there is none: every key, its default, and a line
/// saying what it does. The file is its own documentation.
pub fn documented_defaults() -> String {
    let defaults = Settings::default();
    format!(
        r#"// Settings for {APPLICATION}.
//
// This file is yours. The application only rewrites the key it changes: your
// comments and your layout stay where they are. Comments and trailing commas
// are accepted when reading.
{{
  // Example: replace it with your own.
  "dark_theme": {},

  // Example: replace it with your own.
  "text_size": {}
}}
"#,
        defaults.dark_theme, defaults.text_size
    )
}

/// Writes every key of `settings` into `source`, changing as few bytes as
/// possible.
pub fn patch(source: &str, settings: &Settings) -> String {
    let Ok(serde_json_lenient::Value::Object(values)) = serde_json_lenient::to_value(settings)
    else {
        return source.to_string();
    };

    let mut out = source.to_string();
    for (key, value) in values {
        let rendered = value.to_string();
        out = match replace(&out, &key, &rendered) {
            Some(patched) => patched,
            None => insert(&out, &key, &rendered),
        };
    }
    out
}

/// Replaces the value of a top-level key, keeping everything else.
///
/// Answers `None` when the key is not there, which tells the caller to add it.
fn replace(source: &str, key: &str, value: &str) -> Option<String> {
    let (member, _) = walk(source, key);
    let member = member?;
    Some(format!(
        "{}{value}{}",
        &source[..member.start],
        &source[member.end..]
    ))
}

/// Adds a key just after the opening brace.
///
/// After the opening brace and not before the closing one: the last thing in an
/// object is very often a comment, and a comma added there would end up
/// commented out — two members with no separator, an invalid file.
fn insert(source: &str, key: &str, value: &str) -> String {
    let (_, position) = walk(source, key);
    let Some(position) = position else {
        return format!("{{\n  \"{key}\": {value}\n}}\n");
    };

    let bytes = source.as_bytes();
    let next = skip_blanks(bytes, position);
    let empty = next >= bytes.len() || bytes[next] == b'}';
    let separator = if empty { "" } else { "," };

    format!(
        "{}\n  \"{key}\": {value}{separator}{}",
        &source[..position],
        &source[position..]
    )
}

/// Walks the members of the top-level object.
///
/// Answers the span of `key`'s value if it is there, and the place where a new
/// key can be inserted.
fn walk(source: &str, key: &str) -> (Option<std::ops::Range<usize>>, Option<usize>) {
    let bytes = source.as_bytes();
    // Not `find('{')`: the file opens with a block of comments, and a brace
    // written in there would anchor the whole walk inside the comment.
    let brace = skip_blanks(bytes, 0);
    if brace >= bytes.len() || bytes[brace] != b'{' {
        return (None, None);
    }
    let after = brace + 1;

    let expected = format!("\"{key}\"");
    let mut index = skip_blanks(bytes, after);
    while index < bytes.len() && bytes[index] != b'}' {
        if bytes[index] != b'"' {
            break;
        }
        let name_end = end_of_string(bytes, index);
        let found = source[index..name_end] == expected;

        let colon = skip_blanks(bytes, name_end);
        if colon >= bytes.len() || bytes[colon] != b':' {
            break;
        }
        let start = skip_blanks(bytes, colon + 1);
        let end = end_of_value(bytes, start);
        if found {
            return (Some(start..end), Some(after));
        }

        index = skip_blanks(bytes, end);
        if index < bytes.len() && bytes[index] == b',' {
            index = skip_blanks(bytes, index + 1);
        }
    }

    (None, Some(after))
}

/// Skips whitespace and comments.
///
/// A comment is not JSON, and the walk has to step over it without reading a
/// brace, a quote or a colon written inside as structure. A single odd quote in
/// a comment would otherwise leave the scan “inside a string” to the end of the
/// file.
fn skip_blanks(bytes: &[u8], mut index: usize) -> usize {
    loop {
        while index < bytes.len() && bytes[index].is_ascii_whitespace() {
            index += 1;
        }
        if bytes[index..].starts_with(b"//") {
            while index < bytes.len() && bytes[index] != b'\n' {
                index += 1;
            }
            continue;
        }
        if bytes[index..].starts_with(b"/*") {
            index += 2;
            while index + 1 < bytes.len() && !(bytes[index] == b'*' && bytes[index + 1] == b'/') {
                index += 1;
            }
            index = (index + 2).min(bytes.len());
            continue;
        }
        return index;
    }
}

/// The index just after the string whose opening quote is at `index`.
fn end_of_string(bytes: &[u8], index: usize) -> usize {
    let mut index = index + 1;
    while index < bytes.len() {
        match bytes[index] {
            b'\\' => index += 2,
            b'"' => return index + 1,
            _ => index += 1,
        }
    }
    bytes.len()
}

/// The index just after the JSON value starting at `index`.
fn end_of_value(bytes: &[u8], index: usize) -> usize {
    if index >= bytes.len() {
        return bytes.len();
    }
    match bytes[index] {
        b'"' => end_of_string(bytes, index),
        opening @ (b'[' | b'{') => {
            let closing = if opening == b'[' { b']' } else { b'}' };
            let mut depth = 0usize;
            let mut index = index;
            while index < bytes.len() {
                index = skip_blanks(bytes, index);
                if index >= bytes.len() {
                    break;
                }
                match bytes[index] {
                    b'"' => {
                        index = end_of_string(bytes, index);
                        continue;
                    }
                    byte if byte == opening => depth += 1,
                    byte if byte == closing => {
                        depth -= 1;
                        if depth == 0 {
                            return index + 1;
                        }
                    }
                    _ => {}
                }
                index += 1;
            }
            bytes.len()
        }
        // A number, `true`, `false` or `null`: the value stops at the first
        // character that cannot be part of it — a separator, whitespace, or the
        // start of a comment.
        _ => {
            let mut index = index;
            while index < bytes.len() {
                let byte = bytes[index];
                if byte.is_ascii_whitespace()
                    || matches!(byte, b',' | b'}' | b']')
                    || bytes[index..].starts_with(b"//")
                    || bytes[index..].starts_with(b"/*")
                {
                    break;
                }
                index += 1;
            }
            index
        }
    }
}

/// The file's path, as it stands: handy to show in a settings screen, or to
/// open in the user's editor.
pub fn displayable_path() -> String {
    path()
        .map(|path| path.display().to_string())
        .unwrap_or_else(|| "no location for it on this system".into())
}

/// For tests: reads a given file rather than the application's own.
pub fn load_from(path: &Path) -> Settings {
    let Ok(source) = std::fs::read_to_string(path) else {
        return Settings::default();
    };
    serde_json_lenient::from_str_lenient(&source).unwrap_or_default()
}
"##
    .to_string()
}

/// Adds the palette module to an existing project and declares it.
///
/// The only copied module that leans on `gpui_component`: the mode lives in its
/// theme, and keeping a second one beside it would let the two disagree — the
/// window in one mode and its buttons in the other.
pub fn add_theme_module(root: &Path) -> io::Result<()> {
    if let Some(error) = legacy_copy(root, "theme") {
        return Err(error);
    }
    let main_path = root.join("src/main.rs");
    let source = std::fs::read_to_string(&main_path)?;
    let mut lines: Vec<String> = source.lines().map(str::to_string).collect();
    if !lines.iter().any(|line| line.trim() == "mod theme;") {
        lines.insert(header_end(&lines), "mod theme;".into());
    }

    let path = root.join("src/theme.rs");
    let body = theme_rs();
    if !path.exists() {
        std::fs::write(&path, &body)?;
        crate::projectfile::record(root, "theme", module_version("theme").unwrap_or(1), &body)?;
    }

    std::fs::write(&main_path, joined(&lines, &source))
}

/// The palette of a generated project.
///
/// Roles rather than colours, and two values for each: `bg` says where it goes
/// and survives a change of palette, `0x1e2127` says what it looks like in one
/// of the two modes and does not.
fn theme_rs() -> String {
    r#"//! The application's palette, in two modes.
//!
//! Written by maxx, yours from here. Add your roles to [`Role`], give each one
//! its two values, and call them from your views.
//!
//! Two modes and not one, because the choice is not yours to make: someone
//! reading in a dark room and someone in the sun are asking for opposite
//! things, and the system already knows which. `gpui_component` carries that
//! answer — its own widgets follow it — so this module reads the mode from
//! there rather than keeping a second one that could disagree.

#![allow(dead_code)]

use gpui::{App, Rgba, Window, rgb};
use gpui_component::{Theme, ThemeMode};

/// One role of the palette, in both modes.
///
/// A role and not a colour: `bg` says where it goes, `0x1e2127` says what it
/// looks like in one of the two modes, and only the first survives a change of
/// palette.
pub struct Role {
    /// The value in the dark mode.
    pub dark: u32,
    /// The value in the light mode.
    pub light: u32,
}

impl Role {
    /// The value for the mode in use.
    pub fn get(&self, cx: &App) -> Rgba {
        rgb(if is_dark(cx) { self.dark } else { self.light })
    }
}

/// Whether the application is drawing in the dark.
pub fn is_dark(cx: &App) -> bool {
    Theme::global(cx).is_dark()
}

/// Switches the palette, and redraws.
///
/// This is what a switch on a view calls. It moves `gpui_component`'s own
/// theme, so the components follow in the same gesture.
pub fn toggle(window: &mut Window, cx: &mut App) {
    let mode = if is_dark(cx) { ThemeMode::Light } else { ThemeMode::Dark };
    Theme::change(mode, Some(window), cx);
}

/// Follows the appearance the system reports.
///
/// Call it once at startup. Without it the application opens in the light mode
/// whatever the system says, which is the one thing every user notices.
pub fn follow_system(window: Option<&mut Window>, cx: &mut App) {
    Theme::sync_system_appearance(window, cx);
}

/// Background of the main area.
pub const BACKGROUND: Role = Role { dark: 0x1e2127, light: 0xfafafa };
/// Background of a panel, a sidebar, a bar.
pub const PANEL: Role = Role { dark: 0x22262d, light: 0xf3f3f3 };
/// Background of a hovered row.
pub const HOVER: Role = Role { dark: 0x2c313a, light: 0xe8e8e8 };
/// Background of a selected row.
pub const SELECTED: Role = Role { dark: 0x3a4048, light: 0xdcdcdc };
/// Separator between regions.
pub const BORDER: Role = Role { dark: 0x2f343d, light: 0xdfdfdf };
/// Default foreground.
pub const TEXT: Role = Role { dark: 0xc8ccd4, light: 0x24292f };
/// Foreground for secondary information.
pub const TEXT_MUTED: Role = Role { dark: 0x7f8896, light: 0x6b7280 };
/// Accent, for what the eye should land on first.
pub const ACCENT: Role = Role { dark: 0x61afef, light: 0x0969da };
/// Foreground on top of the accent.
pub const ON_ACCENT: Role = Role { dark: 0x1e2127, light: 0xffffff };
/// The colour of a failure.
pub const DANGER: Role = Role { dark: 0xe06c75, light: 0xcf222e };
"#
    .to_string()
}

/// The menu bar of a generated project.
///
/// A GPUI application gets no menu bar of its own — not even a Quit — unless it
/// calls `set_menus`, so the template ships a usable one and maxx edits it.
fn menus_rs() -> String {
    r#"use gpui::{
    App, Bounds, Context, Menu, MenuItem, OsAction, SharedString, TitlebarOptions, Window,
    WindowBounds, WindowOptions, actions, div, prelude::*, px, rgb, size,
};

actions!(app, [About, Quit, HideApp, HideOthers, ShowAll, Undo, Redo, Cut, Copy, Paste, SelectAll, Minimize]);

/// Wires what the menu entries do.
pub fn register(cx: &mut App) {
    cx.on_action(|_: &Quit, cx: &mut App| cx.quit());
    cx.on_action(|_: &HideApp, cx: &mut App| cx.hide());
    cx.on_action(|_: &HideOthers, cx: &mut App| cx.hide_other_apps());
    cx.on_action(|_: &ShowAll, cx: &mut App| cx.unhide_other_apps());
    cx.on_action(|_: &About, cx: &mut App| open_about(cx));
    cx.on_action(|_: &Minimize, cx: &mut App| {
        // Deferred: an action handler runs inside the window's own update, and
        // gpui refuses to enter a second one.
        cx.defer(|cx| {
            if let Some(handle) = cx.active_window() {
                let _ = handle.update(cx, |_, window, _| window.minimize_window());
            }
        });
    });
    // maxx:handlers
}

/// The shortcuts the menu entries display.
pub fn key_bindings() -> Vec<gpui::KeyBinding> {
    use gpui::KeyBinding;
    vec![
        KeyBinding::new("cmd-q", Quit, None),
        KeyBinding::new("cmd-h", HideApp, None),
        KeyBinding::new("cmd-alt-h", HideOthers, None),
        KeyBinding::new("cmd-z", Undo, None),
        KeyBinding::new("cmd-shift-z", Redo, None),
        KeyBinding::new("cmd-x", Cut, None),
        KeyBinding::new("cmd-c", Copy, None),
        KeyBinding::new("cmd-v", Paste, None),
        KeyBinding::new("cmd-a", SelectAll, None),
        KeyBinding::new("cmd-m", Minimize, None),
    ]
}

/// What the About window shows.
///
/// Name and version are read from Cargo.toml at build time: `[package]` is the
/// one place a version number should live, and `cargo set-version` or a hand
/// edit there is enough to change what this window says.
struct AboutWindow {
    name: SharedString,
    version: SharedString,
}

impl Render for AboutWindow {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .flex()
            .flex_col()
            .items_center()
            .justify_center()
            .gap_2()
            .size_full()
            .bg(rgb(0x1e2127))
            .text_color(rgb(0xc8ccd4))
            .child(div().text_2xl().child(self.name.clone()))
            .child(
                div()
                    .text_color(rgb(0x7f8896))
                    .child(format!("version {}", self.version)),
            )
    }
}

/// Opens the About window, or brings it forward when it is already up.
///
/// Plain gpui, no `gpui_component`: a window drawing a component widget has to
/// be rooted in `gpui_component::Root`, and this one does not need it.
///
/// Deferred for the same reason as Minimize above: an action handler runs
/// inside the update of a window, and gpui refuses to enter a second one.
fn open_about(cx: &mut App) {
    cx.defer(open_about_now);
}

fn open_about_now(cx: &mut App) {
    if let Some(existing) = cx
        .windows()
        .into_iter()
        .find(|handle| handle.downcast::<AboutWindow>().is_some())
    {
        let _ = existing.update(cx, |_, window, _| window.activate_window());
        return;
    }

    let bounds = Bounds::centered(None, size(px(320.), px(180.)), cx);
    let options = WindowOptions {
        window_bounds: Some(WindowBounds::Windowed(bounds)),
        titlebar: Some(TitlebarOptions {
            title: Some(SharedString::from("About")),
            ..Default::default()
        }),
        is_resizable: false,
        is_minimizable: false,
        ..Default::default()
    };

    cx.open_window(options, |_window, cx| {
        cx.new(|_| AboutWindow {
            name: SharedString::from(env!("CARGO_PKG_NAME")),
            version: SharedString::from(env!("CARGO_PKG_VERSION")),
        })
    })
    .ok();
}

/// The menu bar itself.
pub fn app_menus() -> Vec<Menu> {
    // maxx:begin
    vec![
        Menu {
            name: "app".into(),
            items: vec![
                MenuItem::action("About", About),
                MenuItem::separator(),
                MenuItem::action("Hide", HideApp),
                MenuItem::action("Hide Others", HideOthers),
                MenuItem::action("Show All", ShowAll),
                MenuItem::separator(),
                MenuItem::action("Quit", Quit),
            ],
        },
        Menu {
            name: "Edit".into(),
            items: vec![
                MenuItem::os_action("Undo", Undo, OsAction::Undo),
                MenuItem::os_action("Redo", Redo, OsAction::Redo),
                MenuItem::separator(),
                MenuItem::os_action("Cut", Cut, OsAction::Cut),
                MenuItem::os_action("Copy", Copy, OsAction::Copy),
                MenuItem::os_action("Paste", Paste, OsAction::Paste),
                MenuItem::os_action("Select All", SelectAll, OsAction::SelectAll),
            ],
        },
        Menu {
            name: "Window".into(),
            items: vec![MenuItem::action("Minimize", Minimize)],
        },
    ]
    // maxx:end
}
"#
    .to_string()
}

/// The entry point of the generated project.
///
/// `own_title_bar` says whether the window carries a title bar of its own — a
/// shape decision, taken once at creation. It has to be taken here and not in
/// the shell: `TitleBar::title_bar_options()` is what makes the system bar
/// transparent, and `gpui_component::TitleBar` is what is drawn in its place.
/// Either one without the other is visible at a glance — a doubled bar, or a
/// bare strip where the traffic lights sit.
fn main_rs(own_title_bar: bool) -> String {
    let (import, option) = if own_title_bar {
        (
            "use gpui_component::{Root, TitleBar};\n\n",
            "                titlebar: Some(TitleBar::title_bar_options()),\n",
        )
    } else {
        ("use gpui_component::Root;\n\n", "")
    };

    let body = r#"mod menus;
mod ui;

use gpui::{App, Application, Bounds, WindowBounds, WindowOptions, prelude::*, px, size};
IMPORT
use crate::ui::home::Home;

fn main() {
    Application::new().run(|cx: &mut App| {
        gpui_component::init(cx);
        cx.activate(true);

        menus::register(cx);
        cx.bind_keys(menus::key_bindings());
        cx.set_menus(menus::app_menus());

        let bounds = Bounds::centered(None, size(px(900.), px(600.)), cx);
        cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
OPTION                ..Default::default()
            },
            |window, cx| {
                let view = cx.new(|cx| Home::new(window, cx));
                cx.new(|cx| Root::new(view, window, cx))
            },
        )
        .expect("the window must open");
    });
}
"#;

    body.replace("IMPORT\n", import).replace("OPTION", option)
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
fn view_rs(type_name: &str, module: &str) -> String {
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

/// Gives an existing project a menu bar: writes `src/menus.rs` and wires it
/// into `src/main.rs`.
///
/// Wired by textual insertion, like `create_view`: the project may predate the
/// template entirely, and rewriting its `main.rs` from the template would throw
/// away whatever it does at startup.
pub fn add_menu_bar(root: &Path) -> io::Result<()> {
    // `main.rs` is patched first, and nothing is written until it is known to
    // work: a `src/menus.rs` left behind by a failed wiring would make the next
    // attempt believe the project already has a menu bar.
    let main_path = root.join("src/main.rs");
    let source = std::fs::read_to_string(&main_path)?;
    let mut lines: Vec<String> = source.lines().map(str::to_string).collect();

    if !lines.iter().any(|line| line.trim() == "mod menus;") {
        lines.insert(header_end(&lines), "mod menus;".into());
    }

    if !source.contains("menus::app_menus()") {
        // `cx.activate` is what every gpui `main` does first; failing that, the
        // line that opens the closure `run` was given.
        let anchor = lines.iter().position(|line| line.contains(".activate(")).or_else(|| {
            lines.iter().position(|line| line.contains(".run(") && line.trim_end().ends_with('{'))
        });
        let Some(anchor) = anchor else {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "src/main.rs: neither cx.activate(…) nor Application::new().run(…) — \
                 add menus::register(cx), cx.bind_keys(menus::key_bindings()) \
                 and cx.set_menus(menus::app_menus()) by hand",
            ));
        };

        // The three calls need the name this `main` gave its application. Both
        // anchors carry it, in different places: `cx.activate(true)` names it
        // as the receiver, `run(|app| {` as the closure's argument. Assuming
        // `cx` would hand a project written as `run(|app| {` three lines naming
        // something that does not exist.
        let Some(app) = application_binding(&lines[anchor]) else {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "src/main.rs: cannot read the application's name in “{}” — \
                     add menus::register(…), …bind_keys(menus::key_bindings()) \
                     and …set_menus(menus::app_menus()) by hand",
                    lines[anchor].trim()
                ),
            ));
        };

        let indent: String =
            lines[anchor].chars().take_while(|character| character.is_whitespace()).collect();
        for (offset, call) in [
            format!("menus::register({app});"),
            format!("{app}.bind_keys(menus::key_bindings());"),
            format!("{app}.set_menus(menus::app_menus());"),
        ]
        .iter()
        .enumerate()
        {
            lines.insert(anchor + 1 + offset, format!("{indent}{call}"));
        }
    }

    let menus_path = root.join("src/menus.rs");
    let created = !menus_path.exists();
    if created {
        std::fs::write(&menus_path, menus_rs())?;
    }

    if let Err(error) = std::fs::write(&main_path, joined(&lines, &source)) {
        // A `menus.rs` left behind by a failed wiring would make the next
        // attempt believe the project already has a menu bar, and skip the
        // wiring for good.
        if created {
            let _ = std::fs::remove_file(&menus_path);
        }
        return Err(error);
    }
    Ok(())
}

/// Whether `line` is one of the three calls `add_menu_bar` writes, or the
/// module declaration that goes with them.
fn is_menu_wiring(line: &str) -> bool {
    if line == "mod menus;" || line == "menus::register" {
        return true;
    }
    if let Some(argument) =
        line.strip_prefix("menus::register(").and_then(|rest| rest.strip_suffix(");"))
    {
        return identifier(argument).is_some();
    }
    for call in [".bind_keys(menus::key_bindings());", ".set_menus(menus::app_menus());"] {
        if let Some(receiver) = line.strip_suffix(call)
            && identifier(receiver).is_some()
        {
            return true;
        }
    }
    false
}

/// The name `line` gives the application.
///
/// Either the receiver of `.activate(`, or the argument of the closure handed
/// to `run` — `|cx|` as much as `|app: &mut App|`.
fn application_binding(line: &str) -> Option<String> {
    if let Some(dot) = line.find(".activate(") {
        let receiver: String = line[..dot]
            .chars()
            .rev()
            .take_while(|character| character.is_alphanumeric() || *character == '_')
            .collect();
        return identifier(&receiver.chars().rev().collect::<String>());
    }

    let start = line.find('|')?;
    let rest = &line[start + 1..];
    let end = rest.find('|')?;
    identifier(rest[..end].split(':').next()?.trim())
}

/// `name` when it can be a Rust binding, nothing otherwise.
fn identifier(name: &str) -> Option<String> {
    let valid = !name.is_empty()
        && name.chars().all(|character| character.is_alphanumeric() || character == '_')
        && !name.starts_with(|character: char| character.is_ascii_digit());
    valid.then(|| name.to_string())
}

/// Unwires the menu bar from `src/main.rs`.
///
/// The file `src/menus.rs` is the caller's business — the project panel puts it
/// in the Trash — but leaving `mod menus;` behind would stop the project from
/// compiling.
pub fn remove_menu_bar(root: &Path) -> io::Result<()> {
    let main_path = root.join("src/main.rs");
    let source = std::fs::read_to_string(&main_path)?;
    // Matched on shape, not on the exact text: `add_menu_bar` writes these
    // with the name the project gave its application, which is `cx` in the
    // template and anything at all in a hand-written `main.rs`. Filtering
    // literal `cx` lines would leave a call to a module that no longer exists.
    let kept: Vec<String> =
        source.lines().filter(|line| !is_menu_wiring(line.trim())).map(str::to_string).collect();
    std::fs::write(&main_path, joined(&kept, &source))
}

/// Adds the assets module to an existing project, with its build script, and
/// hands it to the application.
///
/// Two files, and both are needed: `src/assets.rs` declares the `AssetSource`,
/// `build.rs` is what embeds the files it serves. The wiring is a single call
/// on `Application::new()`, so removing the module leaves a line that still
/// compiles.
pub fn add_assets_module(root: &Path) -> io::Result<()> {
    if let Some(error) = legacy_copy(root, "assets") {
        return Err(error);
    }

    // A build script the developer wrote is theirs: appending to it would put
    // maxx in the middle of a file it does not understand.
    let build_path = root.join("build.rs");
    let ours = std::fs::read_to_string(&build_path).is_ok_and(|body| body.contains(BUILD_MARKER));
    if build_path.exists() && !ours {
        return Err(io::Error::new(
            io::ErrorKind::AlreadyExists,
            "build.rs is already yours — copy the body of maxx's own build script into \
             its main by hand, or move yours aside and add the module again",
        ));
    }

    // `main.rs` is patched first, and nothing is written until it is known to
    // work: the two files left behind by a failed wiring would make the next
    // attempt believe the project already carries the module.
    let main_path = root.join("src/main.rs");
    let source = std::fs::read_to_string(&main_path)?;
    let mut lines: Vec<String> = source.lines().map(str::to_string).collect();

    if !lines.iter().any(|line| line.trim() == "mod assets;") {
        lines.insert(header_end(&lines), "mod assets;".into());
    }

    if !source.contains(WITH_ASSETS) {
        let Some(anchor) = lines.iter().position(|line| line.contains(APPLICATION_NEW)) else {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "src/main.rs: no {APPLICATION_NEW} — add {WITH_ASSETS} to whatever \
                     builds the application by hand"
                ),
            ));
        };
        lines[anchor] =
            lines[anchor].replacen(APPLICATION_NEW, &format!("{APPLICATION_NEW}{WITH_ASSETS}"), 1);
    }

    let path = root.join("src/assets.rs");
    let body = assets_rs();
    let wrote_module = !path.exists();
    if wrote_module {
        std::fs::write(&path, &body)?;
    }
    let wrote_build = !build_path.exists();
    if wrote_build {
        std::fs::write(&build_path, assets_build_rs())?;
    }

    if let Err(error) = std::fs::write(&main_path, joined(&lines, &source)) {
        if wrote_module {
            let _ = std::fs::remove_file(&path);
        }
        if wrote_build {
            let _ = std::fs::remove_file(&build_path);
        }
        return Err(error);
    }
    if wrote_module {
        crate::projectfile::record(root, "assets", module_version("assets").unwrap_or(1), &body)?;
    }
    Ok(())
}

/// What tells maxx's build script apart from one the developer wrote.
const BUILD_MARKER: &str = "maxx:assets";

/// The call the assets module hangs on, and the one it is added to.
const APPLICATION_NEW: &str = "Application::new()";
const WITH_ASSETS: &str = ".with_assets(assets::Assets)";

/// The assets module of a generated project.
fn assets_rs() -> String {
    r##"//! The project's own files, served to gpui.
//!
//! `img("assets/images/logo.png")` asks gpui for an *asset*, and gpui asks the
//! `AssetSource` the application was built with. Without one, nothing is drawn
//! and a single line goes to the log — so this module is what makes the
//! pictures appear.
//!
//! It answers from two places, in this order:
//!
//! 1. What `build.rs` embedded in the binary: everything under `assets/` and
//!    `icons/` at build time is compiled into the executable, so the
//!    application carries its pictures wherever it goes. A binary someone
//!    double-clicks has no idea where the project directory is.
//! 2. Failing that, the file on disk, read from the directory the process
//!    started in. That is what picks up a picture added since the last build,
//!    and what serves the ones kept outside `assets/`.
//!
//! The price is plain, and it is worth saying out loud: the binary grows by
//! the size of what is embedded.
//!
//! The contract with `build.rs` — change one, change the other: the build
//! script writes `assets.rs` into `OUT_DIR`, declaring `ASSETS`, a table of
//! project-relative path and bytes.
//!
//! `icons/` is walked for `gpui_component`, whose `IconName` asks for
//! `icons/*.svg` as assets: dropping that folder at the root of the project is
//! all it takes for the icons to appear.

#![allow(dead_code)]

use std::borrow::Cow;

use gpui::{AssetSource, Result, SharedString};

// Written by build.rs: `pub static ASSETS: &[(&str, &[u8])]`.
include!(concat!(env!("OUT_DIR"), "/assets.rs"));

/// Hand it over at startup: `Application::new().with_assets(assets::Assets)`.
pub struct Assets;

impl AssetSource for Assets {
    fn load(&self, path: &str) -> Result<Option<Cow<'static, [u8]>>> {
        for (key, bytes) in ASSETS {
            if *key == path {
                return Ok(Some(Cow::Borrowed(*bytes)));
            }
        }
        match std::fs::read(path) {
            Ok(bytes) => Ok(Some(Cow::Owned(bytes))),
            // Missing is not an error: gpui logs what it cannot load and draws
            // the fallback, and one absent picture is no reason to stop.
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(error) => Err(error.into()),
        }
    }

    fn list(&self, path: &str) -> Result<Vec<SharedString>> {
        let mut names: Vec<SharedString> = ASSETS
            .iter()
            // On a segment boundary: `list("icons")` has no business answering
            // for a sibling `icons_extra/`.
            .filter(|(key, _)| key.strip_prefix(path).is_some_and(|rest| rest.starts_with('/')))
            .map(|(key, _)| SharedString::from(*key))
            .collect();

        if let Ok(entries) = std::fs::read_dir(path) {
            for entry in entries.flatten() {
                let name = entry.path().to_string_lossy().replace('\\', "/");
                if !names.iter().any(|known| known.as_ref() == name) {
                    names.push(SharedString::from(name));
                }
            }
        }
        Ok(names)
    }
}
"##
    .to_string()
}

/// The build script that goes with the assets module.
fn assets_build_rs() -> String {
    r##"//! Embeds the project's own files into the binary. maxx:assets
//!
//! Walks the directories of `ROOTS` and writes `assets.rs` into `OUT_DIR`,
//! which `src/assets.rs` includes. The contract between the two is this file's
//! output: `pub static ASSETS: &[(&str, &[u8])]`, keyed by the path relative
//! to the project root — the very string the code hands to `img(…)`.
//!
//! Written by maxx, yours from here. Add a directory to `ROOTS` and it travels
//! inside the binary too.

use std::fmt::Write as _;
use std::path::{Path, PathBuf};

/// The directories whose contents travel inside the binary.
const ROOTS: &[&str] = &["assets", "icons"];

fn main() {
    let mut table = String::from(
        "// Written by build.rs. Do not edit.\npub static ASSETS: &[(&str, &[u8])] = &[\n",
    );
    for root in ROOTS {
        println!("cargo::rerun-if-changed={root}");
        collect(Path::new(root), &mut table);
    }
    table.push_str("];\n");

    let out = PathBuf::from(std::env::var("OUT_DIR").expect("cargo sets OUT_DIR"));
    std::fs::write(out.join("assets.rs"), table).expect("the asset table must be written");
}

/// Adds every file under `directory` to the table, recursively.
///
/// A directory that is not there is not a failure: a project keeps its
/// pictures where it likes, and `icons/` is only there once someone wants the
/// gpui-component icons.
fn collect(directory: &Path, table: &mut String) {
    let Ok(entries) = std::fs::read_dir(directory) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let name = path.file_name().and_then(|name| name.to_str()).unwrap_or(".");
        // Dotfiles belong to the system, not to the project: `.DS_Store` inside
        // the binary is bytes nobody asked for.
        if name.starts_with('.') {
            continue;
        }
        if path.is_dir() {
            collect(&path, table);
            continue;
        }
        // Forward slashes, whatever the system: the key has to match the string
        // written in the source, and that one is written once.
        let key = path.to_string_lossy().replace('\\', "/");
        println!("cargo::rerun-if-changed={key}");
        // `{key:?}` twice, and not once: a quote or a backslash in a file name
        // has to be escaped in the key and in the path just the same.
        let _ = writeln!(
            table,
            "    ({key:?}, include_bytes!(concat!(env!(\"CARGO_MANIFEST_DIR\"), \"/\", {key:?}))),"
        );
    }
}
"##
    .to_string()
}

/// Adds the window module to an existing project and wires it into `main.rs`.
///
/// Pulls the system module in with it: knowing where this system puts an
/// application's files is exactly what `system.rs` answers. And declares
/// `serde` and `serde_json_lenient`, both already compiled in the tree through
/// gpui, so the build does not grow.
///
/// Wired by textual insertion, like `add_menu_bar`: the project may predate the
/// module entirely, and rewriting its `main.rs` from the template would throw
/// away whatever it does at startup. Each inserted line is a whole statement,
/// so removing the module leaves a `main.rs` that still compiles.
pub fn add_window_module(root: &Path) -> io::Result<()> {
    if let Some(error) = legacy_copy(root, "window") {
        return Err(error);
    }
    let main_path = root.join("src/main.rs");
    // Everything is checked before anything at all is written: pulling the
    // system module in and then refusing would leave the project half-changed
    // for a module that was never added — and `maxx.toml` would record it.
    wire_window(&std::fs::read_to_string(&main_path)?)?;
    dependencies_section(&std::fs::read_to_string(root.join("Cargo.toml"))?)?;

    add_system_module(root)?;
    add_dependencies(
        root,
        &[
            ("serde", "{ version = \"1\", features = [\"derive\"] }"),
            ("serde_json_lenient", "\"0.2\""),
        ],
    )?;

    // Read again: `add_system_module` has just inserted a line of its own.
    let source = std::fs::read_to_string(&main_path)?;
    let lines = wire_window(&source)?;

    let path = root.join("src/window.rs");
    let body = window_rs();
    let created = !path.exists();
    if created {
        std::fs::write(&path, &body)?;
    }

    if let Err(error) = std::fs::write(&main_path, joined(&lines, &source)) {
        if created {
            let _ = std::fs::remove_file(&path);
        }
        return Err(error);
    }
    if created {
        crate::projectfile::record(root, "window", module_version("window").unwrap_or(1), &body)?;
    }
    Ok(())
}

/// `src/main.rs`, line by line, with the window module declared and called.
///
/// Each inserted line is a whole statement — a shadowing rebind before the
/// window opens, a call inside it — so that removing the module later leaves a
/// file that still compiles. A call written as an argument to another one would
/// leave a hole where a value is expected.
fn wire_window(source: &str) -> io::Result<Vec<String>> {
    let mut lines: Vec<String> = source.lines().map(str::to_string).collect();

    if !lines.iter().any(|line| line.trim() == "mod window;") {
        lines.insert(header_end(&lines), "mod window;".into());
    }

    if !source.contains("window::bounds(") {
        // The line that computes the bounds, and the name it gives them.
        let anchor = lines.iter().position(|line| {
            let line = line.trim_start();
            line.starts_with("let ") && line.contains("= Bounds::centered(")
        });
        let binding = anchor.and_then(|index| bounds_binding(&lines[index]));
        // The end of the statement, and not the line the anchor is on: rustfmt
        // wraps a long `Bounds::centered(…)` over several lines, and inserting
        // after the first of them drops a statement into an argument list — a
        // `main.rs` that no longer parses, written and reported as a success.
        let end = anchor.and_then(|index| {
            lines[index..]
                .iter()
                .position(|line| line.trim_end().ends_with(';'))
                .map(|offset| index + offset)
        });
        let (Some(anchor), Some(end), Some(binding)) = (anchor, end, binding) else {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "src/main.rs: no `let … = Bounds::centered(…);` — add \
                 `let bounds = window::bounds(bounds);` before the window opens \
                 and `window::remember(&window, cx);` inside it, by hand",
            ));
        };
        let indent: String =
            lines[anchor].chars().take_while(|character| character.is_whitespace()).collect();
        lines.insert(end + 1, format!("{indent}let {binding} = window::bounds({binding});"));
    }

    if !source.contains("window::remember(") {
        // The closure `open_window` was given: its two arguments are the window
        // and the application, whatever this `main.rs` calls them.
        let opened = lines.iter().position(|line| line.contains(".open_window("));
        let closure = opened.and_then(|start| closure_of_call(&lines, start));
        let arguments = closure.and_then(|index| closure_arguments(&lines[index]));
        let (Some(index), Some((window, app))) = (closure, arguments) else {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "src/main.rs: cannot find the closure open_window was given — add \
                 `window::remember(&window, cx);` as its first line, by hand",
            ));
        };
        let indent: String =
            lines[index].chars().take_while(|character| character.is_whitespace()).collect();
        lines.insert(index + 1, format!("{indent}    window::remember(&{window}, {app});"));
    }

    Ok(lines)
}

/// The line opening the closure the call on `start` was given.
///
/// Bounded to that call, by counting its parentheses: scanning to the end of the
/// file found the next closure anywhere — a `cx.observe_new(|view, window, cx| {`
/// twenty lines further down — and the call was written into it. And the search
/// starts on the line of the call itself, because a short `open_window` puts its
/// closure there.
fn closure_of_call(lines: &[String], start: usize) -> Option<usize> {
    let open = lines[start].find(".open_window(")? + ".open_window(".len();
    let mut depth = 1i32;
    for (offset, line) in lines[start..].iter().enumerate() {
        if closure_arguments(line).is_some() {
            return Some(start + offset);
        }
        let text = if offset == 0 { &line[open..] } else { &line[..] };
        for character in text.chars() {
            match character {
                '(' => depth += 1,
                ')' => depth -= 1,
                _ => {}
            }
        }
        if depth <= 0 {
            return None;
        }
    }
    None
}

/// The name `line` gives the bounds it computes.
fn bounds_binding(line: &str) -> Option<String> {
    let rest = line.trim_start().strip_prefix("let ")?;
    let name = rest.split('=').next()?.trim();
    // `let mut bounds = …` names them too, and the shadowing rebind must carry
    // the `mut` no further than the name.
    let name = name.strip_prefix("mut ").unwrap_or(name);
    identifier(name)
}

/// The window and the application, as the closure opened on `line` names them.
///
/// `|window, cx| {` as much as `|w: &mut Window, app: &mut App| {`, and wherever
/// on the line it sits: rustfmt writes a short call and its closure together.
/// Anything that is not exactly two named arguments answers `None`, which is
/// what keeps a `while a || b {` from being taken for a closure.
fn closure_arguments(line: &str) -> Option<(String, String)> {
    let line = line.trim_end();
    if !line.ends_with('{') {
        return None;
    }
    let close = line.rfind('|')?;
    let open = line[..close].rfind('|')?;
    let mut names = line[open + 1..close].split(',').map(|argument| {
        let name = argument.split(':').next().unwrap_or_default().trim();
        identifier(name.strip_prefix("mut ").unwrap_or(name))
    });
    let window = names.next()??;
    let app = names.next()??;
    if names.next().is_some() {
        return None;
    }
    Some((window, app))
}

/// The window module of a generated project.
fn window_rs() -> String {
    r##"//! Where the window was when the application last closed.
//!
//! Written by maxx, yours from here.
//!
//! Two files rather than one: `settings.json` is the user's — annotated,
//! rewritten one key at a time — and this one is the machine's. A window's
//! geometry moves every time it is dragged and nobody edits it by hand, so it
//! has no business in the file the user is invited to open.
//!
//! `bounds` is called before the window opens, `remember` from inside it. The
//! saved geometry goes to the *first* window only: a second one given the same
//! bounds lands pixel for pixel on the first, hiding the window someone is
//! still using.
//!
//! A geometry saved on a screen that is no longer plugged in needs no check
//! here — gpui folds an off-screen window back onto the main display.

#![allow(dead_code)]

use std::path::PathBuf;

use gpui::{App, Bounds, Pixels, Window, point, px, size};
use serde::{Deserialize, Serialize};

/// The application's folder name, under the configuration directory.
const APPLICATION: &str = env!("CARGO_PKG_NAME");

/// A window's place on the desktop, in pixels.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Geometry {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
}

/// What this machine remembers. Add your fields here.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct State {
    pub window: Option<Geometry>,
}

/// Where the file lives.
pub fn path() -> Option<PathBuf> {
    crate::system::config_dir(APPLICATION).map(|folder| folder.join("state.json"))
}

/// Reads what was remembered. A file that is missing or damaged is no worse
/// than no file at all.
pub fn load() -> State {
    let Some(path) = path() else {
        return State::default();
    };
    let Ok(source) = std::fs::read_to_string(&path) else {
        return State::default();
    };
    serde_json_lenient::from_str_lenient(&source).unwrap_or_default()
}

/// Writes the state whole.
///
/// Whole, and not one key at a time like the settings: nobody hand-edits this
/// file, so there are no comments to keep.
pub fn save(state: &State) -> std::io::Result<()> {
    let Some(path) = path() else {
        return Ok(());
    };
    let Ok(body) = serde_json_lenient::to_string_pretty(state) else {
        return Ok(());
    };
    crate::system::write_atomically(&path, &body)
}

/// The bounds to open with: the remembered ones, or `fallback`.
pub fn bounds(fallback: Bounds<Pixels>) -> Bounds<Pixels> {
    match load().window {
        Some(geometry) => Bounds {
            origin: point(px(geometry.x), px(geometry.y)),
            size: size(px(geometry.width), px(geometry.height)),
        },
        None => fallback,
    }
}

/// Saves the geometry when the window closes, and when the application quits.
///
/// Both, because they are two different exits: the close button and `cmd-w` go
/// through the first, `cmd-q` through the second. Neither costs anything per
/// frame — the geometry is read once, at the moment it stops changing.
pub fn remember(window: &Window, cx: &mut App) {
    window.on_window_should_close(cx, |window, _cx| {
        write(window);
        true
    });

    let handle = window.window_handle();
    cx.on_app_quit(move |cx: &mut App| {
        // The windows are still there: gpui runs the quit observers before it
        // drops them.
        let _ = handle.update(cx, |_, window, _| write(window));
        async {}
    })
    .detach();
}

/// Writes this window's geometry now.
pub fn write(window: &Window) {
    // `window_bounds()` and not `bounds()`: of a full-screen window the first
    // answers the size it will come back to, the second the whole display.
    // Saving the display reopens a window as large as the screen.
    let bounds = window.window_bounds().get_bounds();
    let mut state = load();
    state.window = Some(Geometry {
        x: f32::from(bounds.origin.x),
        y: f32::from(bounds.origin.y),
        width: f32::from(bounds.size.width),
        height: f32::from(bounds.size.height),
    });
    let _ = save(&state);
}
"##
    .to_string()
}
