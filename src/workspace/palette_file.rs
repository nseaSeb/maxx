//! The palette page: reading `src/theme.rs` into pickers, and writing back.
//!
//! The colours of the **project**, not of maxx. maxx's own two modes are in
//! `crate::theme` and switch from `View ▸ Light or dark`; what is edited here
//! is the palette the generated application paints with, and it goes to that
//! application's own file.

use gpui::{AppContext as _, Context, Entity, Hsla, Rgba, SharedString, Window};
use gpui_component::color_picker::{ColorPickerEvent, ColorPickerState};

use crate::themefile::{Mode, ThemeFile};
use crate::workspace::Workspace;

/// The colour a role holds, as the picker wants it.
pub fn to_colour(value: u32) -> Hsla {
    gpui::rgb(value).into()
}

/// And back, as the file writes it.
///
/// Through `Rgba` rather than out of the `Hsla` directly: the file speaks in
/// twenty-four bits and the picker in hue, saturation and lightness, so the
/// rounding has to happen once, here, and in the same place both ways — or a
/// colour picked, written and read back would land one unit beside itself and
/// creep on every visit.
pub fn from_colour(colour: Hsla) -> u32 {
    let rgba: Rgba = colour.into();
    let byte = |channel: f32| (channel.clamp(0., 1.) * 255.).round() as u32;
    (byte(rgba.r) << 16) | (byte(rgba.g) << 8) | byte(rgba.b)
}

impl Workspace {
    /// Builds the palette page's pickers, once per project.
    ///
    /// Built here rather than where they are drawn, for the reason every other
    /// panel in maxx has: `SettingItem::render` runs on every repaint, and an
    /// entity created there would be a new one each frame — a picker that shuts
    /// its own popup as soon as it is opened.
    pub(super) fn sync_palette_inputs(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let root = self.project.as_ref().map(|project| project.root.clone());
        if self.palette_synced == root {
            return;
        }
        self.palette_synced = root.clone();
        self.palette_inputs.clear();
        self.palette = None;
        let Some(root) = root else {
            return;
        };

        let Some(file) = ThemeFile::load(&root) else {
            return;
        };
        for swatch in file.swatches() {
            for mode in [Mode::Dark, Mode::Light] {
                let name = swatch.name.clone();
                let colour = to_colour(swatch.value(mode));
                let state = cx.new(|cx| ColorPickerState::new(window, cx).default_value(colour));
                let key = name.clone();
                cx.subscribe(&state, move |this, _, event: &ColorPickerEvent, cx| {
                    // The picker settles on a colour before it says so — it
                    // emits on the click, not while the cursor travels the
                    // gradient — so there is no half-picked value to guard
                    // against here, unlike a text box written per keystroke.
                    let ColorPickerEvent::Change(Some(colour)) = event else {
                        return;
                    };
                    this.set_palette_colour(&key, mode, from_colour(*colour), cx);
                })
                .detach();
                self.palette_inputs.push((name, mode, state));
            }
        }
        self.palette = Some(file);
    }

    /// Writes one colour into `src/theme.rs`.
    fn set_palette_colour(&mut self, name: &str, mode: Mode, value: u32, cx: &mut Context<Self>) {
        let Some(file) = self.palette.as_mut() else {
            return;
        };
        match file.set(name, mode, value) {
            Ok(true) => self.message = Some(crate::tr("message.palette_saved")),
            // Nothing to write: the role is gone from the file, or the value it
            // holds is already the one asked for. Either way the file has just
            // been re-read, so the screen is on it again.
            Ok(false) => {}
            Err(error) => self.message = Some(SharedString::from(error)),
        }
        cx.notify();
    }

    /// The palette of the open project, when it has one.
    pub(crate) fn palette(&self) -> Option<&ThemeFile> {
        self.palette.as_ref()
    }

    /// The pickers of the palette page.
    pub(crate) fn palette_inputs(&self) -> &[(String, Mode, Entity<ColorPickerState>)] {
        &self.palette_inputs
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every byte of every channel survives the trip to the picker and back.
    ///
    /// The picker works in hue, saturation and lightness; the file in
    /// twenty-four bits. A conversion that lost a unit somewhere would show as
    /// a palette that drifts a shade darker each time someone opens the page
    /// and closes it — the kind of thing nobody reports and everybody notices.
    #[test]
    fn a_colour_survives_the_picker() {
        for byte in 0u32..=255 {
            for value in [byte << 16, byte << 8, byte, byte * 0x010101] {
                assert_eq!(from_colour(to_colour(value)), value, "{value:#08x}");
            }
        }
    }

    /// And the values the template ships with, named.
    #[test]
    fn the_template_s_own_colours_survive() {
        for value in
            [0x1e2127, 0xfafafa, 0x22262d, 0x61afef, 0x0969da, 0xe06c75, 0xffffff, 0x000000]
        {
            assert_eq!(from_colour(to_colour(value)), value, "{value:#08x}");
        }
    }
}
