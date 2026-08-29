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

/// Width of a swatch cell, header included.
///
/// One constant for the two, because a header that does not sit over its column
/// is worse than no header: it points at the wrong thing with confidence.
const CELL: f32 = 56.;

impl Workspace {
    /// The palette of the open project, one row per role.
    pub(super) fn render_palette_editor(&self, _cx: &mut Context<Self>) -> AnyElement {
        let Some(palette) = self.palette.as_ref() else {
            return div().into_any_element();
        };

        let picker = |name: &str, mode: Mode| -> AnyElement {
            let state =
                self.palette_inputs().iter().find(|(this, that, _)| this == name && *that == mode);
            // Boxed with a border of its own. A swatch is a flat square of the
            // colour it holds, so `BACKGROUND` in the dark mode is very nearly
            // the colour of the panel behind it and simply disappears — the one
            // role you cannot see is the one you most want to change.
            let cell = div()
                .w(px(CELL))
                .flex()
                .justify_center()
                .rounded_md()
                .border_1()
                .border_color(theme::border())
                .p_0p5();
            match state {
                Some((_, _, state)) => {
                    cell.child(ColorPicker::new(state).small()).into_any_element()
                }
                // The frame between the file opening and `sync_palette_inputs`
                // building the pickers: one repaint, and an empty box is better
                // than a picker built here that would be a new one every frame.
                None => cell.h(px(CELL / 2.)).into_any_element(),
            }
        };

        let column = |key: &'static str| {
            div()
                .w(px(CELL))
                .flex()
                .justify_center()
                .text_xs()
                .whitespace_nowrap()
                .text_color(theme::text_muted())
                .child(crate::tr(key))
        };

        // The same gap and the same horizontal padding as a row, so the two
        // headings stand over the two columns they name.
        let heading = h_flex()
            .gap_3()
            .items_center()
            .px_2()
            .pb_1()
            .child(column("prefs.palette_dark"))
            .child(column("prefs.palette_light"));

        let mut rows = v_flex().gap_1().px_3().pb_3().child(heading);
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
                            .min_w_0()
                            .child(div().child(name))
                            .child(div().text_xs().text_color(theme::text_muted()).child(doc)),
                    ),
            );
        }

        v_flex()
            .flex_1()
            .overflow_hidden()
            .child(
                // The file, then what editing it does. On its own lines and not
                // in the column heading: the sentence is long enough to be cut
                // off by the right edge when it shares a row with anything.
                v_flex()
                    .gap_1()
                    .px_5()
                    .py_3()
                    .border_b_1()
                    .border_color(theme::border())
                    .child(
                        div().child(SharedString::from(
                            palette
                                .path
                                .file_name()
                                .map(|name| name.to_string_lossy().into_owned())
                                .unwrap_or_default(),
                        )),
                    )
                    .child(
                        div()
                            .w_full()
                            .text_xs()
                            .text_color(theme::text_muted())
                            .child(crate::tr("prefs.palette_desc")),
                    ),
            )
            .child(div().id("palette").flex_1().overflow_y_scroll().child(rows))
            .into_any_element()
    }
}
