//! The canvas: the view as it will be drawn, from the tree maxx holds.

use gpui::prelude::*;
use gpui::{AnyElement, Axis, Context, DragMoveEvent, Entity, SharedString, div, img, px};
use gpui_component::alert::Alert;
use gpui_component::button::Button;
use gpui_component::button::ButtonVariants as _;
use gpui_component::checkbox::Checkbox;
use gpui_component::divider::Divider;
use gpui_component::group_box::GroupBox;
use gpui_component::input::{Input, InputState};
use gpui_component::label::Label;
use gpui_component::menu::ContextMenuExt as _;
use gpui_component::scroll::Scrollbar;
use gpui_component::switch::Switch;
use gpui_component::{Sizable as _, h_flex, v_flex};

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
        let board = Board {
            selected: &view.selected,
            root,
            preview: &self.preview,
            editing: self.canvas_edit(),
        };
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
                            .child(node_element(&view.root, &[], &board, cx)),
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
            // One menu for the whole board, acting on the selection that the
            // right click has just moved. Per node it could not work at all:
            // `ContextMenuExt::context_menu` hard-codes the id of what it opens.
            .context_menu(super::node_menu)
    }
}

/// What a node needs to know about the board it is being drawn on.
///
/// One value rather than four more parameters, and it is not tidiness: the two
/// functions that draw a node call each other, so an argument added to one is
/// added to both — and the pair had already reached the count clippy stops at.
pub(super) struct Board<'a> {
    /// The selected node: the one wearing the chrome and the handles.
    pub selected: &'a [usize],
    /// The project root, against which an image path is resolved.
    pub root: Option<&'a std::path::Path>,
    /// The colours the project paints itself with.
    pub preview: &'a Preview,
    /// The node being typed into, and the box typing it.
    pub editing: Option<(&'a [usize], &'a Entity<InputState>)>,
}

/// A handle of the selected node being pulled, and which edge it is.
#[derive(Clone, Copy, Debug)]
pub(super) struct Grab(Axis);

/// What follows the cursor while an edge is being pulled: nothing.
///
/// gpui asks every drag for something to draw, and here the thing being dragged
/// is already on screen — it is the node, growing. A label beside the cursor
/// would name what anyone can see and cover the edge being aimed at.
pub(super) struct GrabGhost;

impl gpui::Render for GrabGhost {
    fn render(&mut self, _: &mut gpui::Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
    }
}

/// Whether the catalogue lets this node carry a width and a height.
///
/// The handles write `w` and `h`, and a component that is not `Styled` — a
/// spinner, a badge — has neither. A handle drawn there would offer a gesture
/// whose only outcome is a project that stops compiling.
fn takes_a_size(node: &Node) -> bool {
    registry::of(node).is_some_and(|spec| {
        registry::props(spec).into_iter().any(|prop| {
            matches!(prop.target, registry::Target::Method("w" | "h"))
                && matches!(prop.kind, registry::Kind::Number)
        })
    })
}

