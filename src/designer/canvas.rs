//! The canvas: the view as it will be drawn, from the tree maxx holds.

use gpui::prelude::*;
use gpui::{AnyElement, Context, SharedString, div, img, px};
use gpui_component::alert::Alert;
use gpui_component::button::Button;
use gpui_component::checkbox::Checkbox;
use gpui_component::divider::Divider;
use gpui_component::group_box::GroupBox;
use gpui_component::label::Label;
use gpui_component::scroll::Scrollbar;
use gpui_component::switch::Switch;
use gpui_component::{h_flex, v_flex};

use crate::model::Node;
use crate::preview::Preview;
use crate::registry::{self};
use crate::theme;
use gpui::{Pixels, Point};

use crate::workspace::Workspace;

use super::inspector::{
    apply, apply_placement, base_source, call_bool, call_number, call_text, call_whole, tag_variant,
};
use super::tree::{children_with_zones, node_label};
use super::{DragGhost, Dragged};

impl Workspace {
    /// The drawing surface: the tree rendered with real components.
    pub(super) fn render_canvas(&self, cx: &mut Context<Self>) -> impl IntoElement {
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
                    // Aligned to the top, and it is not cosmetic: a flex line
                    // stretches its children to its own height by default, so
                    // the board was held to the window and its content spilled
                    // out of it — cut at the same place, with a scrollbar that
                    // had nothing to move.
                    .items_start()
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
                            // No height: the board is as tall as what it
                            // holds, and that is what gives the scroll
                            // something to scroll. Held to the viewport by
                            // `h_full`, its content overflowed it instead —
                            // cut at the same place as before, with a bar that
                            // had nothing to move.
                            .p_2()
                            .rounded_md()
                            .border_1()
                            .border_color(theme::border())
                            // The board is the application's window, so it is
                            // painted with the application's own colours — the
                            // roles of its `src/theme.rs`, not maxx's greys. A
                            // preview in the colours of the workshop is a
                            // preview of the workshop.
                            .bg(self.preview.bg())
                            // Handed down rather than set on each node: gpui
                            // inherits the text colour, so this reaches every
                            // label the view holds without the nodes knowing.
                            .text_color(self.preview.text())
                            .child(node_element(
                                &view.root,
                                &[],
                                &view.selected,
                                root,
                                &self.preview,
                                cx,
                            )),
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
}

/// Renders one node on the canvas, wrapped in its selection chrome.
pub(super) fn node_element(
    node: &Node,
    path: &[usize],
    selected: &[usize],
    root: Option<&std::path::Path>,
    preview: &Preview,
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
        // Selected, maxx's accent — the outline is tooling and has to stay
        // legible whatever the project chose. Unselected, the colour of what it
        // sits on, so the border takes its room without being seen: that is the
        // project's background now, or it would draw a grey frame around every
        // node of a coloured board.
        .border_color(if is_selected { theme::accent() } else { preview.bg() })
        .rounded_sm()
        .cursor_pointer()
        // The chrome carries it rather than the preview: the tooltip needs a
        // stateful element, and this is the one that already has an `id`. A
        // property that reaches the file and shows nothing on the canvas reads
        // as a broken field.
        .when_some(node_tooltip(node), |this, text| {
            this.tooltip(move |window, cx| {
                gpui_component::tooltip::Tooltip::new(text.clone()).build(window, cx)
            })
        })
        .child(node_preview(node, path, selected, root, preview, cx))
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

/// A small look at what the Source field points to.
///
/// A path that is wrong and a file that is missing read exactly the same in a
/// text field. Twenty-eight pixels tell them apart without leaving the
/// inspector, and they answer the question one actually has: is that the right
/// picture?
pub(super) fn thumbnail(value: &str, root: Option<&std::path::Path>) -> AnyElement {
    let frame = div()
        .size(px(28.))
        .flex_none()
        .overflow_hidden()
        .rounded_sm()
        .border_1()
        .border_color(theme::border())
        .bg(theme::bg());
    match root.filter(|_| !value.is_empty()) {
        // A file that is not there paints nothing, which is byte for byte what
        // an empty field paints — and telling those two apart is the whole
        // point of the thumbnail.
        Some(root) => frame
            .child(img(root.join(value)).size_full().with_fallback(missing_thumbnail))
            .into_any_element(),
        None => frame.into_any_element(),
    }
}

/// The fill mode written on a node, read back as gpui's own value.
///
/// Matched on the text because that is what the model holds — the variant as it
/// goes into the file. `None` when nothing was written, which is gpui's
/// `Contain` and therefore nothing to say.
pub(super) fn object_fit(node: &Node) -> Option<gpui::ObjectFit> {
    let written = node.call("object_fit")?.args.first()?.to_source();
    match written.as_str() {
        "ObjectFit::Contain" => Some(gpui::ObjectFit::Contain),
        "ObjectFit::Cover" => Some(gpui::ObjectFit::Cover),
        "ObjectFit::Fill" => Some(gpui::ObjectFit::Fill),
        "ObjectFit::ScaleDown" => Some(gpui::ObjectFit::ScaleDown),
        "ObjectFit::None" => Some(gpui::ObjectFit::None),
        _ => None,
    }
}

/// What the thumbnail shows when the path leads nowhere.
pub(super) fn missing_thumbnail() -> AnyElement {
    div()
        .size_full()
        .flex()
        .items_center()
        .justify_center()
        .bg(theme::hover_bg())
        .text_xs()
        .text_color(theme::danger())
        .child("?")
        .into_any_element()
}

/// The frame an image stands in for: nothing written yet, no project to
/// resolve the path against, or a file that is not there.
pub(crate) fn missing_image() -> AnyElement {
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
pub(super) fn is_double_click(event: &gpui::ClickEvent) -> bool {
    matches!(event, gpui::ClickEvent::Mouse(mouse) if mouse.up.click_count >= 2)
}

/// The component itself, as it will look once generated.
pub(super) fn node_preview(
    node: &Node,
    path: &[usize],
    selected: &[usize],
    root: Option<&std::path::Path>,
    preview: &Preview,
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
            apply_placement(apply(base, &node.calls), &node.calls)
                .min_h(px(16.))
                .children(children_with_zones(node, path, selected, vertical, root, preview, cx))
                .into_any_element()
        }
        Some("Label::new") => Label::new(text(0)).into_any_element(),
        // Drawn from the disk, at the path the binary will read. Without a
        // project to resolve it against, or with nothing written yet, a frame
        // stands in — an image that cannot be found is better admitted than
        // drawn as a blank the user takes for a layout bug.
        Some("img") => {
            // Only maxx's own writing is drawn. The path is relative to the
            // root, which is what makes the canvas and the binary agree: the
            // canvas joins it onto the project, the assets module answers the
            // same string. An expression someone else wrote is left as a frame
            // rather than guessed at.
            let source = registry::of(node)
                .and_then(|spec| spec.props.first())
                .filter(|prop| registry::editable(node, prop))
                .and_then(|prop| registry::read(node, prop))
                .unwrap_or_default();
            match root.filter(|_| !source.is_empty()) {
                // The frame is also the fallback: a file that is not there
                // paints nothing at all otherwise, and an image that silently
                // does not show reads as a layout bug.
                //
                // The three settings are read off the node like every other
                // call: a preview that clamps what the generated code lets
                // grow — or ignores a fill mode the binary honours — shows
                // something that will not happen, and the switches look broken.
                Some(root) => apply(img(root.join(&source)), &node.calls)
                    .map(|image| match object_fit(node) {
                        Some(fit) => image.object_fit(fit),
                        None => image,
                    })
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
            .children(children_with_zones(node, path, selected, true, root, preview, cx))
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
        Some("div") => apply_placement(apply(div(), &node.calls), &node.calls)
            .children(children_with_zones(node, path, selected, true, root, preview, cx))
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
                .children(children_with_zones(node, path, selected, false, root, preview, cx))
                .into_any_element()
        }
        Some("Tag::new") => tag_variant(node)
            .children(children_with_zones(node, path, selected, false, root, preview, cx))
            .into_any_element(),
        Some("Badge::new") => gpui_component::badge::Badge::new()
            .count(call_whole(node, "count").unwrap_or(0))
            // Both numbers, or a count of 30 beyond 9 still draws “30” on the
            // canvas and “9+” in the running application.
            .max(call_whole(node, "max").unwrap_or(99))
            .children(children_with_zones(node, path, selected, false, root, preview, cx))
            .into_any_element(),
        Some("Skeleton::new") => {
            apply(gpui_component::skeleton::Skeleton::new(), &node.calls).into_any_element()
        }
        Some("Spinner::new") => gpui_component::spinner::Spinner::new().into_any_element(),
        // The real one needs the handle of a box that scrolls, which the canvas
        // has no view to hold: a bar of the right shape stands in.
        Some("Scrollbar::new") => {
            div().w(px(6.)).h(px(48.)).rounded_full().bg(theme::hover_bg()).into_any_element()
        }
        // The variant is read back off the node, so the canvas shows the icon
        // the file will draw — and an unknown one is drawn as the fallback
        // rather than guessed at.
        Some("Icon::new") => {
            apply(icon_named(&base_source(node, 0)), &node.calls).into_any_element()
        }
        // Two components the canvas cannot build for real: both take an
        // `Entity<…State>` the project's view owns, and maxx has no such view
        // to hand them. A faithful lookalike, like the text input above.
        Some("Slider::new") => apply(div(), &node.calls)
            .h(px(20.))
            .flex()
            .items_center()
            .child(
                div()
                    .flex_1()
                    .h(px(4.))
                    .rounded_full()
                    .bg(theme::hover_bg())
                    .child(div().w_1_2().h(px(4.)).rounded_full().bg(theme::accent())),
            )
            .into_any_element(),
        Some("ColorPicker::new") => apply(h_flex(), &node.calls)
            .gap_2()
            .items_center()
            .child(
                div()
                    .size(px(16.))
                    .rounded_sm()
                    .border_1()
                    .border_color(preview.border())
                    // The chip stands for "a colour", not for a role of the
                    // project: it is a placeholder and keeps maxx's accent.
                    .bg(theme::accent()),
            )
            .child(
                div()
                    .text_xs()
                    .text_color(preview.text_muted())
                    .child(SharedString::from(call_text(node, "label", ""))),
            )
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
            .border_color(preview.border())
            .bg(preview.bg())
            .text_color(preview.text_muted())
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

/// The tooltip text a node carries, when maxx wrote it.
pub(super) fn node_tooltip(node: &Node) -> Option<SharedString> {
    let source = node.call("tooltip")?.args.first()?.to_source();
    registry::tooltip_text(&source).map(SharedString::from)
}

/// The icon a `IconName::…` path names, as the canvas can draw it.
///
/// A table rather than a parse: `IconName` has no `FromStr`, and the eighty-odd
/// variants are not all offered anyway. `tests/catalogue.rs` holds this list
/// and `registry::ICONS` to each other, so an icon the inspector offers is
/// always one the canvas can show.
pub(super) fn icon_named(source: &str) -> gpui_component::Icon {
    use gpui_component::IconName;
    let name = match source {
        "IconName::Check" => IconName::Check,
        "IconName::Close" => IconName::Close,
        "IconName::Search" => IconName::Search,
        "IconName::Settings" => IconName::Settings,
        "IconName::Plus" => IconName::Plus,
        "IconName::Minus" => IconName::Minus,
        "IconName::Info" => IconName::Info,
        "IconName::TriangleAlert" => IconName::TriangleAlert,
        "IconName::CircleCheck" => IconName::CircleCheck,
        "IconName::CircleX" => IconName::CircleX,
        "IconName::Star" => IconName::Star,
        "IconName::Heart" => IconName::Heart,
        "IconName::Bell" => IconName::Bell,
        "IconName::Calendar" => IconName::Calendar,
        "IconName::File" => IconName::File,
        "IconName::Folder" => IconName::Folder,
        "IconName::Globe" => IconName::Globe,
        "IconName::User" => IconName::User,
        "IconName::Copy" => IconName::Copy,
        "IconName::Delete" => IconName::Delete,
        "IconName::Eye" => IconName::Eye,
        "IconName::ArrowRight" => IconName::ArrowRight,
        // Anything else is a variant written by hand, which the canvas cannot
        // resolve: an asterisk stands in rather than a wrong icon being drawn.
        _ => IconName::Asterisk,
    };
    gpui_component::Icon::new(name)
}
