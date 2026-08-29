//! The palette: the components on offer, and the search that finds one.

use gpui::prelude::*;
use gpui::{Context, SharedString, div};
use gpui_component::input::Input;
use gpui_component::{Sizable as _, v_flex};

use crate::registry::{self};
use crate::theme;
use gpui::{Pixels, Point};

use crate::workspace::Workspace;

use super::{DragGhost, Dragged, section_title};

impl Workspace {
    /// The component palette. Clicking inserts into the selected container.
    pub(crate) fn render_palette(&self, cx: &mut Context<Self>) -> impl IntoElement {
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
            // The search box first, because everything under it answers to what
            // is typed in it: a list above the box changes while you type and
            // reads as a list that is not being filtered.
            .child(section_title("designer.components"))
            .when_some(self.palette_filter().cloned(), |this, filter| {
                this.child(div().px_3().pb_1().child(Input::new(&filter).small()))
            })
            .children(mine.first().map(|_| section_title("designer.project_components")))
            .children(mine.into_iter().map(|brick| {
                let name = brick.type_name.clone();
                div()
                    .id(SharedString::from(format!("brick-{}", brick.type_name)))
                    .px_3()
                    .py_1()
                    .cursor_pointer()
                    .hover(|this| this.bg(theme::hover_bg()))
                    .child(SharedString::from(brick.type_name.clone()))
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
