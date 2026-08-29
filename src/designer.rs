//! The designer: canvas, tree, inspector and palette, each in its own module.
//!
//! These are render methods on [`Workspace`] rather than a separate view, so
//! that the tree stays the single source and every panel is recomputed from it
//! on each frame — a panel can never hold a stale copy of the model.

use gpui::prelude::*;
use gpui::{AnyElement, Context, SharedString, div, px};
use gpui_component::scroll::Scrollbar;
use gpui_component::{h_flex, v_flex};

use crate::model::Path;
use crate::theme;
use gpui::Window;
use gpui_component::resizable::{h_resizable, resizable_panel, v_resizable};

use crate::workspace::{Center, Workspace, fillable};

mod canvas;
mod inspector;
mod menus;
mod palette;
mod palette_editor;
mod tree;

pub(crate) use canvas::missing_image;
pub use palette::{label_matches, matches_query};

impl Workspace {
    /// The width the inspector should take, and the place where the width the
    /// handle left is picked back up.
    ///
    /// Kept in memory only, like the window geometry: writing a file on every
    /// frame of a drag would be absurd, and `settings::flush` puts it away at
    /// quit.
    fn inspector_width(&self, cx: &mut Context<Self>) -> f32 {
        if let Some(largeur) = self.inspector_split.read(cx).sizes().last().copied() {
            let largeur = f32::from(largeur);
            if largeur > 0. {
                crate::settings::stage_state(cx, |state| state.inspector_width = Some(largeur));
                return largeur;
            }
        }
        crate::settings::state(cx).inspector_width.unwrap_or(280.)
    }

    /// The designer, or an invitation to open a view.
    pub(crate) fn render_designer(&self, cx: &mut Context<Self>) -> AnyElement {
        // Matched, not a chain of `if`s asking each mode whether it has
        // something. That chain is how the reader came to be drawn for as long
        // as a file was open, whatever the mode said — and how the tab strip,
        // drawn above every one of these precisely to be the way back, stopped
        // being one. A `match` on the mode cannot be written from a document,
        // and a mode added later has to be answered here.
        //
        // The tab strip stays in every arm: it is the way back to an open view.
        let middle = match self.center() {
            Center::Preferences => Some(self.render_preferences(cx)),
            Center::Palette(_) => Some(self.render_palette_editor(cx).into_any_element()),
            Center::Menus(_) => Some(self.render_menu_editor(cx)),
            // The one arm with a condition, and it is about the document rather
            // than the mode: asked to show a reader holding nothing, the middle
            // shows the view instead of an empty frame.
            Center::Code => self.code().is_some().then(|| self.render_code(cx)),
            Center::Designer => None,
        };
        if let Some(middle) = middle {
            return v_flex()
                .flex_1()
                .overflow_hidden()
                .child(self.render_tabs(cx))
                .child(middle)
                .into_any_element();
        }
        if self.view().is_none() {
            return div()
                .flex()
                .flex_1()
                .items_center()
                .justify_center()
                .text_color(theme::text_muted())
                .child(crate::tr("designer.open_a_view"))
                .into_any_element();
        }

        v_flex()
            .flex_1()
            .overflow_hidden()
            .child(self.render_tabs(cx))
            .child(
                // Not `h_flex`: it centres its children vertically, which would
                // leave the side panel floating in the middle of the window.
                div().flex().flex_row().flex_1().overflow_hidden().child(
                    h_resizable(crate::tr("designer.inspector"))
                        .with_state(&self.inspector_split)
                        .child(
                            resizable_panel()
                                .child(crate::workspace::fillable(self.render_canvas(cx))),
                        )
                        .child(
                            resizable_panel()
                                .size(px(self.inspector_width(cx)))
                                // Below this the inspector's fields fold in
                                // on themselves; beyond it there is no
                                // canvas left to draw.
                                .size_range(px(220.)..px(560.))
                                .child(crate::workspace::fillable(self.render_side_panels(cx))),
                        ),
                ),
            )
            .into_any_element()
    }

