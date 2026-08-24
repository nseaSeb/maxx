//! The designer: canvas, tree, inspector and palette.
//!
//! These are render methods on [`Workspace`] rather than a separate view, so
//! that the tree stays the single source and every panel is recomputed from it
//! on each frame — a panel can never hold a stale copy of the model.

use gpui::prelude::*;
use gpui::{AnyElement, Context, Div, SharedString, div, px, rgb};
use gpui_component::button::Button;
use gpui_component::checkbox::Checkbox;
use gpui_component::divider::Divider;
use gpui_component::group_box::GroupBox;
use gpui_component::input::Input;
use gpui_component::label::Label;
use gpui_component::switch::Switch;
use gpui_component::{Sizable as _, h_flex, v_flex};

use crate::model::{Call, Node, Path};
use gpui::{Pixels, Point, Window};
use crate::registry::{self, Kind, Prop, Spec};
use crate::theme;
use crate::workspace::Workspace;

impl Workspace {
    /// The designer, or an invitation to open a view.
    pub(crate) fn render_designer(&self, cx: &mut Context<Self>) -> AnyElement {
        if self.view.is_none() {
            return div()
                .flex()
                .flex_1()
                .items_center()
                .justify_center()
                .text_color(rgb(theme::TEXT_MUTED))
                .child("Ouvrez une vue de src/ui/ pour la dessiner")
                .into_any_element();
        }

        // Not `h_flex`: it centres its children vertically, which leaves the
        // side panel floating in the middle instead of spanning the window.
        div()
            .flex()
            .flex_row()
            .flex_1()
            .overflow_hidden()
            .child(self.render_canvas(cx))
            .child(self.render_side_panels(cx))
            .into_any_element()
    }

    /// The drawing surface: the tree rendered with real components.
    fn render_canvas(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let view = self.view.as_ref().expect("checked by the caller");
        div()
            .flex()
            .flex_1()
            .p_6()
            .justify_center()
            .child(
                div()
                    .w(px(520.))
                    .p_2()
                    .rounded_md()
                    .border_1()
                    .border_color(rgb(theme::BORDER))
                    .bg(rgb(theme::PANEL_BG))
                    .child(node_element(&view.root, &[], &view.selected, cx)),
            )
    }

    /// Tree, inspector and palette, stacked on the right.
    fn render_side_panels(&self, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .w(px(280.))
            .flex_none()
            .border_l_1()
            .border_color(rgb(theme::BORDER))
            .bg(rgb(theme::PANEL_BG))
            .child(self.render_tree(cx))
            .child(self.render_inspector(cx))
            .child(self.render_palette(cx))
    }

