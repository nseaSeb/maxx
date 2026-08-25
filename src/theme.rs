//! The application's palette, in two modes.
//!
//! Loosely based on Zed's One Dark, with a light counterpart of the same
//! shape — the same nine roles, so a call site never has to know which mode is
//! on.
//!
//! The mode is a process-wide value read without a context. That is not a
//! shortcut around threading one: a colour is asked for from inside closures,
//! from free functions and from element builders that have no `cx` to give,
//! and the mode is genuinely one value for the whole application — the same
//! shape `gpui_component::Theme` and the current locale already have.

use std::sync::atomic::{AtomicBool, Ordering};

use gpui::Rgba;

/// Whether the dark palette is the one in use.
static DARK: AtomicBool = AtomicBool::new(true);

/// Whether maxx is drawing in the dark.
pub fn is_dark() -> bool {
    DARK.load(Ordering::Relaxed)
}

/// Switches the palette. Redrawing is the caller's business.
pub fn set_dark(dark: bool) {
    DARK.store(dark, Ordering::Relaxed);
}

/// One role of the palette, in both modes.
struct Colour {
    dark: u32,
    light: u32,
}

impl Colour {
    /// The value for the mode in use.
    fn get(&self) -> Rgba {
        gpui::rgb(if is_dark() { self.dark } else { self.light })
    }
}

/// Background of the main editor area.
pub fn bg() -> Rgba {
    Colour { dark: 0x1e2127, light: 0xfafafa }.get()
}

/// Background of the titlebar.
pub fn titlebar_bg() -> Rgba {
    Colour { dark: 0x282c34, light: 0xf0f0f0 }.get()
}

/// Background of the project panel and the status bar.
pub fn panel_bg() -> Rgba {
    Colour { dark: 0x22262d, light: 0xf3f3f3 }.get()
}

/// Background of a hovered list row.
pub fn hover_bg() -> Rgba {
    Colour { dark: 0x2c313a, light: 0xe8e8e8 }.get()
}

/// Background of the selected list row.
pub fn selected_bg() -> Rgba {
    Colour { dark: 0x3a4048, light: 0xdcdcdc }.get()
}

/// Separator colour between the main regions of the window.
pub fn border() -> Rgba {
    Colour { dark: 0x2f343d, light: 0xdfdfdf }.get()
}

/// Default foreground colour.
pub fn text() -> Rgba {
    Colour { dark: 0xc8ccd4, light: 0x24292f }.get()
}

/// Foreground colour for secondary information.
pub fn text_muted() -> Rgba {
    Colour { dark: 0x7f8896, light: 0x6b7280 }.get()
}

/// Accent colour, used for directories and the primary button.
pub fn accent() -> Rgba {
    Colour { dark: 0x61afef, light: 0x0969da }.get()
}

/// Foreground colour on top of the accent colour.
pub fn on_accent() -> Rgba {
    Colour { dark: 0x1e2127, light: 0xffffff }.get()
}

/// The colour of a failure, in the output panel and the status bar.
pub fn danger() -> Rgba {
    Colour { dark: 0xe06c75, light: 0xcf222e }.get()
}
