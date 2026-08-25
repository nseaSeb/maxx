//! What maxx remembers between two launches.
//!
//! Two files, the way Zed splits them, because they are not the same kind of
//! thing:
//!
//! - `settings.json` belongs to the user. It is written by hand as much as by
//!   maxx, so maxx patches only the key it changes and leaves the rest of the
//!   bytes alone — the same rule it applies to a `.rs` file. Comments and
//!   layout survive.
//! - `state.json` belongs to the machine: recent projects, window geometry.
//!   Nobody edits it, so it is rewritten whole.
//!
//! JSON with comments, read through `serde_json_lenient` — the crate Zed reads
//! its own settings with, already in the tree through gpui. Plain JSON cannot
//! hold a comment, and a settings file you cannot annotate is a settings file
//! you have to keep a wiki about.
//!
//! Everything has a default that reproduces the behaviour maxx had before it
//! read anything: a missing, partial or damaged file must never be worse than
//! no file at all.

use std::path::{Path, PathBuf};

use gpui::{App, Global};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// How many projects the "Open Recent" list keeps.
const RECENT_LIMIT: usize = 10;

/// What the user chooses. Lives in `settings.json`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(default)]
pub struct Preferences {
    /// Whether the project panel is shown.
    pub show_project_panel: bool,
    /// Whether the status bar is shown.
    pub show_status_bar: bool,
    /// Whether the output panel is shown.
    pub show_output: bool,
    /// Which editor files are handed to, or `auto`.
    pub editor: String,
    /// Which terminal is opened, or `auto`.
    pub terminal: String,
    /// Whether `rustfmt` is run on a file after maxx writes it.
    pub format_on_save: bool,
    /// The interface language: `system`, or a language code maxx translates.
    pub language: String,
    /// The palette: `system`, `light` or `dark`.
    pub theme: String,
}

impl Default for Preferences {
    fn default() -> Self {
        Self {
            show_project_panel: true,
            show_status_bar: true,
            show_output: false,
            editor: crate::tools::AUTOMATIC.into(),
            terminal: crate::tools::AUTOMATIC.into(),
            // Allumé, et ce n'est pas un excès de zèle. Un éditeur Rust
            // formate à l'enregistrement — c'est le défaut de Zed comme de
            // rust-analyzer —, et ce que `codegen` écrit n'est pas ce que
            // rustfmt écrirait. Sans ce réglage, chaque enregistrement dans
            // l'éditeur reformate la zone gérée, que maxx réécrit à sa façon à
            // l'enregistrement suivant : une partie de bras de fer, et un
            // diff parasite à chaque tour. maxx applique donc lui-même ce que
            // l'éditeur appliquerait de toute façon.
            format_on_save: true,
            language: "system".into(),
            theme: "system".into(),
        }
    }
}

/// The saved position and size of the workspace window.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct WindowGeometry {
    /// Distance from the left of the display, in logical pixels.
    pub x: f32,
    /// Distance from the top of the display, in logical pixels.
    pub y: f32,
    /// Width in logical pixels.
    pub width: f32,
    /// Height in logical pixels.
    pub height: f32,
}

/// What maxx notices on its own. Lives in `state.json`.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct State {
    /// Projects opened before, most recent first.
    pub recent_projects: Vec<PathBuf>,
    /// Where the workspace window was left.
    pub window: Option<WindowGeometry>,
    /// Width of the project panel, in logical pixels.
    pub panel_width: Option<f32>,
    /// Width of the inspector, in logical pixels.
    pub inspector_width: Option<f32>,
}

impl State {
    /// Puts `path` at the head of the recent list.
    ///
    /// Answers whether the list changed, so the caller can skip rewriting the
    /// menu bar and the file when reopening the project already at the head.
    pub fn remember_project(&mut self, path: &Path) -> bool {
        if self.recent_projects.first().map(PathBuf::as_path) == Some(path) {
            return false;
        }
        self.recent_projects.retain(|recent| recent != path);
        self.recent_projects.insert(0, path.to_path_buf());
        self.recent_projects.truncate(RECENT_LIMIT);
        true
    }