    /// The node tree, mirroring the canvas selection.
    fn render_tree(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let view = self.view.as_ref().expect("checked by the caller");
        let mut rows: Vec<(Path, SharedString, usize, bool)> = Vec::new();
        view.root.walk(&mut |path, node| {
            rows.push((
                path.to_vec(),
                node_label(node),
                path.len(),
                path == view.selected.as_slice(),
            ));
        });

        v_flex()
            .child(section_title("Structure"))
            .children(rows.into_iter().map(|(path, label, depth, selected)| {
                let target = path.clone();
                div()
                    .id(SharedString::from(format!("tree-{path:?}")))
                    .flex()
                    .items_center()
                    .h(px(20.))
                    .pr_2()
                    .pl(px(8. + 12. * depth as f32))
                    .cursor_pointer()
                    .when(selected, |this| this.bg(rgb(theme::SELECTED_BG)))
                    .hover(|this| this.bg(rgb(theme::HOVER_BG)))
                    .child(label)
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.select(target.clone(), cx);
                    }))
            }))
    }

    /// Property editor for the selected node, driven by the catalogue.
    fn render_inspector(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let view = self.view.as_ref().expect("checked by the caller");
        let node = view.selected();
        let spec = registry::of(node);

        // Built eagerly: `cx` cannot be reborrowed inside the `FnMut` a
        // `.children(map(..))` would need.
        let mut rows = Vec::new();
        if let Some(spec) = spec {
            for prop in spec.props {
                rows.push(self.render_prop(node, spec, prop, cx).into_any_element());
            }
        }

        v_flex()
            .child(section_title("Propriétés"))
            .when(node.is_opaque(), |this| {
                this.child(
                    div()
                        .px_3()
                        .py_2()
                        .text_xs()
                        .text_color(rgb(theme::TEXT_MUTED))
                        .child("Code Rust conservé tel quel — non modifiable ici."),
                )
            })
            .children(rows)
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
        let row = h_flex()
            .items_center()
            .gap_2()
            .px_3()
            .py_1()
            .child(
                div()
                    .w(px(90.))
                    .flex_none()
                    .text_xs()
                    .text_color(rgb(theme::TEXT_MUTED))
                    .child(prop.label),
            );

        match prop.kind {
            Kind::Text | Kind::Field | Kind::Handler => match self.prop_input(prop) {
                Some(state) => row.child(div().flex_1().child(Input::new(state).small())),
                // No input yet this frame: the sync runs at the top of `render`,
                // so this only shows for a frame after a selection change.
                None => row.child(
                    div()
                        .flex_1()
                        .px_2()
                        .rounded_sm()
                        .bg(rgb(theme::BG))
                        .child(SharedString::from(current)),
                ),
            },
            Kind::Bool => {
                let on = current == "true";
                row.child(
                    div()
                        .id(SharedString::from(format!("prop-{}-{}", spec.id, prop.label)))
                        .px_2()
                        .rounded_sm()
                        .cursor_pointer()
                        .bg(rgb(if on { theme::ACCENT } else { theme::BG }))
                        .text_color(rgb(if on { theme::ON_ACCENT } else { theme::TEXT }))
                        .child(if on { "oui" } else { "non" })
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
                        .bg(rgb(theme::BG))
                        .hover(|this| this.bg(rgb(theme::HOVER_BG)))
                        .child(if current.is_empty() {
                            SharedString::from("par défaut")
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

    /// The component palette. Clicking inserts into the selected container.
    fn render_palette(&self, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .child(section_title("Composants"))
            .children(registry::CATALOGUE.iter().map(|spec| {
                div()
                    .id(SharedString::from(format!("palette-{}", spec.id)))
                    .px_3()
                    .py_1()
                    .cursor_pointer()
                    .hover(|this| this.bg(rgb(theme::HOVER_BG)))
                    .child(spec.label)
                    .on_drag(Dragged::Component(spec.id), move |_, _: Point<Pixels>, _, cx| {
                        cx.new(|_| DragGhost {
                            label: SharedString::from(spec.label),
                        })
                    })
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.insert_component(spec.id, cx);
                    }))
            }))
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
            .bg(rgb(theme::ACCENT))
            .text_color(rgb(theme::ON_ACCENT))
            .text_xs()
            .child(self.label.clone())
    }
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
        .drag_over::<Dragged>(|style, _, _, _| style.bg(rgb(theme::ACCENT)))
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
    cx: &mut Context<Workspace>,
) -> Vec<AnyElement> {
    let mut out = Vec::with_capacity(node.children.len() * 2 + 1);
    out.push(drop_zone(path.to_vec(), 0, vertical, cx));
    for (index, child) in node.children.iter().enumerate() {
        let mut child_path = path.to_vec();
        child_path.push(index);
        out.push(node_element(child, &child_path, selected, cx));
        out.push(drop_zone(path.to_vec(), index + 1, vertical, cx));
    }
    out
}

/// Section header inside the right-hand panels.
fn section_title(title: &'static str) -> impl IntoElement {
    div()
        .px_3()
        .py_2()
        .text_xs()
        .text_color(rgb(theme::TEXT_MUTED))
        .border_t_1()
        .border_color(rgb(theme::BORDER))
        .child(title)
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
                Some(text) => format!("{} · {text}", spec.label).into(),
                None => spec.label.into(),
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
            this.on_drag(
                Dragged::Node(drag_path),
                move |_, _: Point<Pixels>, _, cx| {
                    let label = dragged_label.clone();
                    cx.new(|_| DragGhost { label })
                },
            )
        })
        .border_1()
        .border_color(rgb(if is_selected {
            theme::ACCENT
        } else {
            theme::BG
        }))
        .rounded_sm()
        .cursor_pointer()
        .child(preview(node, path, selected, cx))
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

/// Whether this click was the second of a double click.
fn is_double_click(event: &gpui::ClickEvent) -> bool {
    matches!(event, gpui::ClickEvent::Mouse(mouse) if mouse.up.click_count >= 2)
}

/// The component itself, as it will look once generated.
fn preview(
    node: &Node,
    path: &[usize],
    selected: &[usize],
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
            let base = if node.base.path() == Some("v_flex") {
                v_flex()
            } else {
                h_flex()
            };
            let vertical = node.base.path() == Some("v_flex");
            apply(base, &node.calls)
                .min_h(px(16.))
                .children(children_with_zones(node, path, selected, vertical, cx))
                .into_any_element()
        }
        Some("Label::new") => Label::new(text(0)).into_any_element(),
        Some("Checkbox::new") => Checkbox::new(SharedString::from(format!("preview-{path:?}")))
            .label(call_text(node, "label", "Case à cocher"))
            .checked(call_bool(node, "checked"))
            .into_any_element(),
        Some("Switch::new") => Switch::new(SharedString::from(format!("preview-{path:?}")))
            .label(call_text(node, "label", "Interrupteur"))
            .checked(call_bool(node, "checked"))
            .into_any_element(),
        Some("GroupBox::new") => GroupBox::new()
            .title(call_text(node, "title", "Cadre"))
            .children(children_with_zones(node, path, selected, true, cx))
            .into_any_element(),
        Some("Divider::horizontal") => match node.call("label") {
            Some(_) => Divider::horizontal()
                .label(call_text(node, "label", ""))
                .into_any_element(),
            None => Divider::horizontal().into_any_element(),
        },
        // A hand-written `div()` can hold children; only an empty one is the
        // spacer the palette drops.
        Some("div") if node.children.is_empty() => {
            apply(div(), &node.calls).h(px(20.)).into_any_element()
        }
        Some("div") => apply(div(), &node.calls)
            .children(children_with_zones(node, path, selected, true, cx))
            .into_any_element(),
        Some("Button::new") => Button::new(SharedString::from(format!("preview-{path:?}")))
            .label(call_text(node, "label", "Bouton"))
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
            .border_color(rgb(theme::BORDER))
            .bg(rgb(theme::BG))
            .text_color(rgb(theme::TEXT_MUTED))
            .child(SharedString::from(format!("{}", text(0))))
            .into_any_element(),
        _ => div()
            .px_2()
            .py_1()
            .rounded_sm()
            .bg(rgb(theme::HOVER_BG))
            .text_xs()
            .text_color(rgb(theme::TEXT_MUTED))
            .child("code Rust")
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
