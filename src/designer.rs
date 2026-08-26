//! The designer: canvas, tree, inspector and palette.
//!
//! These are render methods on [`Workspace`] rather than a separate view, so
//! that the tree stays the single source and every panel is recomputed from it
//! on each frame — a panel can never hold a stale copy of the model.

use rust_i18n::t;

use gpui::prelude::*;
use gpui::{AnyElement, Context, Div, SharedString, div, img, px};
use gpui_component::alert::Alert;
use gpui_component::button::Button;
use gpui_component::checkbox::Checkbox;
use gpui_component::divider::Divider;
use gpui_component::group_box::GroupBox;
use gpui_component::input::Input;
use gpui_component::label::Label;
use gpui_component::scroll::Scrollbar;
use gpui_component::switch::Switch;
use gpui_component::{Sizable as _, h_flex, v_flex};

use crate::menu_model::ItemDef;
use crate::menufile::{Drop as MenuDrop, Selection};
use crate::model::{Call, Node, Path};
use crate::registry::{self, Kind, Prop, Spec};
use crate::theme;
use gpui::{Pixels, Point, Window};
use gpui_component::resizable::{h_resizable, resizable_panel};

use crate::workspace::{MenuField, Workspace};

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
        if self.preferences {
            // The tab strip stays: it is the way back to an open view.
            return v_flex()
                .flex_1()
                .overflow_hidden()
                .child(self.render_tabs(cx))
                .child(self.render_preferences(cx))
                .into_any_element();
        }
        if self.menu_file.is_some() {
            // The tab strip stays: it is the way back to an open view.
            return v_flex()
                .flex_1()
                .overflow_hidden()
                .child(self.render_tabs(cx))
                .child(self.render_menu_editor(cx))
                .into_any_element();
        }
        if self.code.is_some() {
            // The tab strip stays: it is the way back to an open view.
            return v_flex()
                .flex_1()
                .overflow_hidden()
                .child(self.render_tabs(cx))
                .child(self.render_code(cx))
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
        let reading = self.code.as_ref().is_some_and(|file| !file.of_view);
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
        let read_tab = self.code.as_ref().filter(|file| !file.of_view).map(|file| file.name());

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

    /// The menu bar of the project, as a tree with a small inspector.
    fn render_menu_editor(&self, cx: &mut Context<Self>) -> AnyElement {
        let menus = self.menu_file.as_ref().expect("checked by the caller");
        let selection = menus.selected;

        let mut rows: Vec<AnyElement> = Vec::new();
        for (menu_index, menu) in menus.menus.iter().enumerate() {
            rows.push(menu_zone(MenuDrop::Menu(menu_index), cx));
            let target = Selection::Menu(menu_index);
            rows.push(
                menu_row(
                    SharedString::from(menu.name.clone()),
                    0,
                    selection == Some(target),
                    target,
                    cx,
                )
                .into_any_element(),
            );
            for (item_index, item) in menu.items.iter().enumerate() {
                rows.push(menu_zone(MenuDrop::Item(menu_index, item_index), cx));
                let target = Selection::Item(menu_index, item_index);
                rows.push(
                    menu_row(
                        SharedString::from(item.label()),
                        1,
                        selection == Some(target),
                        target,
                        cx,
                    )
                    .into_any_element(),
                );
                // A submenu shows its entries one notch further in: without
                // that, it would be a row that can be selected without ever
                // seeing what it holds.
                let ItemDef::Submenu(inner) = item else {
                    continue;
                };
                for (sub_index, sub_item) in inner.items.iter().enumerate() {
                    rows.push(menu_zone(MenuDrop::SubItem(menu_index, item_index, sub_index), cx));
                    let target = Selection::SubItem(menu_index, item_index, sub_index);
                    rows.push(
                        menu_row(
                            SharedString::from(sub_item.label()),
                            2,
                            selection == Some(target),
                            target,
                            cx,
                        )
                        .into_any_element(),
                    );
                }
                rows.push(menu_zone(
                    MenuDrop::SubItem(menu_index, item_index, inner.items.len()),
                    cx,
                ));
            }
            // The end of each menu is a target: without it, nothing can be
            // dropped after the last entry.
            rows.push(menu_zone(MenuDrop::Item(menu_index, menu.items.len()), cx));
        }
        rows.push(menu_zone(MenuDrop::Menu(menus.menus.len()), cx));

        div()
            .flex()
            .flex_row()
            .flex_1()
            .overflow_hidden()
            .child(
                v_flex()
                    .flex_1()
                    .p_4()
                    .gap_1()
                    .child(
                        div()
                            .text_xs()
                            .text_color(theme::text_muted())
                            .child(crate::tr("designer.project_menu_bar")),
                    )
                    .children(rows),
            )
            .child(
                v_flex()
                    .w(px(280.))
                    .flex_none()
                    .border_l_1()
                    .border_color(theme::border())
                    .bg(theme::panel_bg())
                    .child(self.render_menu_inspector(cx))
                    .child(section_title("designer.add"))
                    .child(
                        v_flex()
                            .gap_1()
                            .p_2()
                            .child(menu_button("menu-add", "designer.menu", cx, |this, cx| {
                                this.add_menu(cx)
                            }))
                            .child(menu_button("item-add", "designer.entry", cx, |this, cx| {
                                this.add_menu_item(false, cx)
                            }))
                            .child(menu_button("sep-add", "designer.separator", cx, |this, cx| {
                                this.add_menu_item(true, cx)
                            }))
                            .child(menu_button(
                                "submenu-add",
                                "designer.submenu",
                                cx,
                                |this, cx| this.add_submenu(cx),
                            ))
                            .child(menu_button("menu-del", "designer.delete", cx, |this, cx| {
                                this.remove_menu_selection(cx)
                            })),
                    )
                    .child(section_title("designer.order"))
                    .child(
                        h_flex()
                            .gap_1()
                            .p_2()
                            .child(menu_button("menu-up", "designer.move_up", cx, |this, cx| {
                                this.move_menu_selection(true, cx)
                            }))
                            .child(menu_button(
                                "menu-down",
                                "designer.move_down",
                                cx,
                                |this, cx| this.move_menu_selection(false, cx),
                            )),
                    ),
            )
            .into_any_element()
    }

    /// The fields of the selected menu or entry.
    fn render_menu_inspector(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let menus = self.menu_file.as_ref().expect("checked by the caller");
        // The labels are translation keys, like the catalogue's.
        let fields: &[(MenuField, &str)] = match menus.selected {
            Some(Selection::Menu(_)) => &[(MenuField::Name, "menu.title")],
            // A submenu carries a title, not an action: offering it an Action
            // field would be offering what cannot be written.
            Some(_) if matches!(menus.selected_item(), Some(ItemDef::Submenu(_))) => {
                &[(MenuField::Label, "menu.title")]
            }
            Some(_)
                if matches!(
                    menus.selected_item(),
                    Some(ItemDef::Action { os_action: None, .. })
                ) =>
            {
                &[
                    (MenuField::Label, "prop.label"),
                    (MenuField::Action, "prop.action"),
                    (MenuField::Shortcut, "menu.shortcut"),
                ]
            }
            Some(Selection::Item(..)) | Some(Selection::SubItem(..)) => {
                &[(MenuField::Label, "prop.label"), (MenuField::Action, "prop.action")]
            }
            None => &[],
        };

        let mut rows = Vec::new();
        for (field, label) in fields {
            let Some(state) = self.menu_input(*field) else {
                continue;
            };
            rows.push(
                h_flex()
                    .items_center()
                    .gap_2()
                    .px_3()
                    .py_1()
                    .child(
                        div()
                            .w(px(70.))
                            .flex_none()
                            .text_xs()
                            .text_color(theme::text_muted())
                            .child(crate::tr(label)),
                    )
                    .child(div().flex_1().child(Input::new(state).small()))
                    .when(*field == MenuField::Action, |this| {
                        this.child(
                            div()
                                .id("menu-goto")
                                .px_2()
                                .rounded_sm()
                                .text_xs()
                                .cursor_pointer()
                                .hover(|this| this.bg(theme::hover_bg()))
                                .child(SharedString::from(format!(
                                    "→ {}",
                                    crate::tools::editor_label(cx)
                                )))
                                .on_click(cx.listener(|this, _, _, cx| this.open_menu_handler(cx))),
                        )
                    })
                    .into_any_element(),
            );
        }

        v_flex()
            .child(section_title("designer.properties"))
            .when(rows.is_empty(), |this| {
                this.child(
                    div()
                        .px_3()
                        .py_2()
                        .text_xs()
                        .text_color(theme::text_muted())
                        .child(crate::tr("designer.select_menu_or_entry")),
                )
            })
            .children(rows)
    }

    /// The drawing surface: the tree rendered with real components.
    fn render_canvas(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let view = self.view().expect("checked by the caller");
        // What the canvas needs the project for: an image is written as a path
        // relative to the root, because that is the directory `cargo run`
        // starts in — resolving it against anything else would draw something
        // the binary will not.
        let root = self.project().map(|project| project.root.as_path());
        div()
            .relative()
            .flex_1()
            .size_full()
            .child(
                // The board is as tall as what it holds — an image at its
                // natural size is enough to pass the window — so the canvas
                // scrolls rather than cutting the view off with no way down.
                div()
                    .id("canvas")
                    .size_full()
                    .overflow_y_scroll()
                    .track_scroll(&self.canvas_scroll)
                    .flex()
                    .p_6()
                    .justify_center()
                    .overflow_x_hidden()
                    .child(
                        div()
                            // A capped width rather than a fixed one: the board
                            // is 520 px when there is room, and shrinks rather
                            // than being cut when the window narrows. Cut and
                            // centred, it lost both of its edges at once.
                            .w_full()
                            .max_w(px(520.))
                            .h_full()
                            .p_2()
                            .rounded_md()
                            .border_1()
                            .border_color(theme::border())
                            .bg(theme::panel_bg())
                            .child(node_element(&view.root, &[], &view.selected, root, cx)),
                    ),
            )
            .child(
                div()
                    .absolute()
                    .top_0()
                    .right_0()
                    .bottom_0()
                    .child(Scrollbar::vertical(&self.canvas_scroll)),
            )
    }

    /// Tree, inspector and palette, stacked on the right.
    ///
    /// The three sections together are taller than the window as soon as a view
    /// has a few nodes, so the column scrolls and carries a visible bar.
    fn render_side_panels(&self, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .relative()
            // No width here: the resizable panel is what gives it, and a fixed
            // width inside would fight with the handle.
            .size_full()
            .border_l_1()
            .border_color(theme::border())
            .bg(theme::panel_bg())
            .child(
                div()
                    .id("side-panels")
                    .size_full()
                    .overflow_y_scroll()
                    .track_scroll(&self.side_scroll)
                    .child(
                        v_flex()
                            .child(self.render_tree(cx))
                            .child(self.render_inspector(cx))
                            .child(self.render_state(cx))
                            .child(self.render_palette(cx)),
                    ),
            )
            .child(
                div()
                    .absolute()
                    .top_0()
                    .right_0()
                    .bottom_0()
                    .child(Scrollbar::vertical(&self.side_scroll)),
            )
    }

    /// The node tree, mirroring the canvas selection.
    fn render_tree(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let view = self.view().expect("checked by the caller");
        let mut rows: Vec<TreeRow> = Vec::new();
        view.root.walk(&mut |path, node| {
            rows.push(TreeRow {
                path: path.to_vec(),
                label: node_label(node),
                depth: path.len(),
                selected: path == view.selected.as_slice(),
                container: registry::of(node).is_some_and(|spec| spec.container),
                children: node.children.len(),
            });
        });
        let top_level = view.root.children.len();
        let root_takes_children =
            registry::of(&view.root).is_some_and(|spec| spec.container) || top_level > 0;

        // A strip before every row, and one at the end for the root: dropping
        // between two rows is what says *where*, and dropping on a row what
        // says *inside what* — the two questions a tree has that the canvas,
        // where the containers are drawn, does not.
        let mut out: Vec<AnyElement> = Vec::with_capacity(rows.len() * 2 + 1);
        for row in rows {
            if let Some((index, parent)) = row.path.split_last() {
                out.push(tree_zone(parent.to_vec(), *index, cx));
            }
            out.push(tree_row(row, cx));
        }
        if root_takes_children {
            out.push(tree_zone(Vec::new(), top_level, cx));
        }

        v_flex().child(section_title("designer.structure")).children(out)
    }

    /// Property editor for the selected node, driven by the catalogue.
    fn render_inspector(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let view = self.view().expect("checked by the caller");
        let node = view.selected();
        let spec = registry::of(node);

        // Built eagerly: `cx` cannot be reborrowed inside the `FnMut` a
        // `.children(map(..))` would need.
        let mut rows = Vec::new();
        if let Some(spec) = spec {
            for prop in registry::props(spec) {
                rows.push(self.render_prop(node, spec, prop, cx).into_any_element());
            }

            // Everything the model carries and no property owns. Shown rather
            // than hidden: maxx preserves these faithfully, so it should at
            // least admit they are there.
            let extra: Vec<Call> = node
                .calls
                .iter()
                .filter(|call| {
                    call.name != crate::model::CHILD_SLOT && !registry::covers(spec, &call.name)
                })
                .cloned()
                .collect();
            if !extra.is_empty() {
                rows.push(section_title("designer.other_calls").into_any_element());
                for call in extra {
                    rows.push(self.render_extra_call(&call, cx).into_any_element());
                }
            }
        }

        v_flex()
            .child(section_title("designer.properties"))
            .when(node.is_opaque(), |this| {
                this.child(
                    div()
                        .px_3()
                        .py_2()
                        .text_xs()
                        .text_color(theme::text_muted())
                        .child(crate::tr("designer.opaque")),
                )
            })
            .children(rows)
    }

    /// One call the catalogue does not know about: shown, and removable.
    fn render_extra_call(&self, call: &Call, cx: &mut Context<Self>) -> impl IntoElement {
        let name = call.name.clone();
        let mut text = format!(".{}(", call.name);
        for (index, arg) in call.args.iter().enumerate() {
            if index > 0 {
                text.push_str(", ");
            }
            text.push_str(&arg.to_source());
        }
        text.push(')');

        h_flex()
            .items_center()
            .gap_2()
            .px_3()
            .py_1()
            .child(
                div()
                    .flex_1()
                    .text_xs()
                    .font_family("Menlo")
                    .text_color(theme::text_muted())
                    .child(SharedString::from(text)),
            )
            .child(
                div()
                    .id(SharedString::from(format!("drop-call-{name}")))
                    .px_2()
                    .rounded_sm()
                    .cursor_pointer()
                    .hover(|this| this.bg(theme::hover_bg()))
                    .child("×")
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.remove_call_at_selection(&name, cx);
                    })),
            )
    }

    /// One property row.
    fn render_prop(
        &self,
        node: &Node,
        spec: &'static Spec,
        prop: &'static Prop,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let current = registry::read(node, prop).unwrap_or_default();
        let row = h_flex().items_center().gap_2().px_3().py_1().child(
            div()
                .w(px(90.))
                .flex_none()
                .text_xs()
                .text_color(theme::text_muted())
                .child(crate::tr(prop.label)),
        );

        match prop.kind {
            // The state panel knows which fields can back an input; making the
            // name be typed again when the answer is on screen is the tool
            // contradicting itself.
            Kind::Field if !self.state_fields().is_empty() => {
                let current = current.clone();
                row.child(
                    div()
                        .id(SharedString::from(format!("field-{}-{}", spec.id, prop.label)))
                        .flex_1()
                        .px_2()
                        .rounded_sm()
                        .cursor_pointer()
                        .bg(theme::bg())
                        .text_color(theme::accent())
                        .hover(|this| this.bg(theme::hover_bg()))
                        .child(if current.is_empty() {
                            SharedString::from("—")
                        } else {
                            SharedString::from(current)
                        })
                        .on_click(
                            cx.listener(move |this, _, _, cx| this.cycle_input_field(prop, cx)),
                        ),
                )
            }
            Kind::Text if registry::read_binding(node, prop).is_some() => {
                let field = registry::read_binding(node, prop).unwrap_or_default();
                row.child(
                    div()
                        .id(SharedString::from(format!("bind-{}-{}", spec.id, prop.label)))
                        .flex_1()
                        .px_2()
                        .rounded_sm()
                        .cursor_pointer()
                        .bg(theme::bg())
                        .text_color(theme::accent())
                        .hover(|this| this.bg(theme::hover_bg()))
                        .child(SharedString::from(field))
                        .on_click(cx.listener(move |this, _, _, cx| this.cycle_binding(prop, cx))),
                )
                .child(binding_toggle(spec, prop, true, cx))
            }
            Kind::Text
            | Kind::Field
            | Kind::Handler
            | Kind::Number
            | Kind::Color
            | Kind::Ratio
            | Kind::Path => {
                match self.prop_input(prop) {
                    Some(state) if matches!(prop.kind, Kind::Handler) => {
                        row.child(div().flex_1().child(Input::new(state).small())).child(
                            div()
                                .id(SharedString::from(format!("goto-{}", prop.label)))
                                .px_2()
                                .rounded_sm()
                                .text_xs()
                                .cursor_pointer()
                                .hover(|this| this.bg(theme::hover_bg()))
                                .child(
                                    t!("designer.open_in", editor = crate::tools::editor_label(cx))
                                        .into_owned(),
                                )
                                .on_click(
                                    cx.listener(move |this, _, _, cx| this.open_handler(prop, cx)),
                                ),
                        )
                    }
                    Some(state) if matches!(prop.kind, Kind::Path) => {
                        row.child(div().flex_1().child(Input::new(state).small())).child(
                            div()
                                .id(SharedString::from(format!("pick-{}", prop.label)))
                                .px_2()
                                .rounded_sm()
                                .text_xs()
                                .cursor_pointer()
                                .hover(|this| this.bg(theme::hover_bg()))
                                .child(crate::tr("designer.choose"))
                                .on_click(
                                    cx.listener(move |this, _, _, cx| this.pick_path(prop, cx)),
                                ),
                        )
                    }
                    Some(state) if matches!(prop.kind, Kind::Text) => row
                        .child(div().flex_1().child(Input::new(state).small()))
                        .child(binding_toggle(spec, prop, false, cx)),
                    Some(state) => row.child(div().flex_1().child(Input::new(state).small())),
                    // No input this frame: the sync runs at the top of `render`, so
                    // this only shows for a frame after a selection change.
                    None => row.child(
                        div()
                            .flex_1()
                            .px_2()
                            .rounded_sm()
                            .bg(theme::bg())
                            .child(SharedString::from(current)),
                    ),
                }
            }
            Kind::Bool => {
                let on = current == "true";
                row.child(
                    div()
                        .id(SharedString::from(format!("prop-{}-{}", spec.id, prop.label)))
                        .px_2()
                        .rounded_sm()
                        .cursor_pointer()
                        .bg(if on { theme::accent() } else { theme::bg() })
                        .text_color(if on { theme::on_accent() } else { theme::text() })
                        .child(crate::tr(if on { "designer.yes" } else { "designer.no" }))
                        .on_click(cx.listener(move |this, _, _, cx| {
                            this.edit_prop(prop, if on { "false" } else { "true" }, cx);
                        })),
                )
            }
            Kind::Choice => {
                let names = match prop.target {
                    crate::registry::Target::Family(names) => names,
                    _ => &[][..],
                };
                let next = next_in_family(names, &current);
                row.child(
                    div()
                        .id(SharedString::from(format!("prop-{}-{}", spec.id, prop.label)))
                        .flex_1()
                        .px_2()
                        .rounded_sm()
                        .cursor_pointer()
                        .bg(theme::bg())
                        .hover(|this| this.bg(theme::hover_bg()))
                        .child(if current.is_empty() {
                            crate::tr("designer.default")
                        } else {
                            SharedString::from(current)
                        })
                        .on_click(cx.listener(move |this, _, _, cx| {
                            this.edit_prop(prop, &next, cx);
                        })),
                )
            }
        }
    }

    /// The fields of the view's struct, and a box to add one.
    ///
    /// A property can only read what exists, so declaring the field comes
    /// first; binding a property to it is one click away in the inspector.
    fn render_state(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let fields = self.view().map(|view| view.state_fields()).unwrap_or_default();
        let (type_label, _, _) = crate::view::STATE_TYPES[self.state_type()];
        let type_label = crate::tr(type_label);

        v_flex()
            .child(section_title("designer.state"))
            .children(fields.into_iter().map(|field| {
                h_flex()
                    .gap_2()
                    .px_3()
                    .py_1()
                    .text_xs()
                    .child(div().flex_1().child(SharedString::from(field.name)))
                    .child(
                        div()
                            .text_color(theme::text_muted())
                            .font_family("Menlo")
                            .child(SharedString::from(field.ty)),
                    )
            }))
            .child(
                h_flex()
                    .gap_2()
                    .px_3()
                    .py_1()
                    .when_some(self.state_name_input().cloned(), |this, state| {
                        this.child(div().flex_1().child(Input::new(&state).small()))
                    })
                    .child(
                        div()
                            .id("state-type")
                            .px_2()
                            .rounded_sm()
                            .text_xs()
                            .cursor_pointer()
                            .bg(theme::bg())
                            .hover(|this| this.bg(theme::hover_bg()))
                            .child(type_label)
                            .on_click(cx.listener(|this, _, _, cx| this.cycle_state_type(cx))),
                    )
                    .child(
                        div()
                            .id("state-add")
                            .px_2()
                            .rounded_sm()
                            .text_xs()
                            .cursor_pointer()
                            .bg(theme::accent())
                            .text_color(theme::on_accent())
                            .child(crate::tr("designer.add"))
                            .on_click(cx.listener(|this, _, _, cx| this.add_state_field(cx))),
                    ),
            )
    }

    /// The component palette. Clicking inserts into the selected container.
    fn render_palette(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let query = self
            .palette_filter()
            .map(|filter| filter.read(cx).value().to_string())
            .unwrap_or_default();
        let matching: Vec<_> =
            registry::CATALOGUE.iter().filter(|spec| matches_query(spec, &query)).collect();

        v_flex()
            .child(section_title("designer.components"))
            .when_some(self.palette_filter().cloned(), |this, filter| {
                this.child(div().px_3().pb_1().child(Input::new(&filter).small()))
            })
            .when(matching.is_empty(), |this| {
                this.child(
                    div()
                        .px_3()
                        .py_1()
                        .text_xs()
                        .text_color(theme::text_muted())
                        .child(crate::tr("designer.no_component")),
                )
            })
            .children(matching.into_iter().map(|spec| {
                div()
                    .id(SharedString::from(format!("palette-{}", spec.id)))
                    .px_3()
                    .py_1()
                    .cursor_pointer()
                    .hover(|this| this.bg(theme::hover_bg()))
                    .child(crate::tr(spec.label))
                    .on_drag(Dragged::Component(spec.id), move |_, _: Point<Pixels>, _, cx| {
                        cx.new(|_| DragGhost { label: crate::tr(spec.label) })
                    })
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.insert_component(spec.id, cx);
                    }))
            }))
    }
}

