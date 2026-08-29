//! The palette editor: a project's `src/theme.rs`, drawn as its roles.
//!
//! A mode of the middle panel, opened by selecting `src/theme.rs` in the
//! project panel — exactly the way `src/menus.rs` opens the menu editor. It was
//! a page of the preferences screen first, and that was wrong twice over: these
//! are the colours of *this window's project*, not of the person using maxx, and
//! `⌘,` is where you go to set maxx up, not to open one of a project's files.

use gpui::prelude::*;
use gpui::{AnyElement, Context, SharedString, div, px};
use gpui_component::color_picker::ColorPicker;
use gpui_component::{Sizable as _, h_flex, v_flex};

use crate::theme;
use crate::themefile::Mode;
use crate::workspace::Workspace;

impl Workspace {
    /// The palette of the open project, one row per role.
    pub(super) fn render_palette_editor(&self, _cx: &mut Context<Self>) -> AnyElement {
        let Some(palette) = self.palette.as_ref() else {
            return div().into_any_element();
        };

        let picker = |name: &str, mode: Mode| -> AnyElement {
            let state =
                self.palette_inputs().iter().find(|(this, that, _)| this == name && *that == mode);
            match state {
                Some((_, _, state)) => ColorPicker::new(state).small().into_any_element(),
                // The frame between the file opening and `sync_palette_inputs`
                // building the pickers: one repaint, and an empty box is better
                // than a picker built here that would be a new one every frame.
                None => div().w(px(28.)).h(px(28.)).into_any_element(),
            }
        };

        let mut rows = v_flex().gap_1().p_3();
        for swatch in palette.swatches() {
            let name = SharedString::from(swatch.name.clone());
            let doc = SharedString::from(swatch.doc.clone());
            rows = rows.child(
                h_flex()
                    .gap_3()
                    .items_center()
                    .px_2()
                    .py_1p5()
                    .rounded_md()
                    .hover(|this| this.bg(theme::hover_bg()))
                    .child(picker(&swatch.name, Mode::Dark))
                    .child(picker(&swatch.name, Mode::Light))
                    .child(
                        v_flex()
                            .flex_1()
                            .child(div().child(name))
                            .child(div().text_xs().text_color(theme::text_muted()).child(doc)),
                    ),
            );
        }

        v_flex()
            .flex_1()
            .overflow_hidden()
            .child(
                // What the two columns are, said once at the top rather than on
                // every row: ten rows carrying the same two words would be ten
                // times the noise for the same information.
                h_flex()
                    .gap_3()
                    .items_center()
                    .px_5()
                    .pt_3()
                    .text_xs()
                    .text_color(theme::text_muted())
                    .child(div().w(px(28.)).child(crate::tr("prefs.palette_dark")))
                    .child(div().w(px(28.)).child(crate::tr("prefs.palette_light")))
                    .child(div().flex_1().child(crate::tr("prefs.palette_desc"))),
            )
            .child(div().id("palette").flex_1().overflow_y_scroll().child(rows))
            .into_any_element()
    }
}
