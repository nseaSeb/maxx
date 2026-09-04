//! The palette: the components on offer, and the search that finds one.

use gpui::prelude::*;
use gpui::{Context, SharedString, div};
use gpui_component::input::Input;
use gpui_component::menu::ContextMenuExt as _;
use gpui_component::tooltip::Tooltip;
use gpui_component::{Sizable as _, h_flex, v_flex};

use crate::registry::{self};
use crate::theme;
use gpui::{Pixels, Point};

use crate::workspace::Workspace;

use super::{DragGhost, Dragged, section_title};

impl Workspace {
    /// The palette's heading and its search box, drawn outside the scroll.
    ///
    /// Apart from the list on purpose: inside it, the box you type in scrolled
    /// away from the results it filters — you searched, the matches appeared,
    /// and reaching them took the field off the screen.
    pub(crate) fn render_palette_header(&self, _cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .flex_none()
            .bg(theme::panel_bg())
            .child(section_title("designer.components"))
            .when_some(self.palette_filter().cloned(), |this, filter| {
                this.child(div().px_3().pb_2().child(Input::new(&filter).small()))
            })
    }

    /// The component palette. Clicking inserts into the selected container.
    pub(crate) fn render_palette(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let target = self.palette_target();
        let query = self
            .palette_filter()
            .map(|filter| filter.read(cx).value().to_string())
            .unwrap_or_default();
        let matching: Vec<_> = registry::CATALOGUE
            .iter()
            .filter(|spec| spec.palette && matches_query(spec, &query))
            .collect();
        let templates: Vec<(&'static str, &'static str)> = registry::SUBTREE_LABELS
            .iter()
            .filter(|(id, label)| label_matches(&crate::tr(label), id, &query))
            .copied()
            .collect();

        // The project's own, answering the same search box. Their section comes
        // first: once a project has components of its own, they are what it is
        // built from, and hunting for them under thirty of maxx's would say the
        // opposite.
        let mine: Vec<_> = self
            .bricks
            .iter()
            .filter(|brick| label_matches(&brick.type_name, &fold(&brick.doc), &query))
            .cloned()
            .collect();

        v_flex()
            .children(mine.first().map(|_| section_title("designer.project_components")))
            .children(mine.into_iter().map(|brick| {
                let name = brick.type_name.clone();
                let module = brick.module.clone();
                let doc = SharedString::from(brick.doc.clone());
                h_flex()
                    .id(SharedString::from(format!("brick-{}", brick.type_name)))
                    .group(SharedString::from(format!("brick-row-{}", brick.type_name)))
                    .px_3()
                    .py_1()
                    .gap_2()
                    .items_center()
                    .cursor_pointer()
                    .hover(|this| this.bg(theme::hover_bg()))
                    .child(div().flex_1().child(SharedString::from(brick.type_name.clone())))
                    // What the brick is written like, one click away. It is the
                    // project's own file, so the reader can already show it from
                    // the explorer — what was missing is the way there from
                    // where the brick is seen rather than from where it is
                    // filed.
                    .child(
                        div()
                            .id(SharedString::from(format!("brick-source-{}", brick.type_name)))
                            .invisible()
                            .group_hover(
                                SharedString::from(format!("brick-row-{}", brick.type_name)),
                                |this| this.visible(),
                            )
                            .px_1()
                            .rounded_sm()
                            .text_xs()
                            .text_color(theme::text_muted())
                            .hover(|this| this.text_color(theme::text()))
                            .child(crate::tr("designer.read_source"))
                            .tooltip(move |window, cx| Tooltip::new(doc.clone()).build(window, cx))
                            .on_click(cx.listener(move |this, _, _, cx| {
                                // Every ancestor carries a click listener, and
                                // the outermost runs last: without this the row
                                // would insert the brick straight after opening
                                // its source.
                                cx.stop_propagation();
                                this.open_brick_source(&module, cx);
                            })),
                    )
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.insert_brick(&name, cx);
                    }))
            }))
            .child(section_title("designer.catalogue"))
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
            // The catalogue rows, and only them, under the menu. The project's
            // own components and the templates stay outside it: the three
            // entries insert what the tree accepts from a drag, and neither a
            // brick nor a template is dragged. A menu opening over one of those
            // rows could only speak about some other row — `ContextMenuExt`
            // hard-codes the id of what it opens, so the menu belongs to a list.
            .child(
                v_flex()
                    .children(matching.into_iter().map(|spec| {
                        div()
                            .id(SharedString::from(format!("palette-{}", spec.id)))
                            .px_3()
                            .py_1()
                            .cursor_pointer()
                            // Lit like a selection, because that is what it is:
                            // the menu acts on this row long after the click
                            // that chose it, and a choice nobody can see is a
                            // menu acting somewhere else.
                            .when(target == Some(spec.id), |this| this.bg(theme::selected_bg()))
                            .hover(|this| this.bg(theme::hover_bg()))
                            .child(crate::tr(spec.label))
                            .on_drag(
                                Dragged::Component(spec.id),
                                move |_, _: Point<Pixels>, _, cx| {
                                    cx.new(|_| DragGhost { label: crate::tr(spec.label) })
                                },
                            )
                            .on_click(cx.listener(move |this, _, _, cx| {
                                this.insert_component(spec.id, cx);
                            }))
                            // The menu acts on the row the right click lit, so
                            // it has to light this one before the menu is built
                            // — which it does, the menu being deferred to the
                            // next frame.
                            .on_mouse_down(
                                gpui::MouseButton::Right,
                                cx.listener(move |this, _, _, cx| {
                                    this.target_palette_component(spec.id, cx);
                                }),
                            )
                    }))
                    .context_menu(|menu, _window, _cx| {
                        menu.menu(
                            crate::tr("menu.insert_before"),
                            Box::new(crate::actions::InsertBefore),
                        )
                        .menu(crate::tr("menu.insert_after"), Box::new(crate::actions::InsertAfter))
                        .menu(crate::tr("menu.insert_into"), Box::new(crate::actions::InsertInto))
                    }),
            )
            // The templates come after the components and under a title of
            // their own: they answer the same search box, but a card is not a
            // component — it is several, already arranged.
            .children(templates.first().map(|_| section_title("designer.templates")))
            .children(templates.into_iter().map(|(id, label)| {
                div()
                    .id(SharedString::from(format!("template-{id}")))
                    .px_3()
                    .py_1()
                    .cursor_pointer()
                    .hover(|this| this.bg(theme::hover_bg()))
                    .child(crate::tr(label))
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.insert_subtree(id, cx);
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
pub(super) fn fold(value: &str) -> String {
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