    /// Drops the recent entries whose directory no longer exists.
    ///
    /// Run at startup: a menu offering a project that was moved or deleted is
    /// worse than a shorter menu.
    pub fn forget_missing_projects(&mut self) {
        self.recent_projects.retain(|path| path.is_dir());
    }
}

/// The directory the files live in.
///
/// The three conventions, in order: `XDG_CONFIG_HOME` when the user set it,
/// `APPDATA` on Windows, and otherwise the home directory — under
/// `Library/Application Support` on macOS, `.config` elsewhere.
pub fn directory() -> Option<PathBuf> {
    if cfg!(target_os = "windows") {
        return Some(PathBuf::from(std::env::var("APPDATA").ok()?).join("maxx"));
    }
    if let Ok(xdg) = std::env::var("XDG_CONFIG_HOME")
        && !xdg.is_empty()
    {
        return Some(PathBuf::from(xdg).join("maxx"));
    }
    let home = PathBuf::from(std::env::var("HOME").ok()?);
    Some(if cfg!(target_os = "macos") {
        home.join("Library/Application Support/maxx")
    } else {
        home.join(".config/maxx")
    })
}

/// The file the user edits.
pub fn settings_path() -> Option<PathBuf> {
    Some(directory()?.join("settings.json"))
}

/// The file maxx writes for itself.
pub fn state_path() -> Option<PathBuf> {
    Some(directory()?.join("state.json"))
}

/// The JSON Schema maxx writes beside the settings, for editor completion.
pub fn schema_path() -> Option<PathBuf> {
    Some(directory()?.join("settings-schema.json"))
}

/// Reads a JSON document, tolerating comments and trailing commas.
///
/// A damaged file is reported and then ignored, and left untouched on disk:
/// refusing to start over a stray comma would be a poor trade, and rewriting
/// it would lose whatever the user was in the middle of writing.
pub fn read_json<T: Default + serde::de::DeserializeOwned>(path: &Path) -> T {
    let Ok(source) = std::fs::read_to_string(path) else {
        return T::default();
    };
    match serde_json_lenient::from_str_lenient(&source) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("{} illisible : {error}", path.display());
            T::default()
        }
    }
}

/// Writes `body` to `path` through a temporary file.
///
/// A direct write interrupted halfway leaves a truncated file, which the next
/// launch would read as a damaged one.
fn write_atomically(path: &Path, body: &str) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    // The name, not the extension: `reglages.json` and `reglages.toml` would
    // otherwise both write to `reglages.tmp` and clobber each other.
    let name = path.file_name().unwrap_or_default().to_string_lossy();
    let temporary = path.with_file_name(format!("{name}.tmp"));
    std::fs::write(&temporary, body)?;
    if let Err(error) = std::fs::rename(&temporary, path) {
        let _ = std::fs::remove_file(&temporary);
        return Err(error);
    }
    Ok(())
}