/// Whether a catalogue entry answers to what was typed in the search box.
pub fn matches_query(spec: &registry::Spec, query: &str) -> bool {
    label_matches(&crate::tr(spec.label), spec.id, query)
}

/// The search itself, over a label already in the interface's language.
///
/// The id is searched as well as the label: `input` finds the text field
/// whatever that language is, which is what someone who has read the generated
/// code will type.
pub fn label_matches(label: &str, id: &str, query: &str) -> bool {
    let query = fold(query);
    query.is_empty() || fold(label).contains(&query) || id.contains(&query)
}

/// Lowercase, and without the accents.
///
/// Someone looking for « Étiquette » types `etiquette`: a search that answers
/// only to the exact spelling is a search nobody uses twice.
fn fold(value: &str) -> String {
    value
        .to_lowercase()
        .chars()
        .map(|character| match character {
            'á'..='å' | 'à' => 'a',
            'é' | 'è' | 'ê' | 'ë' => 'e',
            'í' | 'ì' | 'î' | 'ï' => 'i',
            'ó' | 'ò' | 'ô' | 'ö' | 'õ' => 'o',
            'ú' | 'ù' | 'û' | 'ü' => 'u',
            'ç' => 'c',
            other => other,
        })
        .collect()
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

/// One row of the structure tree.
struct TreeRow {
    path: Path,
    label: SharedString,
    depth: usize,
    selected: bool,
    /// Whether the catalogue lets this node hold children.
    container: bool,
    children: usize,
}

/// A gap between two rows of the structure tree, and what it accepts.
///
/// Six pixels, like the menu editor's: enough to aim at, and invisible when
/// nothing is being dragged.
fn tree_zone(parent: Path, index: usize, cx: &mut Context<Workspace>) -> AnyElement {
    div()
        .id(SharedString::from(format!("tree-zone-{parent:?}-{index}")))
        .flex_none()
        .h(px(6.))
        .w_full()
        .drag_over::<Dragged>(|style, _, _, _| style.bg(theme::accent()))
        .on_drop(cx.listener(move |this, dragged: &Dragged, _, cx| {
            this.drop_at(&parent, index, dragged.clone(), cx);
        }))
        .into_any_element()
}

/// One row of the structure tree: selectable, draggable, and a drop target.
///
/// The root is the one row that cannot be dragged — it has no parent to be
/// moved into, and moving it would detach the whole tree.
fn tree_row(row: TreeRow, cx: &mut Context<Workspace>) -> AnyElement {
    let TreeRow { path, label, depth, selected, container, children } = row;
    let clicked = path.clone();
    let dropped = path.clone();
    let ghost = label.clone();
    // A leaf takes the drop beside it, a container inside it. A root that is
    // neither — a hand-written expression maxx only carries — has nowhere to
    // put it, and must not colour itself as if it had.
    let takes_drop = container || !path.is_empty();
    div()
        .id(SharedString::from(format!("tree-{path:?}")))
        .when(!path.is_empty(), move |this| {
            this.on_drag(Dragged::Node(path.clone()), move |_, _: Point<Pixels>, _, cx| {
                cx.new(|_| DragGhost { label: ghost.clone() })
            })
        })
        .flex()
        .items_center()
        .h(px(20.))
        .pr_2()
        .pl(px(8. + 12. * depth as f32))
        .cursor_pointer()
        .when(selected, |this| this.bg(theme::selected_bg()))
        .hover(|this| this.bg(theme::hover_bg()))
        .when(takes_drop, |this| {
            this.drag_over::<Dragged>(|style, _, _, _| style.bg(theme::accent())).on_drop(
                cx.listener(move |this, dragged: &Dragged, _, cx| {
                    // A node dropped on its own row has not moved. Saying
                    // nothing is the whole answer: taking a checkpoint here
                    // would clear the redo stack for a move that never
                    // happened, and `drop_at` cannot see it — the destination
                    // it computes is the sibling index just past the source.
                    if matches!(dragged, Dragged::Node(from) if *from == dropped) {
                        return;
                    }
                    // On a container, the drop goes inside it, after what it
                    // already holds; on a leaf, right after the row itself.
                    // Anything else would be asking the user to aim at a strip
                    // they cannot see.
                    match dropped.split_last() {
                        _ if container => this.drop_at(&dropped, children, dragged.clone(), cx),
                        Some((index, parent)) => {
                            this.drop_at(parent, index + 1, dragged.clone(), cx)
                        }
                        None => {}
                    }
                }),
            )
        })
        .child(label)
        .on_click(cx.listener(move |this, _, _, cx| {
            this.select(clicked.clone(), cx);
        }))
        .into_any_element()
}

/// A thin strip between two children that accepts a drop.
///
/// Insertion points are their own elements rather than a computation on the
/// container's bounds: the strip is what the user aims at, and it is what
/// highlights.
fn drop_zone(
    parent: Path,
    index: usize,
    vertical: bool,
    cx: &mut Context<Workspace>,
) -> AnyElement {
    div()
        .id(SharedString::from(format!("zone-{parent:?}-{index}")))
        .flex_none()
        .when(vertical, |this| this.h(px(8.)).w_full())
        .when(!vertical, |this| this.w(px(8.)).h_full().min_h(px(16.)))
        .drag_over::<Dragged>(|style, _, _, _| style.bg(theme::accent()))
        .on_drop(cx.listener(move |this, dragged: &Dragged, _, cx| {
            this.drop_at(&parent, index, dragged.clone(), cx);
        }))
        .into_any_element()
}

/// The children of a container, interleaved with their insertion points.
fn children_with_zones(
    node: &Node,
    path: &[usize],
    selected: &[usize],
    vertical: bool,
    root: Option<&std::path::Path>,
    cx: &mut Context<Workspace>,
) -> Vec<AnyElement> {
    let mut out = Vec::with_capacity(node.children.len() * 2 + 1);
    out.push(drop_zone(path.to_vec(), 0, vertical, cx));
    for (index, child) in node.children.iter().enumerate() {
        let mut child_path = path.to_vec();
        child_path.push(index);
        out.push(node_element(child, &child_path, selected, root, cx));
        out.push(drop_zone(path.to_vec(), index + 1, vertical, cx));
    }
    out
}

/// The button that switches a text property between a literal and a field of
/// the view's state.
fn binding_toggle(
    spec: &'static Spec,
    prop: &'static Prop,
    bound: bool,
    cx: &mut Context<Workspace>,
) -> impl IntoElement {
    div()
        .id(SharedString::from(format!("toggle-{}-{}", spec.id, prop.label)))
        .px_1()
        .rounded_sm()
        .text_xs()
        .cursor_pointer()
        .font_family("Menlo")
        .text_color(if bound { theme::accent() } else { theme::text_muted() })
        .hover(|this| this.bg(theme::hover_bg()))
        .child(if bound { "{ }" } else { "abc" })
        .on_click(cx.listener(move |this, _, _, cx| this.toggle_binding(prop, cx)))
}

/// One line of the menu tree.
/// A gap between two rows of the menu tree, and what it accepts.
///
/// Eight pixels, like the canvas's: enough to aim at, and it disappears into
/// the row spacing when nothing is being dragged.
fn menu_zone(to: MenuDrop, cx: &mut Context<Workspace>) -> AnyElement {
    div()
        .id(SharedString::from(format!("menu-zone-{to:?}")))
        .flex_none()
        .h(px(6.))
        .w_full()
        .drag_over::<Selection>(|style, _, _, _| style.bg(theme::accent()))
        .on_drop(cx.listener(move |this, from: &Selection, _, cx| {
            this.drop_menu_row(*from, to, cx);
        }))
        .into_any_element()
}

fn menu_row(
    label: SharedString,
    depth: usize,
    selected: bool,
    target: Selection,
    cx: &mut Context<Workspace>,
) -> impl IntoElement {
    div()
        .id(SharedString::from(format!("menu-{target:?}")))
        .on_drag(target, {
            let label = label.clone();
            move |_, _: Point<Pixels>, _, cx| cx.new(|_| DragGhost { label: label.clone() })
        })
        .flex()
        .items_center()
        .h(px(22.))
        .pr_2()
        .pl(px(8. + 16. * depth as f32))
        .rounded_sm()
        .cursor_pointer()
        .when(selected, |this| this.bg(theme::selected_bg()))
        .hover(|this| this.bg(theme::hover_bg()))
        .child(label)
        .on_click(cx.listener(move |this, _, _, cx| this.select_menu(target, cx)))
}

/// One button of the menu panel.
/// A small button of the menu panel, labelled from its translation key.
fn menu_button(
    id: &'static str,
    key: &'static str,
    cx: &mut Context<Workspace>,
    action: impl Fn(&mut Workspace, &mut Context<Workspace>) + 'static,
) -> impl IntoElement {
    div()
        .id(id)
        .px_2()
        .py_1()
        .rounded_sm()
        .text_xs()
        .cursor_pointer()
        .bg(theme::bg())
        .hover(|this| this.bg(theme::hover_bg()))
        .child(crate::tr(key))
        .on_click(cx.listener(move |this, _, _, cx| action(this, cx)))
}

/// Section header inside the right-hand panels.
fn section_title(key: &'static str) -> impl IntoElement {
    div()
        .px_3()
        .py_2()
        .text_xs()
        .text_color(theme::text_muted())
        .border_t_1()
        .border_color(theme::border())
        .child(crate::tr(key))
}

/// Name shown for a node in the tree.
fn node_label(node: &Node) -> SharedString {
    match registry::of(node) {
        Some(spec) => {
            let detail = node
                .call("label")
                .and_then(|call| call.args.first())
                .and_then(|arg| arg.as_str().map(str::to_string))
                .or_else(|| match &node.base {
                    crate::model::Base::Known { args, .. } => {
                        args.first().and_then(|arg| arg.as_str().map(str::to_string))
                    }
                    crate::model::Base::Opaque(_) => None,
                });
            match detail {
                Some(text) => format!("{} · {text}", crate::tr(spec.label)).into(),
                None => crate::tr(spec.label),
            }
        }
        None if node.is_opaque() => "code Rust".into(),
        None => node.base.path().unwrap_or("?").to_string().into(),
    }
}

/// The next value of a family, cycling through it and back to "unset".
fn next_in_family(names: &'static [&'static str], current: &str) -> String {
    match names.iter().position(|name| *name == current) {
        None => names.first().map(|name| (*name).to_string()).unwrap_or_default(),
        Some(index) if index + 1 < names.len() => names[index + 1].to_string(),
        Some(_) => String::new(),
    }
}

