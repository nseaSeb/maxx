//! The palette page: reading `src/theme.rs` into pickers, and writing back.
//!
//! The colours of the **project**, not of maxx. maxx's own two modes are in
//! `crate::theme` and switch from `View ▸ Light or dark`; what is edited here
//! is the palette the generated application paints with, and it goes to that
//! application's own file.

use gpui::{AppContext as _, Context, Entity, Hsla, Rgba, SharedString, Window};
use gpui_component::color_picker::{ColorPickerEvent, ColorPickerState};
use rust_i18n::t;

use crate::themefile::{Mode, ThemeFile};
use crate::workspace::{Center, Workspace};

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
    /// Opens the palette new projects start from, creating it if it is not
    /// there yet.
    ///
    /// The same editor as a project's own, on a file that lives in maxx's
    /// directory rather than in a project. One reader, one editor, one writer
    /// for both — the alternative was a second screen doing the same thing to
    /// the same shape of file.
    pub fn open_default_palette(&mut self, cx: &mut Context<Self>) {
        let path = match crate::settings::palette_path() {
            Some(path) if path.exists() => path,
            Some(_) => match crate::settings::create_palette() {
                Ok(path) => path,
                Err(error) => {
                    self.message = Some(SharedString::from(error));
                    cx.notify();
                    return;
                }
            },
            None => {
                self.message = Some(crate::tr("error.no_config_directory"));
                cx.notify();
                return;
            }
        };
        match ThemeFile::open(&path) {
            Some(palette) => self.show(Center::Palette(palette)),
            // 4. Named, because the way out is to open or delete that file and
            // an error that does not say which file is a dead end: the button
            // goes on offering to edit what cannot be read.
            None => {
                self.message = Some(SharedString::from(
                    t!("error.palette_unreadable", path = path.display().to_string()).into_owned(),
                ))
            }
        }
        cx.notify();
    }

    /// Builds the palette page's pickers, once per project.
    ///
    /// Built here rather than where they are drawn, for the reason every other
    /// panel in maxx has: `SettingItem::render` runs on every repaint, and an
    /// entity created there would be a new one each frame — a picker that shuts
    /// its own popup as soon as it is opened.
    pub(super) fn sync_palette_inputs(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let key = self.palette().map(|palette| palette.path.clone());
        if self.palette_synced == key {
            return;
        }
        self.palette_synced = key;
        self.palette_inputs.clear();

        let Some(file) = self.palette().cloned() else {
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
    }

    /// Writes one colour into `src/theme.rs`.
    fn set_palette_colour(&mut self, name: &str, mode: Mode, value: u32, cx: &mut Context<Self>) {
        // Read before the palette is borrowed: what the canvas paints depends
        // on whether this file belongs to the open project.
        let root = self.project.as_ref().map(|project| project.root.clone());
        let root_for_record = root.clone();
        let Some(file) = self.palette_mut() else {
            return;
        };
        // The whole list, values included, and not just the names. `set`
        // re-reads the file, so it silently absorbs whatever was written there
        // since — a colour changed in Zed between the watcher's last tick and
        // this click. Comparing names alone leaves that picker showing the
        // colour from before the edit, for good: the later `reload` finds the
        // source already up to date and never rebuilds anything.
        let before = file.roles();
        let outcome = file.set(name, mode, value);
        let after = file.roles();
        // Everything except the one just written: that one moved on purpose,
        // and its picker is the one being used.
        let elsewhere = before.len() != after.len()
            || before
                .iter()
                .zip(&after)
                .any(|(was, now)| was != now && !(was.0 == name && now.0 == name));
        // The canvas paints from its own copy, so a colour just written has to
        // reach it — but only when the palette being written *is* the project's.
        // The same editor also serves the user's own default palette, and
        // painting the open project with those would be showing colours no file
        // of it holds, until some unrelated event happened to correct it.
        let preview = root
            .filter(|root| file.path.starts_with(root))
            .map(|_| crate::preview::Preview::from_file(file));
        // The fingerprint follows what was just written. Without this, choosing
        // a colour in maxx made the file stop matching what maxx had recorded,
        // so the project silently left the update mechanism — and the branch
        // that keeps a project's colours through an update could never run for
        // the very people it was written for. An edit made in Zed still counts
        // as the developer taking the file over; one made here does not.
        if matches!(outcome, Ok(true))
            && let Some(root) = root_for_record
        {
            let _ = crate::projectfile::record(
                &root,
                "theme",
                crate::scaffold::module_version("theme").unwrap_or(1),
                file.source(),
            );
        }
        if elsewhere {
            self.palette_synced = None;
        }
        if let Some(preview) = preview {
            self.preview = preview;
        }
        match outcome {
            Ok(true) => self.message = Some(crate::tr("message.palette_saved")),
            // Nothing to write: the role is gone from the file, or the value it
            // holds is already the one asked for. Either way the file has just
            // been re-read, so the screen is on it again.
            Ok(false) => {}
            Err(error) => self.message = Some(SharedString::from(error)),
        }
        cx.notify();
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
