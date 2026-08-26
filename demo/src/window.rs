//! Where the window was when the application last closed.
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