/// Renders one node on the canvas, wrapped in its selection chrome.
fn node_element(
    node: &Node,
    path: &[usize],
    selected: &[usize],
    root: Option<&std::path::Path>,
    cx: &mut Context<Workspace>,
) -> AnyElement {
    let is_selected = path == selected;
    let target = path.to_vec();

    let dragged_label = node_label(node);
    let drag_path = path.to_vec();

    div()
        .id(SharedString::from(format!("canvas-{path:?}")))
        .when(!path.is_empty(), |this| {
            // The root has nowhere to go, so it is the one node that is not
            // draggable.
            this.on_drag(Dragged::Node(drag_path), move |_, _: Point<Pixels>, _, cx| {
                let label = dragged_label.clone();
                cx.new(|_| DragGhost { label })
            })
        })
        .border_1()
        .border_color(if is_selected { theme::accent() } else { theme::bg() })
        .rounded_sm()
        .cursor_pointer()
        .child(preview(node, path, selected, root, cx))
        .on_click(cx.listener(move |this, event: &gpui::ClickEvent, _, cx| {
            // Every node wraps its children in a listener of its own, so
            // without this the click keeps bubbling and each ancestor
            // re-selects itself — the root always won.
            cx.stop_propagation();
            this.select(target.clone(), cx);
            if is_double_click(event) {
                this.add_handler_to_selection(cx);
            }
        }))
        .into_any_element()
}

