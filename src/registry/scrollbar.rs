//! The scrollbar assembly: a scrolling box and its bar, which are one thing on
//! the canvas and two in the source.

use crate::model::{Arg, Base, Node};

/// Wraps `box_node` in what a visible scrollbar needs, and answers the wrapper.
///
/// The shape is not a choice: in gpui, `Div::prepaint` moves **every** child by
/// the scroll offset — an absolutely positioned one included — so a bar drawn
/// inside the box that scrolls travels with the content and leaves the screen.
/// `gpui-component` mounts its own the only way that works, and this is it: a
/// `relative` wrapper holding two children, the box that scrolls and, over it,
/// the bar.
///
/// Nothing of the box moves into anything: it keeps its style, its gap, its
/// children and its id. That is what the wrapper buys — the alternative, moving
/// the layout onto an inner element, is what makes `overflow_y_scrollbar()`
/// lose the container's `gap` and alignment.
///
/// `field` and `bar_id` come from the caller because neither can be decided
/// from one node: a field no other component is bound to, and an element id no
/// sibling is using.
pub fn scrollbar_assembly(mut box_node: Node, ids: [&str; 2], field: &str) -> Node {
    let [box_id, bar_id] = ids;

    // The id first, and that is not a matter of taste: `overflow_*_scroll` and
    // `track_scroll` live on a *stateful* element, which a `div` only becomes
    // once it carries one. A chain that names them before the id does not
    // compile — in the developer's project, on a line maxx wrote — and calls
    // are emitted in the order they are set.
    if box_node.call("id").is_none() {
        box_node.set_call("id", Arg::Str(box_id.to_string()));
    }

    // A bar over a box that does not scroll watches something that never
    // moves: the switch says "show the bar", and scrolling is what a bar is
    // for, so the box is made to scroll. The axis follows its own direction —
    // a row scrolls sideways.
    if box_node.call("overflow_x_scroll").is_none() && box_node.call("overflow_y_scroll").is_none()
    {
        box_node.set_flag(
            if box_node.base.path() == Some("h_flex") {
                "overflow_x_scroll"
            } else {
                "overflow_y_scroll"
            },
            true,
        );
    }
    let horizontal = box_node.call("overflow_x_scroll").is_some();

    // And nothing scrolls inside a box whose size follows its content: it
    // grows instead, and the window cuts it.
    let hold = if horizontal { "w_full" } else { "h_full" };
    let size = if horizontal { "w" } else { "h" };
    if box_node.call(size).is_none() && box_node.call("size_full").is_none() {
        box_node.set_flag(hold, true);
    }

    box_node.set_call("track_scroll", Arg::Verbatim(format!("&self.{field}")));

    let mut bar = Node::known("Scrollbar::new");
    if let Base::Known { args, .. } = &mut bar.base {
        *args = vec![Arg::Verbatim(format!("&self.{field}"))];
    }
    bar.set_call("id", Arg::Str(bar_id.to_string()));
    // The axis the box actually scrolls on: a vertical bar over a row is a bar
    // that tracks nothing.
    bar.set_call(
        "axis",
        Arg::Verbatim(
            if horizontal { "ScrollbarAxis::Horizontal" } else { "ScrollbarAxis::Vertical" }
                .to_string(),
        ),
    );

    // The overlay: `Scrollbar` is not `Styled`, so what positions it is the
    // `div` around it.
    let mut overlay = Node::known("div");
    for flag in ["absolute", "top_0", "left_0", "right_0", "bottom_0"] {
        overlay.set_flag(flag, true);
    }
    overlay.push_child(bar);

    let mut wrapper = Node::known("div");
    wrapper.set_flag("relative", true);
    // The wrapper takes the size the box was holding: it is what stands in the
    // parent's layout now, and a box that grows with its content scrolls
    // nothing.
    wrapper.set_flag(if horizontal { "w_full" } else { "h_full" }, true);
    wrapper.push_child(box_node);
    wrapper.push_child(overlay);
    wrapper
}

/// Whether `node` is a wrapper maxx wrote around a scrolling box.
///
/// Recognised by shape, and strictly: a `div` that is `relative`, holding
/// exactly the box and an overlay whose only child is a `Scrollbar`. Anything
/// the developer added to either goes through this test — and fails it, which
/// is the point: what maxx did not write, maxx does not take away.
pub fn is_scrollbar_wrapper(node: &Node) -> bool {
    if node.base.path() != Some("div") || node.call("relative").is_none() {
        return false;
    }
    let [_, overlay] = node.children.as_slice() else {
        return false;
    };
    overlay.base.path() == Some("div")
        && overlay.call("absolute").is_some()
        && matches!(overlay.children.as_slice(), [bar] if bar.base.path() == Some("Scrollbar::new"))
}

/// Takes the wrapper apart and answers the box it held, bar removed.
pub fn unwrap_scrollbar(wrapper: &Node) -> Option<Node> {
    if !is_scrollbar_wrapper(wrapper) {
        return None;
    }
    let mut box_node = wrapper.children.first()?.clone();
    box_node.remove_call("track_scroll");
    // What was written above the wrapper was written about this box: taking
    // the wrapper away must not take the sentence with it.
    let mut comments = wrapper.comments.clone();
    comments.extend(box_node.comments);
    box_node.comments = comments;
    box_node.trailing.extend(wrapper.trailing.clone());
    Some(box_node)
}

/// The call that holds the axis a scrolling box scrolls on.
///
/// Nothing scrolls inside a box whose size follows its own content: it grows
/// instead, and the window cuts it. The property owns this call as much as the
/// overflow one — which is why it removes it too, and why the inspector shows
/// neither among the calls it does not know.
pub(super) fn hold_for(axis: &str) -> &'static str {
    if axis == "overflow_x_scroll" { "w_full" } else { "h_full" }
}