/// Renders one node on the canvas, wrapped in its selection chrome.
pub(super) fn node_element(
    node: &Node,
    path: &[usize],
    board: &Board,
    cx: &mut Context<Workspace>,
) -> AnyElement {
    let is_selected = path == board.selected;
    let target = path.to_vec();
    let menu_target = path.to_vec();

    let dragged_label = node_label(node);
    let drag_path = path.to_vec();
    // The box stands over the node whose words it writes, so it is drawn by
    // that node and by no other.
    let editing = board.editing.filter(|(edited, _)| *edited == path).map(|(_, state)| state);
    let sizable = is_selected && takes_a_size(node);
    let preview = board.preview;

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
        // The listener sits on the node and not on the board, and that is the
        // whole reason it is here: what a drag says is where the cursor is, and
        // a size is that minus the edge it is measured from — which is this
        // element's own bounds. The same arithmetic `gpui_component::resizable`
        // does with its panel's.
        .when(sizable, |this| {
            this.on_drag_move(cx.listener(|this, event: &DragMoveEvent<Grab>, _, cx| {
                let Grab(axis) = *event.drag(cx);
                let size = match axis {
                    Axis::Horizontal => event.event.position.x - event.bounds.left(),
                    Axis::Vertical => event.event.position.y - event.bounds.top(),
                };
                this.resize_selection(axis, f32::from(size), cx);
            }))
        })
        .child(node_preview(node, path, board.root, preview, |vertical| {
            children_with_zones(node, path, board, vertical, cx)
        }))
        .when(sizable, |this| {
            this.child(grab_handle(path, Axis::Horizontal, cx)).child(grab_handle(
                path,
                Axis::Vertical,
                cx,
            ))
        })
        .when_some(editing, |this, state| this.child(text_box(state)))
        .on_click(cx.listener(move |this, event: &gpui::ClickEvent, window, cx| {
            // Every node wraps its children in a listener of its own, so
            // without this the click keeps bubbling and each ancestor
            // re-selects itself — the root always won.
            cx.stop_propagation();
            this.select(target.clone(), cx);
            if is_double_click(event) {
                // The words first: a button carries both a label and an action,
                // and typing what it says is the gesture a double click on a
                // button means everywhere else. Writing a handler is what the
                // same double click still does on a node with nothing to say.
                if !this.edit_text_on_canvas(target.clone(), window, cx) {
                    this.add_handler_to_selection(cx);
                }
            }
        }))
        // The board's menu is about the selected node, so the right click moves
        // the selection first — and stops there, for the reason the left click
        // stops: every ancestor holds a listener of its own, and the root would
        // otherwise always win.
        .on_mouse_down(
            gpui::MouseButton::Right,
            cx.listener(move |this, _, _, cx| {
                cx.stop_propagation();
                this.select(menu_target.clone(), cx);
            }),
        )
        .into_any_element()
}

/// One edge of the selected node, there to be pulled.
///
/// Two of them, the right one and the bottom one, and no corner: a corner would
/// have to write a width and a height from a single gesture, and the two are
/// separate calls meaning separate things — a hand that wanted only one would
/// come away with both.
fn grab_handle(path: &[usize], axis: Axis, cx: &mut Context<Workspace>) -> AnyElement {
    let down = matches!(axis, Axis::Vertical);
    div()
        .id(SharedString::from(format!("grab-{path:?}-{}", if down { "h" } else { "w" })))
        .absolute()
        // Opaque to the mouse, which is what keeps the node underneath from
        // being selected — or dragged off — by a hand aiming at its edge.
        .occlude()
        .bg(theme::accent())
        .when(!down, |this| this.top_0().bottom_0().right(px(-2.)).w(px(4.)).cursor_col_resize())
        .when(down, |this| this.left_0().right_0().bottom(px(-2.)).h(px(4.)).cursor_row_resize())
        .on_mouse_down(gpui::MouseButton::Left, cx.listener(|this, _, _, _| this.grab_handle()))
        // gpui's own drag is what carries the gesture past the edge of the
        // element and keeps it alive wherever in the window the cursor runs.
        .on_drag(Grab(axis), |_, _, _, cx| cx.new(|_| GrabGhost))
        .into_any_element()
}