/// The frame an image stands in for: nothing written yet, no project to
/// resolve the path against, or a file that is not there.
fn missing_image() -> AnyElement {
    div()
        .h(px(60.))
        .w(px(90.))
        .flex()
        .items_center()
        .justify_center()
        .rounded_sm()
        .border_1()
        .border_color(theme::border())
        .bg(theme::bg())
        .text_xs()
        .text_color(theme::text_muted())
        .child(crate::tr("component.image"))
        .into_any_element()
}

/// Whether this click was the second of a double click.
fn is_double_click(event: &gpui::ClickEvent) -> bool {
    matches!(event, gpui::ClickEvent::Mouse(mouse) if mouse.up.click_count >= 2)
}

/// The component itself, as it will look once generated.
fn preview(
    node: &Node,
    path: &[usize],
    selected: &[usize],
    root: Option<&std::path::Path>,
    cx: &mut Context<Workspace>,
) -> AnyElement {
    let text = |index: usize| -> SharedString {
        match &node.base {
            crate::model::Base::Known { args, .. } => args
                .get(index)
                .map(|arg| SharedString::from(arg.as_str().unwrap_or("").to_string()))
                .unwrap_or_default(),
            crate::model::Base::Opaque(_) => SharedString::default(),
        }
    };

    match node.base.path() {
        Some("v_flex") | Some("h_flex") => {
            let base = if node.base.path() == Some("v_flex") { v_flex() } else { h_flex() };
            let vertical = node.base.path() == Some("v_flex");
            apply(base, &node.calls)
                .min_h(px(16.))
                .children(children_with_zones(node, path, selected, vertical, root, cx))
                .into_any_element()
        }
        Some("Label::new") => Label::new(text(0)).into_any_element(),
        // Drawn from the disk, at the path the binary will read. Without a
        // project to resolve it against, or with nothing written yet, a frame
        // stands in — an image that cannot be found is better admitted than
        // drawn as a blank the user takes for a layout bug.
        Some("img") => {
            // Only maxx's own writing is drawn from the disk. A hand-written
            // `img("logo.png")` is an asset of the application, looked up in an
            // `AssetSource` the canvas has no access to: showing the file of
            // the same name would promise something the binary will not draw.
            let source = registry::of(node)
                .and_then(|spec| spec.props.first())
                .filter(|prop| registry::editable(node, prop))
                .and_then(|prop| registry::read(node, prop))
                .unwrap_or_default();
            match root.filter(|_| !source.is_empty()) {
                // The frame is also the fallback: a file that is not there
                // paints nothing at all otherwise, and an image that silently
                // does not show reads as a layout bug.
                Some(root) => img(root.join(&source))
                    .max_w_full()
                    .with_fallback(missing_image)
                    .into_any_element(),
                None => missing_image(),
            }
        }
        Some("Checkbox::new") => Checkbox::new(SharedString::from(format!("preview-{path:?}")))
            .label(call_text(node, "label", &crate::tr("component.checkbox")))
            .checked(call_bool(node, "checked"))
            .into_any_element(),
        Some("Switch::new") => Switch::new(SharedString::from(format!("preview-{path:?}")))
            .label(call_text(node, "label", &crate::tr("component.switch")))
            .checked(call_bool(node, "checked"))
            .into_any_element(),
        Some("GroupBox::new") => GroupBox::new()
            .title(call_text(node, "title", &crate::tr("component.group_box")))
            .children(children_with_zones(node, path, selected, true, root, cx))
            .into_any_element(),
        Some("Divider::horizontal") => match node.call("label") {
            Some(_) => Divider::horizontal().label(call_text(node, "label", "")).into_any_element(),
            None => Divider::horizontal().into_any_element(),
        },
        // A hand-written `div()` can hold children; only an empty one is the
        // spacer the palette drops.
        Some("div") if node.children.is_empty() => {
            apply(div(), &node.calls).h(px(20.)).into_any_element()
        }
        Some("div") => apply(div(), &node.calls)
            .children(children_with_zones(node, path, selected, true, root, cx))
            .into_any_element(),
        Some("Button::new") => Button::new(SharedString::from(format!("preview-{path:?}")))
            .label(call_text(node, "label", &crate::tr("component.button")))
            .into_any_element(),
        Some("Radio::new") => {
            gpui_component::radio::Radio::new(SharedString::from(format!("preview-{path:?}")))
                .label(call_text(node, "label", &crate::tr("component.radio")))
                .checked(call_bool(node, "checked"))
                .into_any_element()
        }
        Some("Alert::new") => Alert::new(
            SharedString::from(format!("preview-{path:?}")),
            SharedString::from(text(1).to_string()),
        )
        .title(call_text(node, "title", ""))
        .into_any_element(),
        Some("Progress::new") => gpui_component::progress::Progress::new()
            .value(call_number(node, "value").unwrap_or(0.))
            .into_any_element(),
        // Two containers whose content is a child of the model rather than an
        // argument: their preview therefore has to carry the drop zones.
        Some("Link::new") => {
            gpui_component::link::Link::new(SharedString::from(format!("preview-{path:?}")))
                .children(children_with_zones(node, path, selected, false, root, cx))
                .into_any_element()
        }
        Some("Tag::new") => gpui_component::tag::Tag::new()
            .children(children_with_zones(node, path, selected, false, root, cx))
            .into_any_element(),
        // A live text input on the canvas would swallow the clicks the designer
        // needs, so the preview is a faithful lookalike.
        Some("Input::new") => div()
            .h(px(30.))
            .px_2()
            .flex()
            .items_center()
            .rounded_md()
            .border_1()
            .border_color(theme::border())
            .bg(theme::bg())
            .text_color(theme::text_muted())
            .child(SharedString::from(format!("{}", text(0))))
            .into_any_element(),
        _ => div()
            .px_2()
            .py_1()
            .rounded_sm()
            .bg(theme::hover_bg())
            .text_xs()
            .text_color(theme::text_muted())
            .child(crate::tr("designer.rust_code"))
            .into_any_element(),
    }
}