/// The settings file maxx writes when there is none.
///
/// Every key, with its default and a line saying what it does — the file is its
/// own documentation, which is the part of Zed's settings worth copying before
/// any question of format.
///
/// In English whatever the interface speaks. The file is written once, keeps
/// whatever the user then writes in it, and is read by hand years later; a
/// comment that depends on the language selected on the day of the first launch
/// would be a strange thing to find in it.
pub fn documented_defaults() -> String {
    let defaults = Preferences::default();
    format!(
        r#"// maxx settings.
//
// This file is yours. maxx only rewrites the key it changes: your comments and
// your layout stay where they are. Comments and trailing commas are accepted
// when reading.
{{
  "$schema": "./settings-schema.json",

  // The explorer, on the left. ⌘B does the same thing.
  "show_project_panel": {},

  // The bottom line: the view's name, messages, conflicts.
  "show_status_bar": {},

  // What cargo writes during a run. ⌘J toggles it.
  "show_output": {},

  // The editor files are handed to, and the terminal opened on the project.
  // "auto" takes the first one installed — for the editor, $VISUAL and $EDITOR
  // come first. The possible values are listed in the schema.
  "editor": "{}",
  "terminal": "{}",

  // Run rustfmt on the file after every save, so that what maxx writes follows
  // the project's conventions — its rustfmt.toml included.
  //
  // On: a Rust editor formats on save, and what maxx writes is not what rustfmt
  // would write. Without this, the editor and maxx reformat the managed region
  // at each other every round. Turn it off if your project does not use
  // rustfmt — it formats the whole file.
  "format_on_save": {},

  // The interface language. "system" follows what the system reports and falls
  // back to English; "en" and "fr" pin it. Anything maxx has not translated
  // shows in English.
  "language": "{}",

  // The palette maxx draws with, and the one the canvas previews the
  // components in. "system" follows the appearance the window reports.
  "theme": "{}"
}}
"#,
        defaults.show_project_panel,
        defaults.show_status_bar,
        defaults.show_output,
        defaults.editor,
        defaults.terminal,
        defaults.format_on_save,
        defaults.language,
        defaults.theme
    )
}

