//! The `settings` module: a project's own settings, read and written as JSON.

use std::io;
use std::path::Path;

use super::modules::{header_end, joined, legacy_copy, module_version};
use super::system::add_system_module;
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
pub(super) fn add_dependencies(root: &Path, crates: &[(&str, &str)]) -> io::Result<()> {
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
pub(super) fn dependencies_section(source: &str) -> io::Result<usize> {
    source.lines().position(|line| line.trim() == "[dependencies]").ok_or_else(|| {
        io::Error::new(io::ErrorKind::InvalidData, "Cargo.toml: no [dependencies] section")
    })
}

/// The settings module of a generated project.
///
/// The same discipline maxx applies to its own: JSON with comments, only the
/// changed key rewritten, a documented default file. It is a copy, and a copy
/// is a debt — a defect found on one side has to be carried to the other.
pub(super) fn settings_rs() -> String {
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
