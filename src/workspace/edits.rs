//! Editing the tree: what a drop, a duplication, a paste or a deletion does to
//! it, and the checkpoints that let it be undone.

use super::*;

impl Workspace {
    /// Removes a call from the selected node.
    pub fn remove_call_at_selection(&mut self, name: &str, cx: &mut Context<Self>) {
        let Some(view) = self.view() else {
            return;
        };
        let selected = view.selected.clone();
        if view.root.at(&selected).is_none() {
            return;
        }
        self.checkpoint();
        let view = self.view_mut().expect("just borrowed");
        if let Some(node) = view.root.at_mut(&selected) {
            node.remove_call(name);
        }
        cx.notify();
    }

    /// Records the current tree so the change about to be made can be undone.
    pub(super) fn checkpoint(&mut self) {
        self.revision += 1;
        if let Some(view) = self.view_mut() {
            let snapshot = view.root.clone();
            view.past.push(snapshot);
            view.future.clear();
        }
    }

    /// Inserts a component into the selected container, or beside the selected
    /// node when it cannot hold children.
    /// Where a new node goes, given the selection.
    ///
    /// Inside the selected node when it takes children, just after it
    /// otherwise. Answers `None`, having said why, when there is nowhere: the
    /// root is not a container, so nothing can be dropped beside it.
    pub(super) fn insertion_point(&mut self, cx: &mut Context<Self>) -> Option<crate::model::Path> {
        let view = self.view()?;
        let selected = view.selected.clone();
        let target = view.root.at(&selected)?;
        let accepts_children = registry::of(target).is_some_and(|spec| spec.container);

        if accepts_children {
            let mut path = selected;
            path.push(target.children.len());
            return Some(path);
        }
        if selected.is_empty() {
            self.message = Some(crate::tr("message.root_takes_no_child"));
            cx.notify();
            return None;
        }
        let mut path = selected;
        let last = path.last_mut().expect("not empty");
        *last += 1;
        Some(path)
    }

    /// Copies the selected node, and everything under it, next to itself.
    ///
    /// The copy is re-bound before it lands: a duplicated text input that kept
    /// `&self.field` would mirror the original at runtime while compiling
    /// perfectly, which is the worst kind of defect to ship from a copy
    /// gesture.
    pub fn duplicate_selected(&mut self, cx: &mut Context<Self>) {
        let Some(view) = self.view() else {
            return;
        };
        if view.selected.is_empty() {
            self.message = Some(crate::tr("message.root_not_duplicated"));
            cx.notify();
            return;
        }
        let Some(mut copy) = view.root.at(&view.selected).cloned() else {
            return;
        };
        registry::rebind_state_fields(&mut copy, &view.root);

        // Beside the original, not inside it: duplicating a column that takes
        // children would otherwise nest the copy in the original.
        let mut destination = view.selected.clone();
        *destination.last_mut().expect("not the root") += 1;

        self.checkpoint();
        let view = self.view_mut().expect("just borrowed");
        if view.root.insert(&destination, copy) {
            view.selected = destination;
        }
        cx.notify();
    }

    /// Puts the selected node on the clipboard, as Rust source.
    ///
    /// Rust and not a private format: what is copied here pastes into Zed, and
    /// what is written there pastes back. The clipboard is one more place where
    /// the `.rs` is the truth.
    pub fn copy_selection(&mut self, cx: &mut Context<Self>) {
        let Some(view) = self.view() else {
            return;
        };
        let Some(node) = view.root.at(&view.selected) else {
            return;
        };
        let source = crate::codegen::render(node, 0);
        cx.write_to_clipboard(gpui::ClipboardItem::new_string(source));
        self.message = Some(crate::tr("message.node_copied"));
        cx.notify();
    }

    /// Reads a builder expression from the clipboard and inserts it.
    ///
    /// An expression maxx cannot read is refused rather than dropped in as an
    /// opaque node: pasting is a deliberate gesture, and silently turning it
    /// into something unmodifiable would be a surprise.
    pub fn paste_node(&mut self, cx: &mut Context<Self>) {
        let Some(text) = cx.read_from_clipboard().and_then(|item| item.text()) else {
            self.message = Some(crate::tr("message.clipboard_empty"));
            cx.notify();
            return;
        };
        let mut node = match crate::parser::parse_expr(&text) {
            Ok(node) if !node.is_opaque() => node,
            _ => {
                self.message = Some(crate::tr("message.clipboard_not_expression"));
                cx.notify();
                return;
            }
        };
        let Some(destination) = self.insertion_point(cx) else {
            return;
        };
        let Some(view) = self.view() else {
            return;
        };
        registry::rebind_state_fields(&mut node, &view.root);

        self.checkpoint();
        let view = self.view_mut().expect("just borrowed");
        if view.root.insert(&destination, node) {
            view.selected = destination;
        }
        cx.notify();
    }

