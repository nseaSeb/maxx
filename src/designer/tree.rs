//! The structure tree: what the view holds, and where a dropped node lands.

use gpui::prelude::*;
use gpui::{AnyElement, Context, SharedString, div, px};
use gpui_component::v_flex;

use crate::model::{Node, Path};
use crate::preview::Preview;
use crate::registry::{self};
use crate::theme;
use gpui::{Pixels, Point};

use crate::workspace::Workspace;

use super::canvas::node_element;
use super::{DragGhost, Dragged, section_title};

impl Workspace {
    /// The node tree, mirroring the canvas selection.
    pub(super) fn render_tree(&self, cx: &mut Context<Self>) -> impl IntoElement {
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
}

/// One row of the structure tree.
pub(super) struct TreeRow {
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
pub(super) fn tree_zone(parent: Path, index: usize, cx: &mut Context<Workspace>) -> AnyElement {
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
pub(super) fn tree_row(row: TreeRow, cx: &mut Context<Workspace>) -> AnyElement {
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
pub(super) fn drop_zone(
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
pub(super) fn children_with_zones(
    node: &Node,
    path: &[usize],
    selected: &[usize],
    vertical: bool,
    root: Option<&std::path::Path>,
    preview: &Preview,
    cx: &mut Context<Workspace>,
) -> Vec<AnyElement> {
    let mut out = Vec::with_capacity(node.children.len() * 2 + 1);
    out.push(drop_zone(path.to_vec(), 0, vertical, cx));
    for (index, child) in node.children.iter().enumerate() {
        let mut child_path = path.to_vec();
        child_path.push(index);
        out.push(node_element(child, &child_path, selected, root, preview, cx));
        out.push(drop_zone(path.to_vec(), index + 1, vertical, cx));
    }
    out
}

/// Name shown for a node in the tree.
pub(super) fn node_label(node: &Node) -> SharedString {
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
