//! The menu editor: the application's menu bar, drawn as rows that drag.

use gpui::prelude::*;
use gpui::{AnyElement, Context, SharedString, div, px};
use gpui_component::input::Input;
use gpui_component::menu::ContextMenuExt as _;
use gpui_component::resizable::{h_resizable, resizable_panel};
use gpui_component::{Sizable as _, h_flex, v_flex};

use crate::menu_model::ItemDef;
use crate::menufile::{Drop as MenuDrop, Selection};
use crate::theme;
use gpui::{Pixels, Point};

use crate::workspace::{MenuField, Workspace, fillable};

use super::{DragGhost, section_title};

impl Workspace {
    /// The menu bar of the project, as a tree with a small inspector.
    pub(super) fn render_menu_editor(&self, cx: &mut Context<Self>) -> AnyElement {
        let menus = self.menu_file().expect("checked by the caller");
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

        let inspector_width = self.menu_inspector_width(cx);

        div()
            .flex()
            .flex_row()
            .flex_1()
            .overflow_hidden()
            .child(
                h_resizable("menus-inspecteur")
                    .with_state(&self.menu_inspector_split)
                    .child(
                        resizable_panel().child(fillable(
                            v_flex()
                                .size_full()
                                .p_4()
                                .gap_1()
                                .child(
                                    div()
                                        .text_xs()
                                        .text_color(theme::text_muted())
                                        .child(crate::tr("designer.project_menu_bar")),
                                )
                                .children(rows)
                                // One menu for the whole column, acting on the
                                // selected row: `ContextMenuExt` hard-codes the
                                // id of what it opens, so a menu per row would
                                // leave every row sharing one open state. The
                                // entries are the panel's own buttons, brought
                                // to where the rows are.
                                .context_menu(|menu, _window, _cx| {
                                    menu.menu(
                                        crate::tr("menu.move_up"),
                                        Box::new(crate::actions::MoveMenuUp),
                                    )
                                    .menu(
                                        crate::tr("menu.move_down"),
                                        Box::new(crate::actions::MoveMenuDown),
                                    )
                                    .separator()
                                    .menu(
                                        crate::tr("menu.add_entry"),
                                        Box::new(crate::actions::AddMenuEntry),
                                    )
                                    .menu(
                                        crate::tr("menu.add_submenu"),
                                        Box::new(crate::actions::AddSubmenu),
                                    )
                                    .menu(
                                        crate::tr("menu.delete_menu_entry"),
                                        Box::new(crate::actions::DeleteMenuEntry),
                                    )
                                }),
                        )),
                    )
                    .child(
                        resizable_panel()
                            .size(px(inspector_width))
                            // Below this its fields fold in on themselves; beyond it
                            // there are no menu rows left to edit.
                            .size_range(px(220.)..px(560.))
                            .child(fillable(
                                v_flex()
                                    .size_full()
                                    .border_l_1()
                                    .border_color(theme::border())
                                    .bg(theme::panel_bg())
                                    .child(self.render_menu_inspector(cx))
                                    .child(section_title("designer.add"))
                                    .child(
                                        v_flex()
                                            .gap_1()
                                            .p_2()
                                            .child(menu_button(
                                                "menu-add",
                                                "designer.menu",
                                                cx,
                                                |this, cx| this.add_menu(cx),
                                            ))
                                            .child(menu_button(
                                                "item-add",
                                                "designer.entry",
                                                cx,
                                                |this, cx| this.add_menu_item(false, cx),
                                            ))
                                            .child(menu_button(
                                                "sep-add",
                                                "designer.separator",
                                                cx,
                                                |this, cx| this.add_menu_item(true, cx),
                                            ))
                                            .child(menu_button(
                                                "submenu-add",
                                                "designer.submenu",
                                                cx,
                                                |this, cx| this.add_submenu(cx),
                                            ))
                                            .child(menu_button(
                                                "menu-del",
                                                "designer.delete",
                                                cx,
                                                |this, cx| this.remove_menu_selection(cx),
                                            )),
                                    )
                                    .child(section_title("designer.order"))
                                    .child(
                                        h_flex()
                                            .gap_1()
                                            .p_2()
                                            .child(menu_button(
                                                "menu-up",
                                                "designer.move_up",
                                                cx,
                                                |this, cx| this.move_menu_selection(true, cx),
                                            ))
                                            .child(menu_button(
                                                "menu-down",
                                                "designer.move_down",
                                                cx,
                                                |this, cx| this.move_menu_selection(false, cx),
                                            )),
                                    ),
                            )),
                    ),
            )
            .into_any_element()
    }

    /// The width the menu editor's inspector should take, and where the width
    /// the handle left is picked back up.
    ///
    /// Kept in memory only, like the other splits: writing a file on every
    /// frame of a drag would be absurd, and `settings::flush` puts it away at
    /// quit.
    fn menu_inspector_width(&self, cx: &mut Context<Self>) -> f32 {
        if let Some(largeur) = self.menu_inspector_split.read(cx).sizes().last().copied() {
            let largeur = f32::from(largeur);
            if largeur > 0. {
                crate::settings::stage_state(cx, |state| {
                    state.menu_inspector_width = Some(largeur);
                });
                return largeur;
            }
        }
        crate::settings::state(cx).menu_inspector_width.unwrap_or(280.)
    }

    /// The fields of the selected menu or entry.
    pub(super) fn render_menu_inspector(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let menus = self.menu_file().expect("checked by the caller");
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
}

/// One line of the menu tree.
/// A gap between two rows of the menu tree, and what it accepts.
///
/// Eight pixels, like the canvas's: enough to aim at, and it disappears into
/// the row spacing when nothing is being dragged.
pub(super) fn menu_zone(to: MenuDrop, cx: &mut Context<Workspace>) -> AnyElement {
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

pub(super) fn menu_row(
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
        // The menu acts on the selection, so the right click has to move it
        // before the menu is built — which it does, the menu being deferred to
        // the next frame. Same shape as the tree's rows.
        .on_mouse_down(
            gpui::MouseButton::Right,
            cx.listener(move |this, _, _, cx| this.select_menu(target, cx)),
        )
}

/// One button of the menu panel.
/// A small button of the menu panel, labelled from its translation key.
pub(super) fn menu_button(
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