/// The box a double click opens, standing over the node.
///
/// Above it rather than on top of it: a field drawn across the words it is
/// editing hides the one thing the gesture is about — how the node looks as the
/// letters land.
fn text_box(state: &Entity<InputState>) -> AnyElement {
    div()
        .absolute()
        .bottom_full()
        .left_0()
        .pb_1()
        .w(px(180.))
        // Opaque to the mouse, or every click meant for the caret would reach
        // the node underneath — and selecting a node is what closes the box.
        .occlude()
        .child(Input::new(state).small())
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
///
/// It wears the size written on the node, and that is not decoration: the
/// handles put `w(px(…))` on whatever is selected, and a frame that answered a
/// gesture by not moving would read as a handle that does not work. Only the
/// size — the colours stay the placeholder's, since what it says is precisely
/// that the picture is not there.
pub(crate) fn missing_image(calls: &[crate::model::Call]) -> AnyElement {
    apply(div().h(px(60.)).w(px(90.)), calls)
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

/// A tree drawn as a still picture: no selection, no drag, no drop zone.
///
/// The welcome screen's thumbnails go through here. They show a project that is
/// **not open**, so every listener the board carries would act on the wrong tree
/// — or on no tree at all. What is shared with the board is the only thing worth
/// sharing: [`node_preview`], the table that says what each component looks
/// like.
pub(crate) fn still_node(
    node: &Node,
    path: &[usize],
    root: Option<&std::path::Path>,
    preview: &Preview,
) -> AnyElement {
    node_preview(node, path, root, preview, |_| {
        node.children
            .iter()
            .enumerate()
            .map(|(index, child)| {
                let mut child_path = path.to_vec();
                child_path.push(index);
                still_node(child, &child_path, root, preview)
            })
            .collect()
    })
}

/// The component itself, as it will look once generated.
///
/// Its children arrive through a closure rather than being read off the node,
/// because that is the whole difference between the two surfaces that draw a
/// tree: the board interleaves drop zones between them, a thumbnail does not.
/// The closure is handed the direction the container stacks in, which only the
/// branch drawing it knows. It is called by the containers alone, so a leaf
/// still costs nothing.
pub(super) fn node_preview(
    node: &Node,
    path: &[usize],
    root: Option<&std::path::Path>,
    preview: &Preview,
    children: impl FnOnce(bool) -> Vec<AnyElement>,
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
            with_hover(apply_placement(apply(base, &node.calls), &node.calls), node)
                .min_h(px(16.))
                .children(children(vertical))
                .into_any_element()
        }
        // Every branch below runs its component through `apply`, and the reason
        // is the handles: they write `w(px(…))` and `h(px(…))` on whatever is
        // selected, and a component drawn without the style calls answers a
        // gesture by not moving. The catalogue already says which components
        // accept the shared box — `Common::None` has no width to write — so
        // what is applied here is exactly what the file will hold.
        Some("Label::new") => apply(Label::new(text(0)), &node.calls).into_any_element(),
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
                Some(root) => {
                    let calls = node.calls.clone();
                    apply(img(root.join(&source)), &node.calls)
                        .map(|image| match object_fit(node) {
                            Some(fit) => image.object_fit(fit),
                            None => image,
                        })
                        .with_fallback(move || missing_image(&calls))
                        .into_any_element()
                }
                None => missing_image(&node.calls),
            }
        }
        // A drawing, on the same path as the image above and joined onto the
        // project the same way — but drawn with `img` rather than with `svg`.
        // `Svg::path` hands its text to the application's `AssetSource`, and on
        // this canvas that source is maxx's own: the generated binary would
        // find the file and the workshop would not. gpui's image loader reads
        // `.svg` from disk (`Img::extensions` names it), so the picture is the
        // right one. What it cannot show is the tint: `img` rasterizes the file
        // with the colours it holds, where the generated view paints it in
        // `text_color`. A drawing in the wrong colour is a smaller lie than no
        // drawing at all.
        Some("svg") => {
            let source = call_text(node, "path", "");
            match root.filter(|_| !source.is_empty()) {
                Some(root) => {
                    let calls = node.calls.clone();
                    apply(img(root.join(&source)).size(px(16.)), &node.calls)
                        .with_fallback(move || missing_image(&calls))
                        .into_any_element()
                }
                None => missing_image(&node.calls),
            }
        }
        Some("Checkbox::new") => apply(
            Checkbox::new(SharedString::from(format!("preview-{path:?}")))
                .label(call_text(node, "label", &crate::tr("component.checkbox")))
                .checked(call_bool(node, "checked")),
            &node.calls,
        )
        .into_any_element(),
        Some("Switch::new") => apply(
            Switch::new(SharedString::from(format!("preview-{path:?}")))
                .label(call_text(node, "label", &crate::tr("component.switch")))
                .checked(call_bool(node, "checked")),
            &node.calls,
        )
        .into_any_element(),
        Some("GroupBox::new") => apply(
            GroupBox::new().title(call_text(node, "title", &crate::tr("component.group_box"))),
            &node.calls,
        )
        .children(children(true))
        .into_any_element(),
        Some("Divider::horizontal") => {
            let divider = match node.call("label") {
                Some(_) => Divider::horizontal().label(call_text(node, "label", "")),
                None => Divider::horizontal(),
            };
            apply(divider, &node.calls).into_any_element()
        }
        // A hand-written `div()` can hold children; only an empty one is the
        // spacer the palette drops.
        Some("div") if node.children.is_empty() => {
            with_hover(apply(div(), &node.calls), node).h(px(20.)).into_any_element()
        }
        Some("div") => with_hover(apply_placement(apply(div(), &node.calls), &node.calls), node)
            .children(children(true))
            .into_any_element(),
        Some("Button::new") => apply(
            Button::new(SharedString::from(format!("preview-{path:?}"))).label(call_text(
                node,
                "label",
                &crate::tr("component.button"),
            )),
            &node.calls,
        )
        .into_any_element(),
        Some("Radio::new") => apply(
            gpui_component::radio::Radio::new(SharedString::from(format!("preview-{path:?}")))
                .label(call_text(node, "label", &crate::tr("component.radio")))
                .checked(call_bool(node, "checked")),
            &node.calls,
        )
        .into_any_element(),
        Some("Alert::new") => apply(
            Alert::new(
                SharedString::from(format!("preview-{path:?}")),
                SharedString::from(text(1).to_string()),
            )
            .title(call_text(node, "title", "")),
            &node.calls,
        )
        .into_any_element(),
        Some("Progress::new") => apply(
            gpui_component::progress::Progress::new()
                .value(call_number(node, "value").unwrap_or(0.)),
            &node.calls,
        )
        .into_any_element(),
        // Two containers whose content is a child of the model rather than an
        // argument: their preview therefore has to carry the drop zones.
        Some("Link::new") => apply(
            gpui_component::link::Link::new(SharedString::from(format!("preview-{path:?}"))),
            &node.calls,
        )
        .children(children(false))
        .into_any_element(),
        Some("Tag::new") => {
            apply(tag_variant(node), &node.calls).children(children(false)).into_any_element()
        }
        Some("Badge::new") => gpui_component::badge::Badge::new()
            .count(call_whole(node, "count").unwrap_or(0))
            // Both numbers, or a count of 30 beyond 9 still draws “30” on the
            // canvas and “9+” in the running application.
            .max(call_whole(node, "max").unwrap_or(99))
            .children(children(false))
            .into_any_element(),
        Some("Skeleton::new") => {
            apply(gpui_component::skeleton::Skeleton::new(), &node.calls).into_any_element()
        }
        Some("Spinner::new") => gpui_component::spinner::Spinner::new().into_any_element(),
        // The picture is joined onto the project, like the image above and for
        // the same reason: the string is what the generated binary hands its
        // own `AssetSource`, and maxx's would answer with its own files.
        Some("Avatar::new") => {
            let mut avatar = gpui_component::avatar::Avatar::new();
            let name = call_text(node, "name", "");
            if !name.is_empty() {
                avatar = avatar.name(SharedString::from(name));
            }
            let source = call_text(node, "src", "");
            if let Some(root) = root.filter(|_| !source.is_empty()) {
                avatar = avatar.src(root.join(&source));
            }
            // The size is `Sizable`, not `Styled`, so `apply` cannot carry it:
            // an avatar left small on the canvas and drawn large in the
            // application is a switch that reads as broken.
            avatar = match () {
                _ if node.call("xsmall").is_some() => avatar.xsmall(),
                _ if node.call("small").is_some() => avatar.small(),
                _ if node.call("large").is_some() => avatar.large(),
                _ => avatar,
            };
            apply(avatar, &node.calls).into_any_element()
        }
        // The labels are read back off the node, so what the canvas draws is
        // what the file holds — and a list written by hand, which maxx cannot
        // read, draws nothing rather than something invented.
        Some("Breadcrumb::new") => {
            apply(gpui_component::breadcrumb::Breadcrumb::new(), &node.calls)
                .children(labels(node, "children"))
                .into_any_element()
        }
        Some("TabBar::new") => {
            let bar =
                gpui_component::tab::TabBar::new(SharedString::from(format!("preview-{path:?}")))
                    .children(labels(node, "children"))
                    .with_variant(tab_variant(node));
            apply(bar, &node.calls)
                .map(|bar| match call_whole(node, "selected_index") {
                    Some(index) => bar.selected_index(index),
                    None => bar,
                })
                .into_any_element()
        }
        Some("Kbd::new") => {
            let stroke = registry::keystroke_text(&base_source(node, 0)).unwrap_or_default();
            apply(
                gpui_component::kbd::Kbd::new(gpui::Keystroke::parse(&stroke).unwrap_or_default()),
                &node.calls,
            )
            .into_any_element()
        }
        // The real one carries a click that stops propagation, so the node
        // could never be selected again: what stands in is the button
        // `Clipboard` itself draws, without the copying.
        Some("Clipboard::new") => Button::new(SharedString::from(format!("preview-{path:?}")))
            .icon(gpui_component::IconName::Copy)
            .ghost()
            .xsmall()
            .into_any_element(),
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
        // Four more the canvas cannot build for real, for the reason the slider
        // and the colour picker cannot: each takes an `Entity<…State>` the
        // project's view owns, and `node_preview` has no context to make one in.
        // What stands in is drawn from the node, so the properties stay
        // verifiable without `cargo run`.
        Some("DatePicker::new") => field_frame(preview)
            .justify_between()
            .child(SharedString::from(call_text(
                node,
                "placeholder",
                &crate::tr("component.date_picker"),
            )))
            .child(icon_named("IconName::Calendar").text_color(preview.text_muted()))
            .into_any_element(),
        Some("NumberInput::new") => field_frame(preview)
            .justify_between()
            .child(SharedString::from(call_text(node, "placeholder", "0")))
            .child(
                v_flex()
                    .child(icon_named("IconName::ChevronUp").text_color(preview.text_muted()))
                    .child(icon_named("IconName::ChevronDown").text_color(preview.text_muted())),
            )
            .into_any_element(),
        // Six boxes, because six is the length the initializer writes; the
        // groups are read off the node, so the property shows what it does.
        Some("OtpInput::new") => {
            let groups = call_whole(node, "groups").unwrap_or(1).clamp(1, 6);
            let per_group = 6usize.div_ceil(groups);
            h_flex()
                .gap_2()
                .children((0..groups).map(|group| {
                    let from = group * per_group;
                    let count = per_group.min(6usize.saturating_sub(from));
                    h_flex().gap_1().children((0..count).map(|_| {
                        div()
                            .size(px(20.))
                            .rounded_sm()
                            .border_1()
                            .border_color(preview.border())
                            .bg(preview.bg())
                    }))
                }))
                .into_any_element()
        }
        // A month, drawn as the shape of one: the header and thirty-five cells.
        // Not the real dates — the state holds those, and inventing them would
        // show a month the application will not.
        Some("Calendar::new") => apply(v_flex(), &node.calls)
            .gap_1()
            .p_2()
            .rounded_md()
            .border_1()
            .border_color(preview.border())
            .child(
                div().text_xs().text_color(preview.text()).child(crate::tr("component.calendar")),
            )
            .children((0..5).map(|_| {
                h_flex().gap_1().children(
                    (0..7).map(|_| div().size(px(12.)).rounded_sm().bg(theme::hover_bg())),
                )
            }))
            .into_any_element(),
        // A live text input on the canvas would swallow the clicks the designer
        // needs, so the preview is a faithful lookalike.
        Some("Input::new") => apply(field_frame(preview), &node.calls)
            .child(SharedString::from(text(0).to_string()))
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

