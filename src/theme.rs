//! Colour palette for the application, loosely based on Zed's One Dark theme.
//!
//! Colours are stored as `u32` literals so they can live in `const` items; wrap
//! them with [`gpui::rgb`] at the call site.

/// Background of the main editor area.
pub const BG: u32 = 0x1e2127;
/// Background of the titlebar.
pub const TITLEBAR_BG: u32 = 0x282c34;
/// Background of the project panel and the status bar.
pub const PANEL_BG: u32 = 0x22262d;
/// Background of a hovered list row.
pub const HOVER_BG: u32 = 0x2c313a;
/// Background of the selected list row.
pub const SELECTED_BG: u32 = 0x3a4048;
/// Separator colour between the main regions of the window.
pub const BORDER: u32 = 0x2f343d;
/// Default foreground colour.
pub const TEXT: u32 = 0xc8ccd4;
/// Foreground colour for secondary information.
pub const TEXT_MUTED: u32 = 0x7f8896;
/// Accent colour, used for directories and the primary button.
pub const ACCENT: u32 = 0x61afef;
/// Foreground colour on top of the accent colour.
pub const ON_ACCENT: u32 = 0x1e2127;
