//! The `window` module: window geometry, remembered between runs.

use std::io;
use std::path::Path;

use super::identifier;
use super::modules::{header_end, joined, legacy_copy, module_version};
use super::settings::{add_dependencies, dependencies_section};
use super::system::add_system_module;
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
pub(super) fn window_rs() -> String {
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