/// Puts the node's hover chain on the element, so the board shows it.
///
/// The point of drawing it at all: a colour that only appears in the running
/// application is a property nobody can check without `cargo run`, which is the
/// one thing the canvas exists to spare. The board already receives the pointer,
/// so it costs a closure.
///
/// `StyleRefinement` implements `Styled`, so the very function that applies the
/// ordinary calls applies these — the two states cannot come to draw a
/// background differently.
fn with_hover<E: gpui::InteractiveElement>(element: E, node: &Node) -> E {
    match registry::hover_calls(node) {
        Some((_, chain)) => element.hover(move |style| apply(style, &chain.calls)),
        None => element,
    }
}

/// The box every field-shaped lookalike is drawn in.
///
/// One frame for four components — a text field, a number field, a date picker
/// — because they *are* one frame in `gpui-component`: three of them wrap the
/// same `Input`. Drawing three near-identical boxes by hand is three places for
/// them to drift apart on the canvas while staying identical in the file.
fn field_frame(preview: &Preview) -> gpui::Div {
    h_flex()
        .h(px(30.))
        .px_2()
        .gap_2()
        .items_center()
        .rounded_md()
        .border_1()
        .border_color(preview.border())
        .bg(preview.bg())
        .text_color(preview.text_muted())
}

