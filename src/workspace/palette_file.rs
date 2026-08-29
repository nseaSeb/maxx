//! The palette page: reading `src/theme.rs` into boxes, and writing back.
//!
//! The colours of the **project**, not of maxx. maxx's own two modes are in
//! `crate::theme` and switch from `View ▸ Light or dark`; what is edited here
//! is the palette the generated application paints with, and it goes to that
//! application's own file.

use gpui::{AppContext as _, Context, Entity, Window};
use gpui_component::input::{InputEvent, InputState};

use crate::themefile::{Mode, ThemeFile};
use crate::workspace::Workspace;

/// The spelling a colour is typed in, and read back from.
///
/// Six digits and a leading `#`, because that is what a designer hands over and
/// what every other tool shows. The file keeps `0x`, which is what Rust reads;
/// the two spellings meet here and nowhere else.
pub fn format_colour(value: u32) -> String {
    format!("#{value:06x}")
}

/// Reads a typed colour, accepting the spellings people actually use.
///
/// `#1e2127`, `1e2127`, `0x1e2127`, and the three-digit short form the web made
/// everyone fluent in. Anything else is refused rather than guessed: a value
/// half-typed is not a colour, and writing one on every keystroke would put the
/// project through every state on the way to it.
pub fn parse_colour(text: &str) -> Option<u32> {
    let digits = text
        .trim()
        .trim_start_matches('#')
        .trim_start_matches("0x")
        .trim_start_matches("0X")
        .trim();
    if !digits.chars().all(|c| c.is_ascii_hexdigit()) {
        return None;
    }
    match digits.len() {
        6 => u32::from_str_radix(digits, 16).ok(),
        // `#abc` is `#aabbcc`, the rule the web taught everyone.
        3 => {
            let mut full = String::with_capacity(6);
            for c in digits.chars() {
                full.push(c);
                full.push(c);
            }
            u32::from_str_radix(&full, 16).ok()
        }
        _ => None,
    }
}

impl Workspace {
    /// Builds the palette page's boxes, once per project.
    ///
    /// Built here rather than where they are drawn, for the reason every other
    /// panel in maxx has: `SettingItem::render` runs on every repaint, and an
    /// entity created there would be a new one each frame — a field that loses
    /// the caret as soon as it is typed in.
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
                let value = format_colour(swatch.value(mode));
                let state = cx.new(|cx| InputState::new(window, cx).default_value(value));
                let key = name.clone();
                cx.subscribe(&state, move |this, state, event: &InputEvent, cx| match event {
                    // Written when the field is left, not on every keystroke:
                    // `#1e21` is a state no project should be put through, and
                    // a source file rewritten per character is a disk write per
                    // character.
                    InputEvent::Blur | InputEvent::PressEnter { .. } => {
                        let text = state.read(cx).value().to_string();
                        this.set_palette_colour(&key, mode, &text, cx);
                    }
                    _ => {}
                })
                .detach();
                self.palette_inputs.push((name, mode, state));
            }
        }
        self.palette = Some(file);
    }

    /// Writes one colour into `src/theme.rs`.
    ///
    /// A refused spelling is said and left in the box rather than wiped: what
    /// was typed is nearly always one character away from a colour, and taking
    /// it away would make the correction start from nothing. Nothing is written
    /// in that case, so the file and the box disagree until it is fixed — which
    /// is what the message is for.
    fn set_palette_colour(&mut self, name: &str, mode: Mode, text: &str, cx: &mut Context<Self>) {
        let Some(file) = self.palette.as_mut() else {
            return;
        };
        let current = file
            .swatches()
            .iter()
            .find(|swatch| swatch.name == name)
            .map(|swatch| swatch.value(mode));
        let Some(current) = current else {
            return;
        };

        match parse_colour(text) {
            Some(value) if value != current => {
                if file.set(name, mode, value) {
                    match file.save() {
                        Ok(()) => self.message = Some(crate::tr("message.palette_saved")),
                        Err(error) => self.message = Some(error.into()),
                    }
                }
            }
            Some(_) => return,
            None => self.message = Some(crate::tr("error.bad_colour")),
        }
        cx.notify();
    }

    /// The palette of the open project, when it has one.
    pub(crate) fn palette(&self) -> Option<&ThemeFile> {
        self.palette.as_ref()
    }

    /// The boxes of the palette page.
    pub(crate) fn palette_inputs(&self) -> &[(String, Mode, Entity<InputState>)] {
        &self.palette_inputs
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_spellings_people_use_are_all_read() {
        assert_eq!(parse_colour("#1e2127"), Some(0x1e2127));
        assert_eq!(parse_colour("1e2127"), Some(0x1e2127));
        assert_eq!(parse_colour("0x1e2127"), Some(0x1e2127));
        assert_eq!(parse_colour("  #1E2127 "), Some(0x1e2127));
        assert_eq!(parse_colour("#abc"), Some(0xaabbcc), "the short form the web taught");
    }

    #[test]
    fn a_half_typed_value_is_refused_rather_than_guessed() {
        assert_eq!(parse_colour("#1e21"), None);
        assert_eq!(parse_colour(""), None);
        assert_eq!(parse_colour("#12345g"), None);
        assert_eq!(parse_colour("rebeccapurple"), None);
    }

    #[test]
    fn a_colour_is_shown_the_way_it_is_read() {
        for value in [0x000000, 0x1e2127, 0xffffff, 0x0969da] {
            assert_eq!(parse_colour(&format_colour(value)), Some(value));
        }
        assert_eq!(format_colour(0x0969da), "#0969da", "the leading zero is kept");
    }
}