    /// One tab per open view, plus the file the code reader holds.
    fn render_tabs(&self, cx: &mut Context<Self>) -> impl IntoElement {
        // A file opened on its own takes the light; a view seen as code keeps
        // its own tab lit, because it is the same document either way.
        let reading = self.code().is_some_and(|file| !file.of_view);
        let tabs: Vec<(usize, SharedString, bool, bool)> = self
            .open_views()
            .iter()
            .enumerate()
            .map(|(index, view)| {
                (
                    index,
                    SharedString::from(view.name()),
                    !reading && self.active_index() == Some(index),
                    view.dirty(),
                )
            })
            .collect();
        // Never dirty: the reader does not write.
        let read_tab = self.code().filter(|file| !file.of_view).map(|file| file.name());

        div()
            .flex()
            .flex_row()
            .h(px(28.))
            .flex_none()
            .bg(theme::panel_bg())
            .border_b_1()
            .border_color(theme::border())
            .children(tabs.into_iter().map(|(index, name, active, dirty)| {
                h_flex()
                    .id(SharedString::from(format!("tab-{index}")))
                    .gap_1()
                    .px_3()
                    .cursor_pointer()
                    .bg(if active { theme::bg() } else { theme::panel_bg() })
                    .border_r_1()
                    .border_color(theme::border())
                    .text_xs()
                    .text_color(if active { theme::text() } else { theme::text_muted() })
                    .child(name)
                    .when(dirty, |this| this.child("•"))
                    .child(
                        div()
                            .id(SharedString::from(format!("tab-close-{index}")))
                            .px_1()
                            .rounded_sm()
                            .hover(|this| this.bg(theme::hover_bg()))
                            .child("×")
                            .on_click(cx.listener(move |this, _, _, cx| {
                                cx.stop_propagation();
                                this.close_view(index, cx);
                            })),
                    )
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.activate_view(index, cx);
                    }))
            }))
            .children(read_tab.map(|name| {
                h_flex()
                    .id("tab-code")
                    .gap_1()
                    .px_3()
                    .cursor_pointer()
                    .bg(theme::bg())
                    .border_r_1()
                    .border_color(theme::border())
                    .text_xs()
                    .text_color(theme::text())
                    .child(name)
                    .child(
                        div()
                            .id("tab-code-close")
                            .px_1()
                            .rounded_sm()
                            .hover(|this| this.bg(theme::hover_bg()))
                            .child("×")
                            .on_click(cx.listener(|this, _, _, cx| {
                                cx.stop_propagation();
                                this.close_code(cx);
                            })),
                    )
                    .on_click(cx.listener(|this, _, _, cx| this.activate_code(cx)))
            }))
    }

    /// Tree, inspector and palette, stacked on the right.
    ///
    /// The three sections together are taller than the window as soon as a view
    /// has a few nodes, so the column scrolls and carries a visible bar.
    fn render_side_panels(&self, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            // No width here: the resizable panel is what gives it, and a fixed
            // width inside would fight with the handle.
            .size_full()
            .flex()
            .border_l_1()
            .border_color(theme::border())
            .bg(theme::panel_bg())
            .child(
                // Two panes with a handle between them, and each with its own
                // scroll. One column meant that selecting a node deep in the
                // tree pushed the tree out of sight — at the moment you most
                // want to see where you are — and that one scrollbar served
                // five unrelated things.
                v_resizable("cote")
                    .with_state(&self.side_split)
                    .child(
                        resizable_panel()
                            .size(px(220.))
                            // Below this the tree shows two rows; beyond it, it
                            // leaves the inspector nothing.
                            .size_range(px(80.)..px(560.))
                            .child(fillable(
                                div()
                                    .relative()
                                    .size_full()
                                    .child(
                                        div()
                                            .id("side-structure")
                                            .size_full()
                                            .overflow_y_scroll()
                                            .track_scroll(&self.tree_scroll)
                                            .child(self.render_tree(cx)),
                                    )
                                    .child(
                                        div()
                                            .absolute()
                                            .top_0()
                                            .right_0()
                                            .bottom_0()
                                            .child(Scrollbar::vertical(&self.tree_scroll)),
                                    ),
                            )),
                    )
                    .child(
                        resizable_panel().child(fillable(
                            div()
                                .relative()
                                .size_full()
                                .child(
                                    div()
                                        .id("side-panels")
                                        .size_full()
                                        .overflow_y_scroll()
                                        .track_scroll(&self.side_scroll)
                                        .child(
                                            v_flex()
                                                .child(self.render_inspector(cx))
                                                .child(self.render_view_name(cx))
                                                .child(self.render_state(cx)),
                                        ),
                                )
                                .child(
                                    div()
                                        .absolute()
                                        .top_0()
                                        .right_0()
                                        .bottom_0()
                                        .child(Scrollbar::vertical(&self.side_scroll)),
                                ),
                        )),
                    ),
            )
    }
}

/// What is being dragged across the canvas.
#[derive(Clone, Debug)]
pub enum Dragged {
    /// A node already in the tree, identified by its path.
    Node(Path),
    /// A component from the palette, identified by its catalogue id.
    Component(&'static str),
}

/// The label that follows the cursor during a drag.
pub struct DragGhost {
    label: SharedString,
}

impl Render for DragGhost {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .px_2()
            .py_1()
            .rounded_sm()
            .bg(theme::accent())
            .text_color(theme::on_accent())
            .text_xs()
            .child(self.label.clone())
    }
}

/// Section header inside the right-hand panels.
pub(super) fn section_title(key: &'static str) -> impl IntoElement {
    div()
        .px_3()
        .py_2()
        .text_xs()
        .text_color(theme::text_muted())
        .border_t_1()
        .border_color(theme::border())
        .child(crate::tr(key))
}