/// The tooltip text a node carries, when maxx wrote it.
pub(super) fn node_tooltip(node: &Node) -> Option<SharedString> {
    let source = node.call("tooltip")?.args.first()?.to_source();
    registry::tooltip_text(&source).map(SharedString::from)
}

/// The labels a list call holds, as the canvas can draw them.
///
/// Empty for a call maxx did not write — `.children(self.crumbs.clone())` — for
/// the reason an unreadable icon is drawn as the fallback: guessing at an
/// expression would show a bar the file does not have.
fn labels(node: &Node, name: &str) -> Vec<SharedString> {
    node.call(name)
        .and_then(|call| call.args.first())
        .and_then(|arg| registry::label_texts(&arg.to_source()))
        .unwrap_or_default()
        .into_iter()
        .map(SharedString::from)
        .collect()
}

/// The variant written on a tab bar, read back as the crate's own value.
fn tab_variant(node: &Node) -> gpui_component::tab::TabVariant {
    use gpui_component::tab::TabVariant;
    match node.call("with_variant").and_then(|call| call.args.first()).map(|arg| arg.to_source()) {
        Some(source) => match source.as_str() {
            "TabVariant::Outline" => TabVariant::Outline,
            "TabVariant::Pill" => TabVariant::Pill,
            "TabVariant::Segmented" => TabVariant::Segmented,
            "TabVariant::Underline" => TabVariant::Underline,
            // What `TabBar::new` gives, and what an unreadable variant falls
            // back to.
            _ => TabVariant::Tab,
        },
        None => TabVariant::Tab,
    }
}

/// The icon a `IconName::…` path names, as the canvas can draw it.
///
/// The table it reads is generated: `IconName` has no `FromStr`, and a match
/// kept by hand is a match that falls behind the crate — which shows up as an
/// asterisk where an icon should be, and reads as a bug in the icon rather than
/// a hole in a list. `build.rs` writes it from the enum itself, so the
/// eighty-six the inspector offers are the eighty-six drawn here.
pub fn icon_named(source: &str) -> gpui_component::Icon {
    // A name the crate does not carry is one written by hand: an asterisk
    // stands in rather than a wrong icon being drawn.
    gpui_component::Icon::new(icon_name(source).unwrap_or(gpui_component::IconName::Asterisk))
}

include!(concat!(env!("OUT_DIR"), "/icon_name.rs"));
