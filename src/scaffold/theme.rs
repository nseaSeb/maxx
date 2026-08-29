//! The `theme` module: the palette a generated project paints with.

use std::io;
use std::path::Path;

use super::modules::{header_end, joined, legacy_copy, module_version};
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
    // Their colours if they have chosen any, the template's otherwise. The
    // project owns the copy either way: what is recorded in `maxx.toml` is the
    // fingerprint of what was actually written, so a project started from a
    // custom palette is still a project maxx recognises as its own.
    let body = match crate::settings::palette_roles() {
        Some(roles) => crate::themefile::apply_roles(&theme_rs(), &roles),
        None => theme_rs(),
    };
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
pub(super) fn theme_rs() -> String {
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