/// Skips whitespace and JSONC comments, from `index` onward.
///
/// A comment is not JSON, and the scan below has to step over one without
/// reading a brace, a quote or a colon inside it as structure. Getting this
/// wrong destroys the file: a lone `"` in a comment leaves the scanner inside
/// a string it never leaves.
fn skip_trivia(bytes: &[u8], mut index: usize) -> usize {
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

/// Index just past the string literal whose opening quote is at `index`.
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

/// Index just past the JSON value starting at `index`.
fn end_of_value(bytes: &[u8], index: usize) -> usize {
    if index >= bytes.len() {
        return bytes.len();
    }
    match bytes[index] {
        b'"' => end_of_string(bytes, index),
        open @ (b'[' | b'{') => {
            let close = if open == b'[' { b']' } else { b'}' };
            let mut depth = 0usize;
            let mut index = index;
            while index < bytes.len() {
                index = skip_trivia(bytes, index);
                if index >= bytes.len() {
                    break;
                }
                match bytes[index] {
                    b'"' => {
                        index = end_of_string(bytes, index);
                        continue;
                    }
                    byte if byte == open => depth += 1,
                    byte if byte == close => {
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
        // A number, `true`, `false` or `null`: it ends at the first thing that
        // cannot be part of it — a delimiter, a space, or a comment.
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

/// Where a top-level member sits: the span of its value.
struct Member {
    value: std::ops::Range<usize>,
}

/// Walks the members of the flat object at the top of `source`.
///
/// Answers the span of `key`'s value when it is there, and where a new member
/// could be inserted otherwise — right after the opening brace, which is the
/// one position no trailing comment can spoil.
fn walk(source: &str, key: &str) -> (Option<Member>, Option<usize>) {
    let bytes = source.as_bytes();
    // Not `find('{')`: the documented default file opens with a comment block,
    // and a brace written inside it would anchor the whole walk in the comment.
    let brace = skip_trivia(bytes, 0);
    if brace >= bytes.len() || bytes[brace] != b'{' {
        return (None, None);
    }
    let after_brace = brace + 1;

    let mut index = skip_trivia(bytes, after_brace);
    let needle = format!("\"{key}\"");
    while index < bytes.len() && bytes[index] != b'}' {
        if bytes[index] != b'"' {
            // Not a member start: the document is not what we assume, so stop
            // rather than write into the middle of something.
            break;
        }
        let name_end = end_of_string(bytes, index);
        let matched = source[index..name_end] == needle;

        let colon = skip_trivia(bytes, name_end);
        if colon >= bytes.len() || bytes[colon] != b':' {
            break;
        }
        let value_start = skip_trivia(bytes, colon + 1);
        let value_end = end_of_value(bytes, value_start);
        if matched {
            return (Some(Member { value: value_start..value_end }), Some(after_brace));
        }

        index = skip_trivia(bytes, value_end);
        if index < bytes.len() && bytes[index] == b',' {
            index = skip_trivia(bytes, index + 1);
        }
    }

    (None, Some(after_brace))
}

/// Replaces the value of a top-level `key`, keeping every other byte.
///
/// The same move maxx makes on a `.rs` file: find the span, splice, leave the
/// rest alone. The document is a flat object, so walking its members — over
/// strings, escapes, nesting *and* comments — is enough. No parser is involved,
/// so none can reformat the file behind the user's back.
///
/// Answers `None` when the key is not there, which is the caller's cue to add
/// it with [`append_key`].
pub fn splice_key(source: &str, key: &str, value: &str) -> Option<String> {
    let (member, _) = walk(source, key);
    let member = member?;
    Some(format!("{}{value}{}", &source[..member.value.start], &source[member.value.end..]))
}

/// Adds `key` to a flat object, right after its opening brace.
///
/// After the brace and not before the closing one: the last thing inside an
/// object is very often a comment, and a comma appended there would land
/// inside it — commented out, leaving two members with nothing between them.
pub fn append_key(source: &str, key: &str, value: &str) -> String {
    let (_, insert_at) = walk(source, key);
    let Some(insert_at) = insert_at else {
        return format!("{{\n  \"{key}\": {value}\n}}\n");
    };

    let bytes = source.as_bytes();
    let next = skip_trivia(bytes, insert_at);
    let empty = next >= bytes.len() || bytes[next] == b'}';
    let separator = if empty { "" } else { "," };

    format!("{}\n  \"{key}\": {value}{separator}{}", &source[..insert_at], &source[insert_at..])
}

/// Writes the preferences into `source`, changing as few bytes as possible.
pub fn patch_preferences(source: &str, preferences: &Preferences) -> String {
    let Ok(serde_json_lenient::Value::Object(values)) = serde_json_lenient::to_value(preferences)
    else {
        return source.to_string();
    };

    let mut out = source.to_string();
    for (key, value) in values {
        let rendered = value.to_string();
        out = match splice_key(&out, &key, &rendered) {
            Some(patched) => patched,
            None => append_key(&out, &key, &rendered),
        };
    }
    out
}

/// Writes the preferences to their file.
fn save_preferences(preferences: &Preferences) -> std::io::Result<()> {
    let Some(path) = settings_path() else {
        return Ok(());
    };
    let source = match std::fs::read_to_string(&path) {
        Ok(source) if source.contains('{') => source,
        // Nothing usable in it: start from the documented defaults, so the
        // user has something to read.
        Ok(_) => documented_defaults(),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => documented_defaults(),
        // Unreadable for another reason — a permission, a transient failure.
        // Writing here would replace what the user wrote with the defaults,
        // which is the one outcome worse than not saving.
        Err(error) => return Err(error),
    };
    write_atomically(&path, &patch_preferences(&source, preferences))
}

/// Writes the machine state, whole.
pub fn save_state(state: &State) -> std::io::Result<()> {
    let Some(path) = state_path() else {
        return Ok(());
    };
    let body = serde_json_lenient::to_string_pretty(state)
        .map_err(|error| std::io::Error::new(std::io::ErrorKind::InvalidData, error))?;
    write_atomically(&path, &format!("{body}\n"))
}

/// Writes the JSON Schema of the preferences beside them.
///
/// What makes a settings file pleasant to edit is not its format but the
/// completion an editor gives over it, and that comes from a schema. maxx
/// already derives one for its actions, so this costs a call.
fn save_schema() {
    let Some(path) = schema_path() else {
        return;
    };
    let schema = schemars::schema_for!(Preferences);
    if let Ok(body) = serde_json_lenient::to_string_pretty(&schema) {
        let _ = write_atomically(&path, &format!("{body}\n"));
    }
}

/// The old single TOML file, read once and split in two.
#[derive(Default, Deserialize)]
#[serde(default)]
struct LegacyToml {
    recent_projects: Vec<PathBuf>,
    show_project_panel: Option<bool>,
    show_status_bar: Option<bool>,
    show_output: Option<bool>,
    window: Option<WindowGeometry>,
}

/// Imports `settings.toml` from the version that had one, once.
///
/// The old file is renamed rather than deleted: it is the user's, and a
/// migration that eats data is a migration nobody trusts.
fn migrate_from_toml() {
    let (Some(directory), Some(settings)) = (directory(), settings_path()) else {
        return;
    };
    let legacy = directory.join("settings.toml");
    if settings.exists() || !legacy.exists() {
        return;
    }
    let Ok(source) = std::fs::read_to_string(&legacy) else {
        return;
    };
    let Ok(old) = toml::from_str::<LegacyToml>(&source) else {
        return;
    };

    let mut preferences = Preferences::default();
    if let Some(value) = old.show_project_panel {
        preferences.show_project_panel = value;
    }
    if let Some(value) = old.show_status_bar {
        preferences.show_status_bar = value;
    }
    if let Some(value) = old.show_output {
        preferences.show_output = value;
    }
    let _ = save_preferences(&preferences);
    let _ = save_state(&State {
        recent_projects: old.recent_projects,
        window: old.window,
        ..State::default()
    });
    let _ = std::fs::rename(&legacy, legacy.with_extension("toml.repris"));
}

/// The settings of the running application.
struct Store {
    preferences: Preferences,
    state: State,
}

impl Global for Store {}

/// Loads the settings into the application.
pub fn init(cx: &mut App) {
    migrate_from_toml();
    save_schema();

    let preferences = match settings_path() {
        Some(path) => {
            if !path.exists() {
                // Written out so there is something to open and read, the way
                // Zed ships a commented default.
                let _ = write_atomically(&path, &documented_defaults());
            }
            read_json(&path)
        }
        None => Preferences::default(),
    };
    let mut state: State = match state_path() {
        Some(path) => read_json(&path),
        None => State::default(),
    };
    state.forget_missing_projects();

    cx.set_global(Store { preferences, state });
}

/// The preferences as they stand.
pub fn prefs(cx: &App) -> &Preferences {
    &cx.global::<Store>().preferences
}

/// The machine state as it stands.
pub fn state(cx: &App) -> &State {
    &cx.global::<Store>().state
}

/// Changes a preference and writes it, touching only the key that moved.
pub fn update_prefs(cx: &mut App, change: impl FnOnce(&mut Preferences)) {
    let mut preferences = cx.global::<Store>().preferences.clone();
    change(&mut preferences);
    if preferences == cx.global::<Store>().preferences {
        return;
    }
    if let Err(error) = save_preferences(&preferences) {
        eprintln!("settings not saved: {error}");
    }
    cx.global_mut::<Store>().preferences = preferences;
}

/// Changes the machine state and writes it.
pub fn update_state(cx: &mut App, change: impl FnOnce(&mut State)) {
    let mut state = cx.global::<Store>().state.clone();
    change(&mut state);
    if state == cx.global::<Store>().state {
        return;
    }
    if let Err(error) = save_state(&state) {
        eprintln!("state not saved: {error}");
    }
    cx.global_mut::<Store>().state = state;
}

/// Changes the machine state in memory, without touching the disk.
///
/// For what moves continuously — the window being dragged — where writing a
/// file per frame would be absurd. [`flush`] puts it away at quit.
pub fn stage_state(cx: &mut App, change: impl FnOnce(&mut State)) {
    let mut state = cx.global::<Store>().state.clone();
    change(&mut state);
    if state != cx.global::<Store>().state {
        cx.global_mut::<Store>().state = state;
    }
}

/// Writes whatever [`stage_state`] left in memory.
pub fn flush(cx: &App) {
    if let Err(error) = save_state(&cx.global::<Store>().state) {
        eprintln!("state not saved: {error}");
    }
}
