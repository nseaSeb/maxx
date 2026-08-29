//! The colours the canvas paints the project's own content with.
//!
//! The canvas draws two things at once, and they must not share a palette.
//!
//! The **content** — the board standing for the application's window, and the
//! text on it — belongs to the project. It is painted with the roles of that
//! project's `src/theme.rs`, so that choosing a colour in the palette editor
//! shows up where the choice was made, and not only after `cargo run`. That is
//! the whole point: a preview that paints in maxx's greys is a preview of maxx.
//!
//! The **tooling** — the selection outline, the drop zones, the empty-child
//! placeholders — belongs to maxx and keeps maxx's colours. It has to stay
//! legible over whatever the project chose, and a selection outline drawn in the
//! project's accent disappears the moment the project's accent is the colour it
//! sits on.
//!
//! Everything here falls back to [`crate::theme`]. A project with no palette
//! module, a role the developer removed, a file that stopped parsing: the canvas
//! is never left without a colour, because a canvas without colours is a bug
//! that looks like a blank screen.

use gpui::Rgba;

use crate::theme;
use crate::themefile::{Mode, ThemeFile};

/// The palette the canvas paints the project's content with.
///
/// Cloned from the file rather than borrowed: the canvas renders from `&self`
/// while the palette lives behind the same borrow, and a preview is ten small
/// numbers.
#[derive(Clone, Debug, Default)]
pub struct Preview {
    /// The roles read from the project, as `(name, dark, light)`.
    roles: Vec<(String, u32, u32)>,
}

impl Preview {
    /// Reads the palette of the project rooted at `root`.
    ///
    /// Empty when there is none, which is the ordinary case: the palette is a
    /// module added on demand, and a project without one is not a project in
    /// trouble.
    pub fn read(root: &std::path::Path) -> Self {
        let Some(file) = ThemeFile::load(root) else {
            return Self::default();
        };
        Self::from_file(&file)
    }

    /// The same, from a palette already in hand.
    pub fn from_file(file: &ThemeFile) -> Self {
        Self {
            roles: file
                .swatches()
                .iter()
                .map(|swatch| (swatch.name.clone(), swatch.dark, swatch.light))
                .collect(),
        }
    }

    /// Whether the project told us anything at all.
    pub fn is_empty(&self) -> bool {
        self.roles.is_empty()
    }

    /// The value of one role, in the mode maxx is currently showing.
    ///
    /// The project's palette has two modes and so does maxx; the canvas follows
    /// maxx's, because the alternative is a preview in the light mode inside a
    /// dark window, which reads as a bug rather than as a preview.
    fn role(&self, name: &str, mode: Mode) -> Option<u32> {
        self.roles.iter().find(|(this, _, _)| this == name).map(|(_, dark, light)| match mode {
            Mode::Dark => *dark,
            Mode::Light => *light,
        })
    }

    /// A role in one mode, or maxx's own colour when the project lacks it.
    ///
    /// The mode is a parameter and not read here, so that a test can ask for
    /// one without touching `theme::set_dark` — a process-wide global that the
    /// test beside it is reading at the same moment, since Rust runs the tests
    /// of one binary in one process.
    fn in_mode(&self, name: &str, mode: Mode, fallback: fn() -> Rgba) -> Rgba {
        self.role(name, mode).map(gpui::rgb).unwrap_or_else(fallback)
    }

    /// The same, in the mode maxx is currently showing.
    fn or(&self, name: &str, fallback: fn() -> Rgba) -> Rgba {
        let mode = if theme::is_dark() { Mode::Dark } else { Mode::Light };
        self.in_mode(name, mode, fallback)
    }

    /// The board: what the application's window will be filled with.
    pub fn bg(&self) -> Rgba {
        self.or("BACKGROUND", theme::bg)
    }

    /// The text drawn on it.
    pub fn text(&self) -> Rgba {
        self.or("TEXT", theme::text)
    }

    /// Secondary text.
    pub fn text_muted(&self) -> Rgba {
        self.or("TEXT_MUTED", theme::text_muted)
    }

    /// A separator inside the previewed content.
    pub fn border(&self) -> Rgba {
        self.or("BORDER", theme::border)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn preview_of(source: &str) -> Preview {
        let root = std::env::temp_dir().join(format!(
            "maxx-preview-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("src")).unwrap();
        std::fs::write(ThemeFile::path_in(&root), source).unwrap();
        let preview = Preview::read(&root);
        let _ = std::fs::remove_dir_all(&root);
        preview
    }

    /// A project with a palette hands its own colours to the canvas.
    #[test]
    fn the_board_takes_the_project_s_background() {
        let preview = preview_of(
            "/// Background of the main area.\n\
             pub const BACKGROUND: Role = Role { dark: 0x001122, light: 0xffeedd };\n",
        );
        assert_eq!(preview.in_mode("BACKGROUND", Mode::Dark, theme::bg), gpui::rgb(0x001122));
        assert_eq!(
            preview.in_mode("BACKGROUND", Mode::Light, theme::bg),
            gpui::rgb(0xffeedd),
            "and the two modes are not the same colour"
        );
    }

    /// A role the project does not have falls back rather than going blank.
    ///
    /// The failure this guards against is a canvas painted with nothing, which
    /// looks like a broken window rather than like a missing colour.
    #[test]
    fn a_missing_role_falls_back_to_maxx_s_own() {
        let preview =
            preview_of("pub const BACKGROUND: Role = Role { dark: 0x001122, light: 0xffeedd };\n");
        assert_eq!(
            preview.in_mode("TEXT", Mode::Dark, theme::text),
            theme::text(),
            "TEXT is not in that file"
        );
        assert_eq!(preview.in_mode("BORDER", Mode::Dark, theme::border), theme::border());
    }

    /// And a project with no palette at all paints exactly as before.
    #[test]
    fn no_palette_is_maxx_s_palette() {
        let preview = Preview::default();
        assert!(preview.is_empty());
        assert_eq!(preview.in_mode("BACKGROUND", Mode::Dark, theme::bg), theme::bg());
        assert_eq!(preview.in_mode("TEXT", Mode::Light, theme::text), theme::text());
    }
}
