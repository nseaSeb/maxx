//! The `system` module: where a project's files live, and its wastebasket.

use std::io;
use std::path::Path;

use super::modules::{header_end, joined, legacy_copy, module_version};
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

/// The system module of a generated project.
///
/// Only what actually differs from one system to the next *and* is not already
/// in gpui. The clipboard, opening a URL, revealing a file, the file pickers:
/// gpui has all of them (`cx.write_to_clipboard`, `cx.open_url`,
/// `cx.reveal_path`, `cx.open_with_system`, `cx.prompt_for_paths`), and
/// wrapping them would be noise. What is left is where a system puts an
/// application's files, and what it calls its trash.
pub(super) fn system_rs() -> String {
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