/// The string argument of a one-argument call, or `fallback`.
fn call_text(node: &Node, name: &str, fallback: &str) -> String {
    node.call(name)
        .and_then(|call| call.args.first())
        .and_then(|arg| arg.as_str())
        .unwrap_or(fallback)
        .to_string()
}

/// The numeric argument of a one-argument call, when it reads as a number.
fn call_number(node: &Node, name: &str) -> Option<f32> {
    node.call(name)?.args.first()?.to_source().trim_end_matches('.').parse().ok()
}

/// The boolean argument of a one-argument call, `false` when absent.
fn call_bool(node: &Node, name: &str) -> bool {
    node.call(name)
        .and_then(|call| call.args.first())
        .map(|arg| arg.to_source() == "true")
        .unwrap_or(false)
}

/// Applies the style calls the preview knows how to show.
///
/// A call that is not listed here is still carried by the model and written to
/// the file; it simply has no effect on the preview.
fn apply(mut element: Div, calls: &[Call]) -> Div {
    for call in calls {
        element = match call.name.as_str() {
            "gap_0" => element.gap_0(),
            "gap_1" => element.gap_1(),
            "gap_2" => element.gap_2(),
            "gap_3" => element.gap_3(),
            "gap_4" => element.gap_4(),
            "gap_6" => element.gap_6(),
            "gap_8" => element.gap_8(),
            "p_0" => element.p_0(),
            "p_1" => element.p_1(),
            "p_2" => element.p_2(),
            "p_3" => element.p_3(),
            "p_4" => element.p_4(),
            "p_6" => element.p_6(),
            "p_8" => element.p_8(),
            "items_start" => element.items_start(),
            "items_center" => element.items_center(),
            "items_end" => element.items_end(),
            "flex_1" => element.flex_1(),
            _ => element,
        };
    }
    element
}