    /// Drops a sub-tree template in, as one gesture.
    ///
    /// The clipboard path, with the source coming from a table instead of the
    /// system: `parse_expr` is what reads both, which is the whole reason this
    /// needed no new machinery. A template maxx cannot read is a defect in the
    /// table and says so, where a clipboard that cannot be read is only a
    /// clipboard holding something else.
    pub fn insert_subtree(&mut self, id: &str, cx: &mut Context<Self>) {
        let Some((_, _, source)) =
            crate::scaffold::templates::SUBTREES.iter().find(|(this, _, _)| *this == id)
        else {
            return;
        };
        let node = match crate::parser::parse_expr(source) {
            Ok(node) if !node.is_opaque() => node,
            _ => {
                self.message = Some(SharedString::from(
                    t!("message.template_unreadable", name = id).into_owned(),
                ));
                cx.notify();
                return;
            }
        };
        let Some(destination) = self.insertion_point(cx) else {
            return;
        };

        self.checkpoint();
        let Some(view) = self.view_mut() else {
            return;
        };
        if view.root.insert(&destination, node) {
            view.selected = destination;
        }
        cx.notify();
    }

    pub fn insert_component(&mut self, id: &str, cx: &mut Context<Self>) {
        let Some(mut node) = registry::instantiate(id) else {
            return;
        };
        let Some(view) = self.view() else {
            return;
        };

        // Two inputs sharing `&self.champ` compile but mirror each other at
        // runtime, so each one gets its own field. The same holds for every
        // component backed by an entity — a dropdown, a slider, a colour
        // picker: they are not values but state the view owns.
        if registry::by_id(id).is_some_and(|spec| spec.state.is_some()) {
            let field = registry::unique_input_field(&view.root);
            if let crate::model::Base::Known { args, .. } = &mut node.base {
                *args = vec![crate::model::Arg::Verbatim(format!("&self.{field}"))];
            }
        }

        let Some(destination) = self.insertion_point(cx) else {
            return;
        };

        self.checkpoint();
        let view = self.view_mut().expect("just borrowed");
        if view.root.insert(&destination, node) {
            view.selected = destination;
        }
        cx.notify();
    }

    /// Handles a drop on the insertion point `index` of the container at
    /// `parent`.
    pub fn drop_at(
        &mut self,
        parent: &[usize],
        index: usize,
        dragged: crate::designer::Dragged,
        cx: &mut Context<Self>,
    ) {
        let mut destination = parent.to_vec();
        destination.push(index);

        match dragged {
            crate::designer::Dragged::Component(id) => {
                let Some(mut node) = registry::instantiate(id) else {
                    return;
                };
                let Some(view) = self.view() else {
                    return;
                };
                if id == "input" {
                    let field = registry::unique_input_field(&view.root);
                    if let crate::model::Base::Known { args, .. } = &mut node.base {
                        *args = vec![crate::model::Arg::Verbatim(format!("&self.{field}"))];
                    }
                }
                self.checkpoint();
                let view = self.view_mut().expect("just borrowed");
                if view.root.insert(&destination, node) {
                    view.selected = destination;
                }
            }
            crate::designer::Dragged::Node(from) => {
                // Dropping a node back where it already is, or into itself.
                if from == destination {
                    return;
                }
                if destination.len() > from.len() && destination.starts_with(&from) {
                    self.message = Some(crate::tr("message.node_into_itself"));
                    cx.notify();
                    return;
                }
                self.checkpoint();
                let view = self.view_mut().expect("a view is open to be dropped on");
                match view.root.move_node(&from, &destination) {
                    Some(landed) => view.selected = landed,
                    None => {
                        // Nothing moved: drop the checkpoint we just took.
                        view.past.pop();
                    }
                }
            }
        }
        cx.notify();
    }

    /// Deletes the selected node. The root is never deleted.
    pub fn delete_selected(&mut self, cx: &mut Context<Self>) {
        let Some(view) = self.view() else {
            return;
        };
        if view.selected.is_empty() {
            return;
        }
        let selected = view.selected.clone();
        self.checkpoint();
        let view = self.view_mut().expect("just borrowed");
        if view.root.remove(&selected).is_some() {
            view.selected = selected[..selected.len() - 1].to_vec();
        }
        cx.notify();
    }

    /// Steps back one edit.
    ///
    /// The text edit in progress, if any, is closed first: without that, `⌘Z`
    /// typed while a field still holds the focus would step over the text just
    /// written — and the snapshot the field is holding would then be pushed on
    /// top of a history it no longer describes.
    pub fn undo(&mut self, cx: &mut Context<Self>) {
        self.close_text_edit(cx);
        self.revision += 1;
        let Some(view) = self.view_mut() else {
            return;
        };
        if let Some(previous) = view.past.pop() {
            let replaced = std::mem::replace(&mut view.root, previous);
            view.future.push(replaced);
            clamp_selection(view);
            cx.notify();
        }
    }

    /// Steps forward one edit.
    pub fn redo(&mut self, cx: &mut Context<Self>) {
        self.close_text_edit(cx);
        self.revision += 1;
        let Some(view) = self.view_mut() else {
            return;
        };
        if let Some(next) = view.future.pop() {
            let replaced = std::mem::replace(&mut view.root, next);
            view.past.push(replaced);
            clamp_selection(view);
            cx.notify();
        }
    }
}
