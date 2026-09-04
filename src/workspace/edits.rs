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

    /// Notes that a handle of the canvas has just been taken hold of.
    ///
    /// The step is not taken here but at the first movement, which is the very
    /// same tree — nothing can happen between the two. A handle merely clicked
    /// would otherwise leave a `⌘Z` that undoes nothing, and a stack holding
    /// those is a stack nobody trusts.
    pub fn grab_handle(&mut self) {
        self.resized = false;
    }

    /// Writes the size a handle has just dragged the selected node to.
    ///
    /// Pixels are the only thing a drag can say, so `w(px(…))` and `h(px(…))`
    /// are the only things it writes: a percentage or a `flex_1` would be maxx
    /// guessing at an intention the hand did not express.
    ///
    /// One undo step for the whole gesture, and it is the reason
    /// [`Self::grab_handle`] exists: the checkpoint is taken once, at the first
    /// movement, and a drag writing sixty times a second leaves one step.
    pub fn resize_selection(&mut self, axis: gpui::Axis, size: f32, cx: &mut Context<Self>) {
        // Under a few pixels the node has no edge left to grab, and the handle
        // becomes impossible to take hold of again.
        let size = size.round().max(8.).to_string();
        let method = match axis {
            gpui::Axis::Horizontal => "w",
            gpui::Axis::Vertical => "h",
        };
        let Some(view) = self.view() else {
            return;
        };
        let selected = view.selected.clone();
        let Some(node) = view.root.at(&selected) else {
            return;
        };
        // Asked of the catalogue rather than written by hand: a component that
        // is not `Styled` — a spinner, a badge — accepts no width at all, and a
        // call posed on one is a line the generated project will not compile.
        // The same table the inspector's own width field reads.
        let Some(prop) = registry::of(node).map(registry::props).and_then(|props| {
            props.into_iter().find(|prop| {
                matches!(prop.target, registry::Target::Method(name) if name == method)
                    && matches!(prop.kind, Kind::Number)
            })
        }) else {
            return;
        };
        if registry::read(node, prop).as_deref() == Some(size.as_str()) {
            return;
        }

        if !self.resized {
            self.checkpoint();
            self.resized = true;
        }
        let view = self.view_mut().expect("just borrowed");
        if let Some(node) = view.root.at_mut(&selected) {
            registry::write(node, prop, &size);
        }
        // The panels follow the handle instead of waiting for it to be let go:
        // the code and the width field describe the node the hand is pulling,
        // and a number that lags behind a gesture reads as a wrong number. The
        // rule this looks like it breaks is about typing — no caret is in a
        // field while a handle is being dragged, so nothing is rebuilt under
        // one.
        self.revision += 1;
        cx.notify();
    }

    /// Records the current tree so the change about to be made can be undone.
    pub(super) fn checkpoint(&mut self) {
        // A field may still hold the caret: a command reaches the workspace
        // through a focused input, and a click in a menu blurs nothing. What
        // was typed is a step before this one, and has to be pushed first.
        self.take_text_step();
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

    /// Puts the selected node inside a fresh container of the catalogue.
    ///
    /// The wrapper takes the node's place among its siblings and the node
    /// becomes its only child, in **one** checkpoint: a lift and a drop are two
    /// edits of the tree but a single gesture of the hand, and `⌘Z` has to undo
    /// the gesture rather than half of it.
    ///
    /// The wrapped node keeps its own calls. A `bg` written on a button is a
    /// fact about the button and not about the box just put around it; lifting
    /// the calls up would repaint the wrapper and leave the button bare.
    pub fn wrap_selected(&mut self, id: &str, cx: &mut Context<Self>) {
        let Some(view) = self.view() else {
            return;
        };
        if view.selected.is_empty() {
            self.message = Some(crate::tr("message.root_not_wrapped"));
            cx.notify();
            return;
        }
        let selected = view.selected.clone();
        if view.root.at(&selected).is_none() {
            return;
        }
        // Wrapping one of the two children replaces it, so the shape stops
        // matching exactly as it does when one is moved or promoted. The guard
        // belongs on all three commands, and this is the third.
        if inside_a_scrollbar_assembly(&view.root, &selected) {
            self.message = Some(crate::tr("message.scrollbar_is_one_piece"));
            cx.notify();
            return;
        }
        let Some(mut wrapper) = registry::instantiate(id) else {
            return;
        };

        self.checkpoint();
        let view = self.view_mut().expect("just borrowed");
        let Some(node) = view.root.remove(&selected) else {
            // Nothing moved: drop the checkpoint we just took.
            view.past.pop();
            return;
        };
        wrapper.push_child(node);
        if view.root.insert(&selected, wrapper) {
            // The selection follows the node, not the box: the developer was
            // working on that node and is still working on it.
            let mut inner = selected;
            inner.push(0);
            view.selected = inner;
        }
        cx.notify();
    }

    /// Replaces the selected container with its only child.
    ///
    /// Refuses a container holding anything but exactly one child rather than
    /// promoting one of them: which child survives is not something maxx gets
    /// to guess, and the others would be deleted without a word.
    pub fn unwrap_selected(&mut self, cx: &mut Context<Self>) {
        let Some(view) = self.view() else {
            return;
        };
        if view.selected.is_empty() {
            self.message = Some(crate::tr("message.root_not_unwrapped"));
            cx.notify();
            return;
        }
        let selected = view.selected.clone();
        let Some(node) = view.root.at(&selected) else {
            return;
        };
        if node.children.len() != 1 {
            self.message = Some(crate::tr("message.unwrap_needs_one_child"));
            cx.notify();
            return;
        }
        if inside_a_scrollbar_assembly(&view.root, &selected) {
            self.message = Some(crate::tr("message.scrollbar_is_one_piece"));
            cx.notify();
            return;
        }

        self.checkpoint();
        let view = self.view_mut().expect("just borrowed");
        let Some(container) = view.root.remove(&selected) else {
            view.past.pop();
            return;
        };
        let mut child = container.children.into_iter().next().expect("just counted one");
        // What was written above the box, and after its last call, has nowhere
        // else to go once the box is gone. Losing a sentence nobody asked to
        // lose is the defect `take_call` already exists to avoid.
        //
        // The calls themselves do not follow — a `gap` on the box says nothing
        // about the node that was inside it — but every call carries the lines
        // written above it, and those are the developer's words like any
        // other. They are gathered here, in reading order: what was above the
        // box, then what was inside it, then what the child already had.
        let mut moved = container.comments;
        moved.extend(container.calls.into_iter().flat_map(|call| call.comments));
        moved.extend(std::mem::take(&mut child.comments));
        child.comments = moved;
        child.trailing.extend(container.trailing);
        if view.root.insert(&selected, child) {
            view.selected = selected;
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

    /// Drops one of the project's own components into the view.
    ///
    /// The same road a template takes — an expression parsed and inserted —
    /// with the import the view needs to name the type. What lands is a
    /// `Base::Known`, so it moves, renames and deletes like anything else on
    /// the canvas; a component maxx could not write as a call would land opaque
    /// instead, which is why `bricks` does not offer one.
    pub fn insert_brick(&mut self, type_name: &str, cx: &mut Context<Self>) {
        let Some(brick) = self.bricks.iter().find(|brick| brick.type_name == type_name).cloned()
        else {
            return;
        };
        let source = brick.expression();
        let node = match crate::parser::parse_expr(&source) {
            Ok(node) if !node.is_opaque() => node,
            _ => {
                self.message = Some(SharedString::from(
                    t!("message.template_unreadable", name = brick.type_name).into_owned(),
                ));
                cx.notify();
                return;
            }
        };
        let Some(destination) = self.insertion_point(cx) else {
            return;
        };

        self.checkpoint();
        let import = brick.import();
        let Some(view) = self.view_mut() else {
            return;
        };
        if view.root.insert(&destination, node) {
            view.selected = destination;
            // Remembered rather than written: the file is spliced at the save,
            // and that is where every other import a view owes is worked out.
            // A view naming a type it has not imported does not compile, and it
            // would be maxx's own omission the developer found in Zed.
            if !view.extra_imports.contains(&import) {
                view.extra_imports.push(import);
            }
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
    ///
    /// A template that binds state goes through the copy's own rebinding: the
    /// name written in the table is only a first choice, and the second drop
    /// takes the next one rather than mirroring the first at runtime. Declaring
    /// the field is nobody's job here — `view::render_source` walks the tree at
    /// every save and writes whatever binding it finds, which is why this is one
    /// line and not a feature.
    pub fn insert_subtree(&mut self, id: &str, cx: &mut Context<Self>) {
        let Some((_, _, _, source)) =
            crate::scaffold::templates::SUBTREES.iter().find(|(this, _, _, _)| *this == id)
        else {
            return;
        };
        let mut node = match crate::parser::parse_expr(source) {
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
        let Some(view) = self.view() else {
            return;
        };
        registry::rebind_state_fields(&mut node, &view.root);

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

        bind_own_state_field(id, &mut node, &view.root);

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

    /// The catalogue row the palette's context menu is about.
    pub(crate) fn palette_target(&self) -> Option<&'static str> {
        self.palette_target
    }

    /// Lights a palette row, so the menu opening over it knows what it is about.
    ///
    /// The right click's half of the gesture, exactly as in the tree and in the
    /// project panel: the menu is built on the following frame, out of what
    /// this leaves behind.
    pub(crate) fn target_palette_component(&mut self, id: &'static str, cx: &mut Context<Self>) {
        self.palette_target = Some(id);
        cx.notify();
    }

    /// Inserts the component the palette's menu is about, at `at`.
    ///
    /// Through `drop_at` on purpose: it is the road the drag already takes, so
    /// a component with state is given a field of its own here exactly as it is
    /// when dropped — `bind_own_state_field` is named in one place, and a
    /// second way in must not grow a second copy of that rule.
    ///
    /// The three positions are the tree's three drop zones, offered without the
    /// drag: the strip before the selected row, the strip after it, and the row
    /// itself when it holds children.
    pub fn insert_from_palette(&mut self, at: Insert, cx: &mut Context<Self>) {
        let Some(id) = self.palette_target else {
            self.message = Some(crate::tr("message.select_component_first"));
            cx.notify();
            return;
        };
        let Some(view) = self.view() else {
            return;
        };
        let selected = view.selected.clone();
        let Some(node) = view.root.at(&selected) else {
            return;
        };
        let container = registry::of(node).is_some_and(|spec| spec.container);
        let children = node.children.len();

        let (parent, index) = match (at, selected.split_last()) {
            (Insert::Into, _) if container => (selected, children),
            // Saying so rather than falling back on "after": an entry that
            // reads "inside" and inserts beside would be lying about where the
            // node went, and the tree is right there to show it.
            (Insert::Into, Some(_)) => {
                self.message = Some(crate::tr("message.node_takes_no_child"));
                cx.notify();
                return;
            }
            (Insert::Before, Some((last, above))) => (above.to_vec(), *last),
            (Insert::After, Some((last, above))) => (above.to_vec(), last + 1),
            // The selection is the root, which has no sibling: "before" and
            // "after" are then read as the two ends of what it holds, so the
            // three entries all insert rather than two of them refusing on a
            // view nobody has clicked in yet.
            (Insert::Before, None) if container || children > 0 => (Vec::new(), 0),
            (Insert::After, None) if container || children > 0 => (Vec::new(), children),
            (_, None) => {
                self.message = Some(crate::tr("message.root_takes_no_child"));
                cx.notify();
                return;
            }
        };
        self.drop_at(&parent, index, crate::designer::Dragged::Component(id), cx);
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
                bind_own_state_field(id, &mut node, &view.root);
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

    /// Moves the selected node one place among its siblings.
    ///
    /// A move and not a reparenting: the node stays in the container it is in,
    /// which is what makes `⌥↑` safe to hold down. Reached from the tree's
    /// keyboard as well as from the menu, so both ends of a list have to answer
    /// rather than wrap around — a node that jumped from the top to the bottom
    /// would look like a deletion.
    ///
    /// The selection follows the node: the developer was working on it and is
    /// still working on it, and losing it at every step would make the gesture
    /// unrepeatable.
    pub fn move_selected_node(&mut self, up: bool, cx: &mut Context<Self>) {
        let Some(view) = self.view() else {
            return;
        };
        let selected = view.selected.clone();
        let Some((index, parent)) = selected.split_last() else {
            self.message = Some(crate::tr("message.root_not_moved"));
            cx.notify();
            return;
        };
        let Some(siblings) = view.root.at(parent).map(|node| node.children.len()) else {
            return;
        };
        if inside_a_scrollbar_assembly(&view.root, &selected) {
            self.message = Some(crate::tr("message.scrollbar_is_one_piece"));
            cx.notify();
            return;
        }
        // The destination is read *before* the removal, which is the contract
        // `Node::move_node` states: going down means landing past the sibling
        // that is currently next, hence `index + 2`.
        let destination = match (up, index) {
            (true, 0) => {
                self.message = Some(crate::tr("message.node_at_top"));
                cx.notify();
                return;
            }
            (true, index) => index - 1,
            (false, index) if index + 1 >= siblings => {
                self.message = Some(crate::tr("message.node_at_bottom"));
                cx.notify();
                return;
            }
            (false, index) => index + 2,
        };
        let mut destination = {
            let mut path = parent.to_vec();
            path.push(destination);
            path
        };

        self.checkpoint();
        let view = self.view_mut().expect("just borrowed");
        match view.root.move_node(&selected, &destination) {
            Some(landed) => destination = landed,
            None => {
                // Nothing moved: drop the checkpoint we just took, or `⌘Z` would
                // step over an edit that never happened.
                view.past.pop();
                return;
            }
        }
        view.selected = destination;
        cx.notify();
    }

    /// Opens the handler of the selected node in the editor, on its own line.
    ///
    /// The node's own Action property, whichever one its component offers: the
    /// menu entry is about the node under the pointer, not about a property the
    /// inspector happens to be showing. A component with no handler at all, and
    /// a handler that has not been written yet, are told apart by
    /// [`Workspace::open_handler`], which says both out loud.
    pub fn open_selected_handler(&mut self, cx: &mut Context<Self>) {
        let Some(view) = self.view() else {
            return;
        };
        let Some(prop) = registry::of(view.selected())
            .and_then(|spec| spec.props.iter().find(|prop| matches!(prop.kind, Kind::Handler)))
        else {
            self.message = Some(crate::tr("message.no_action"));
            cx.notify();
            return;
        };
        self.open_handler(prop, cx);
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
        // And the box on the canvas goes, for the reason a moved selection
        // takes it: it would go on showing the word that has just been undone,
        // over a node the step may have taken away.
        self.canvas_edit = None;
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
        self.canvas_edit = None;
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

/// Where an insertion asked for from the palette's menu lands.
///
/// The three drop zones of the structure tree, named: the strip before a row,
/// the strip after it, and the row itself. Written as a type rather than as
/// three methods because it is one gesture with three answers, and the tree
/// already proves they are the only three.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Insert {
    /// Just before the selected node, among its siblings.
    Before,
    /// Just after it.
    After,
    /// Inside it, after what it already holds.
    Into,
}

/// Whether the node at `path` is a piece of a scrollbar assembly.
///
/// The visible scrollbar is not one node but three, in a shape gpui makes
/// compulsory: a `relative` wrapper holding the scrolling box and, **second**,
/// an absolutely positioned overlay carrying the bar. `is_scrollbar_wrapper`
/// recognises exactly that shape, and the inspector's switch is what writes it
/// and takes it apart.
///
/// So a command that moves or promotes one of the two children breaks the
/// shape from the outside: the switch stops recognising the assembly and reads
/// as *off* while the file still holds the bar, and turning it back on writes a
/// second one. Promoting the overlay is worse — the bar leaves its absolute
/// box, and the generated project paints it in the flow instead of over the
/// scrolling area. Both compile, which is why neither would have been noticed
/// here.
///
/// Refusing is the answer rather than repairing: the assembly is one switch's
/// to write, and a command that quietly rebuilt it would be a second author.
fn inside_a_scrollbar_assembly(root: &crate::model::Node, path: &[usize]) -> bool {
    let Some((_, parent)) = path.split_last() else {
        return false;
    };
    root.at(parent).is_some_and(registry::is_scrollbar_wrapper)
}

/// Gives a freshly instantiated `node` a state field no sibling is using.
///
/// Two inputs sharing `&self.field` compile and then mirror each other at
/// runtime, so each one needs its own. The same holds for every component
/// backed by an entity — a dropdown, a slider, a colour picker, a date
/// picker: they are not values but state the view owns.
///
/// Written once and called from both insertion paths on purpose. The palette
/// asked `spec.state.is_some()` while the drop asked `id == "input"`, so
/// dragging a dropdown in gave it the unnumbered `&self.field` that
/// `instantiate` leaves behind — two of them then shared one entity, which
/// compiles and only misbehaves when the project runs.
fn bind_own_state_field(id: &str, node: &mut crate::model::Node, root: &crate::model::Node) {
    if registry::by_id(id).is_none_or(|spec| spec.state.is_none()) {
        return;
    }
    let field = registry::unique_input_field(root);
    if let crate::model::Base::Known { args, .. } = &mut node.base {
        *args = vec![crate::model::Arg::Verbatim(format!("&self.{field}"))];
    }
}

#[cfg(test)]
mod tests {
    //! The commands above, driven the way the window drives them.
    //!
    //! These are unit tests rather than integration ones on purpose: what they
    //! hold is internal — `select_file`, `Workspace::new`, the private fields
    //! they read — and moving them to `tests/` would mean widening the crate's
    //! public API to reach them. A test is not a reason to promise a signature
    //! to anyone downstream.
    //!
    //! No window is opened. `TestAppContext` gives a real `App`, real entities
    //! and a real update cycle, which is everything these commands touch; what
    //! it does not give is pixels, so nothing here can say what a click lands
    //! on. That is the one part of the designer still checked by hand.

    use std::path::PathBuf;

    use gpui::{AppContext as _, TestAppContext};

    use crate::model::{Base, Node};
    use crate::project::Project;
    use crate::scaffold::{self, Template};
    use crate::workspace::{TextSurface, Workspace};

    /// A scratch directory of this test's own, removed when it is dropped.
    ///
    /// `MAXX_SCRATCH` is honoured for the same reason the rest of the suite
    /// honours it: a generated project shares one cargo target directory, and
    /// pointing them all at the same place is what keeps a scaffold test at
    /// seconds rather than minutes.
    struct Scratch(PathBuf);

    impl Scratch {
        fn new(name: &str) -> Self {
            let base = std::env::var_os("MAXX_SCRATCH")
                .map(PathBuf::from)
                .unwrap_or_else(std::env::temp_dir);
            let root = base.join(format!("maxx-edits-{name}-{}", std::process::id()));
            let _ = std::fs::remove_dir_all(&root);
            std::fs::create_dir_all(&root).expect("the scratch directory must be creatable");
            Self(root)
        }
    }

    impl Drop for Scratch {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    /// A workspace over a freshly created project, with its `home` view open.
    ///
    /// The empty shape, because what is being tested is the tree of one view
    /// and not the shell around it.
    fn workspace_on_a_view(scratch: &Scratch, cx: &mut TestAppContext) -> gpui::Entity<Workspace> {
        let root = scratch.0.join("trial");
        scaffold::create_project(&root, "trial", Template::Empty)
            .expect("the project must be created");
        let view_path = root.join("src/ui/home.rs");

        let workspace = cx.update(|cx| cx.new(|cx| Workspace::new(Some(Project::open(root)), cx)));
        workspace.update(cx, |workspace, cx| {
            workspace.select_file(view_path, cx);
            assert!(workspace.view().is_some(), "the view must open");
        });
        workspace
    }

    /// The path of every node of the tree, as its constructor name.
    fn shape(node: &Node) -> Vec<String> {
        let mut out = Vec::new();
        node.walk(&mut |path, node| {
            let name = match &node.base {
                Base::Known { path, .. } => path.clone(),
                Base::Opaque(source) => source.clone(),
            };
            out.push(format!("{path:?} {name}"));
        });
        out
    }

    /// Showing one thing in the middle stops showing every other.
    ///
    /// The property the `Center` enum exists to make true, checked from the
    /// gestures rather than from the type: three reviews found the same defect
    /// in three shapes — a tab click, an editor opened, a file deleted — each
    /// one a site that turned off some modes and not the one added last. What
    /// this asserts is not that the code clears the others, but that after each
    /// gesture exactly one thing is showing.
    #[gpui::test]
    fn the_middle_shows_one_thing_at_a_time(cx: &mut TestAppContext) {
        let scratch = Scratch::new("center");
        let root = scratch.0.join("trial");
        scaffold::create_project(&root, "trial", Template::Empty)
            .expect("the project must be created");
        scaffold::add_theme_module(&root).expect("the palette must be added");

        let workspace =
            cx.update(|cx| cx.new(|cx| Workspace::new(Some(Project::open(root.clone())), cx)));

        // How many modes claim the middle. Never more than one, and the
        // designer is the absence of them all. The reader is not counted: it
        // holds a document, which lives under whatever covers it.
        let showing = |workspace: &Workspace| {
            [
                workspace.menu_file().is_some(),
                workspace.palette().is_some(),
                workspace.preferences(),
                // Whether the reader is SHOWN, not whether it holds a file. The
                // two came apart and only the second was ever asserted: a middle
                // that showed the reader for as long as a file was open had no
                // way back to the view, and this test said nothing.
                workspace.showing_code(),
            ]
            .iter()
            .filter(|on| **on)
            .count()
        };

        workspace.update(cx, |workspace, cx| {
            workspace.select_file(root.join("src/ui/home.rs"), cx);
            assert_eq!(showing(workspace), 0, "a view is the designer, not a mode");

            workspace.select_file(root.join("src/theme.rs"), cx);
            assert!(workspace.palette().is_some(), "the palette opens");
            assert_eq!(showing(workspace), 1);

            workspace.select_file(root.join("src/main.rs"), cx);
            assert!(workspace.code().is_some(), "the reader holds the file");
            assert!(workspace.showing_code(), "and shows it");
            assert_eq!(showing(workspace), 1);

            // The reader is a document, so a mode covers it and does not
            // destroy it. Made a variant of `Center`, `⌘,` twice left the
            // designer with the file gone and its tab with it.
            workspace.toggle_preferences(cx);
            assert!(workspace.preferences(), "the settings cover it");
            assert!(workspace.code().is_some(), "the file is still there underneath");
            assert_eq!(showing(workspace), 1, "covered, not doubled");

            // The gesture that was broken: from the reader, the tab strip has to
            // be the way back to the view.
            workspace.activate_code(cx);
            assert!(workspace.showing_code(), "the read tab brings it forward");
            workspace.activate_view(0, cx);
            assert!(!workspace.showing_code(), "and the view tab comes back to the view");
            assert!(workspace.code().is_some(), "without throwing the file away");
            assert_eq!(showing(workspace), 0);

            workspace.select_file(root.join("src/theme.rs"), cx);
            assert!(workspace.palette().is_some());
            workspace.activate_view(0, cx);
            assert_eq!(showing(workspace), 0, "a tab click comes back to the view");
        });
    }

    /// A project component's own source opens in the reader.
    ///
    /// The palette shows the brick's name; the file it is written in is one
    /// click from there. The reader and not an editor: maxx reads, Zed writes.
    #[gpui::test]
    fn a_brick_s_source_opens_in_the_reader(cx: &mut TestAppContext) {
        let scratch = Scratch::new("brick-source");
        let root = scratch.0.join("trial");
        scaffold::create_project(&root, "trial", Template::Empty)
            .expect("the project must be created");
        scaffold::add_components_module(&root).expect("the components must be added");

        let workspace =
            cx.update(|cx| cx.new(|cx| Workspace::new(Some(Project::open(root.clone())), cx)));

        workspace.update(cx, |workspace, cx| {
            assert!(
                workspace.bricks.iter().any(|brick| brick.module == "card"),
                "the library maxx just wrote must be readable"
            );
            workspace.open_brick_source("card", cx);

            let file = workspace.code().expect("the reader holds the file");
            assert!(workspace.showing_code(), "and shows it");
            // The tail, not the whole path: `Project::open` canonicalises, and
            // on macOS the scratch directory is a symlink — `/var/folders/…`
            // resolving to `/private/var/folders/…`.
            assert!(file.path.ends_with("src/components/card.rs"), "{}", file.path.display());
        });
    }

    /// Unwrapping a box keeps the sentences written on its calls.
    ///
    /// The calls do not follow — a `gap` on the box says nothing about what was
    /// inside it — but the lines written above each call are the developer's
    /// words, and the model carries them precisely so that a save cannot erase
    /// them. Dropping the calls wholesale took the words with them, silently,
    /// on the one gesture whose whole purpose is to keep the node.
    #[gpui::test]
    fn unwrapping_keeps_what_was_written_on_the_box(cx: &mut TestAppContext) {
        let scratch = Scratch::new("unwrap-comments");
        let workspace = workspace_on_a_view(&scratch, cx);

        workspace.update(cx, |workspace, cx| {
            workspace.insert_component("label", cx);
            let inner = workspace.view().expect("open").selected.clone();
            workspace.wrap_selected("column", cx);

            // The wrapper took the node's place, so it sits where the node
            // was; the node is now its only child.
            let box_path = inner.clone();
            {
                let view = workspace.view_mut().expect("open");
                let node = view.root.at_mut(&box_path).expect("the wrapper");
                node.comments = vec!["// two things, side by side".into()];
                node.set_call("gap_2", crate::model::Arg::Verbatim(String::new()));
                node.calls
                    .iter_mut()
                    .find(|call| call.name == "gap_2")
                    .expect("just set")
                    .comments = vec!["// a hair of air between them".into()];
            }

            workspace.select(box_path.clone(), cx);
            workspace.unwrap_selected(cx);

            let promoted = workspace.view().expect("open").root.at(&box_path).expect("promoted");
            let kept = promoted.comments.join("\n");
            assert!(
                kept.contains("two things, side by side"),
                "what was written above the box survives: {kept:?}"
            );
            assert!(
                kept.contains("a hair of air between them"),
                "and what was written on its calls: {kept:?}"
            );
        });
    }

    /// Typing on the canvas leaves the inspector to catch up.
    ///
    /// Closing a session adopts the sync key, which says "the panel already
    /// shows what the tree holds" — true of the inspector's own boxes, and of
    /// no other. The canvas box shares the session mechanism but is a different
    /// entity, so adopting the key after a double-click edit left the panel
    /// showing the old label; one more character typed there wrote it straight
    /// back over what the canvas had just said. The surface is carried by the
    /// session so that the answer is declared rather than guessed.
    #[gpui::test]
    fn a_canvas_edit_leaves_the_inspector_to_rebuild(cx: &mut TestAppContext) {
        let scratch = Scratch::new("canvas-sync");
        let workspace = workspace_on_a_view(&scratch, cx);

        workspace.update(cx, |workspace, cx| {
            workspace.insert_component("label", cx);
            let Some(prop) =
                crate::registry::spoken_text(workspace.view().expect("open").selected())
            else {
                panic!("a label says something out loud");
            };

            for (surface, expected) in
                [(TextSurface::Inspector, true), (TextSurface::Canvas, false)]
            {
                // A stand-in entity plays the box: an `InputState` needs a
                // window, and what tells one session from another is identity.
                let field = cx.new(|_| ()).entity_id();
                workspace.begin_text_edit(field, surface, cx);
                workspace.edit_prop_text(prop, &format!("typed from {surface:?}"), cx);
                workspace.close_text_edit_of(field, cx);

                let key = workspace.view().map(|view| (workspace.revision, view.selected.clone()));
                assert_eq!(
                    workspace.synced == key,
                    expected,
                    "{surface:?}: the panel is up to date only when it did the writing"
                );
            }
        });
    }

    /// Wrapping a piece of the scrollbar assembly is refused like moving one.
    ///
    /// Wrapping *replaces* the child, so the wrapper's second child stops being
    /// the absolute overlay and the shape stops matching — the same outcome as
    /// a move, reached by the third command. The guard was written for two and
    /// this is the one it had missed.
    #[gpui::test]
    fn wrapping_a_piece_of_the_scrollbar_is_refused_too(cx: &mut TestAppContext) {
        let scratch = Scratch::new("scrollbar-wrap");
        let workspace = workspace_on_a_view(&scratch, cx);

        workspace.update(cx, |workspace, cx| {
            workspace.insert_component("column", cx);
            let column = workspace.view().expect("open").selected.clone();
            workspace.set_scrollbar(true, true, cx);

            let overlay = {
                let mut path = column.clone();
                path.push(1);
                path
            };
            let before = workspace.view().expect("open").root.clone();
            workspace.select(overlay, cx);
            workspace.wrap_selected("column", cx);

            let after = &workspace.view().expect("open").root;
            assert_eq!(&before, after, "wrapping left the assembly alone");
            assert!(
                crate::registry::is_scrollbar_wrapper(after.at(&column).expect("still there")),
                "and the switch still recognises it"
            );
        });
    }

    /// The scrollbar assembly is one switch's to write, and to take apart.
    ///
    /// Its shape is not a taste: the bar has to be a *sibling* of the scrolling
    /// box under a `relative` wrapper, second of two children. Moving either
    /// child, or promoting one out, leaves a shape the inspector no longer
    /// recognises — the switch then reads *off* while the file still holds the
    /// bar, and turning it on writes a second one. Both halves compile, so
    /// nothing else in the chain would catch it.
    #[gpui::test]
    fn the_scrollbar_assembly_refuses_to_be_taken_apart(cx: &mut TestAppContext) {
        let scratch = Scratch::new("scrollbar-guard");
        let workspace = workspace_on_a_view(&scratch, cx);

        workspace.update(cx, |workspace, cx| {
            workspace.insert_component("column", cx);
            let column = workspace.view().expect("open").selected.clone();
            workspace.set_scrollbar(true, true, cx);

            // The wrapper took the column's place; its second child is the bar.
            let wrapper = workspace.view().expect("open").root.at(&column).expect("the wrapper");
            assert!(
                crate::registry::is_scrollbar_wrapper(wrapper),
                "the switch wrote the assembly"
            );
            let overlay = {
                let mut path = column.clone();
                path.push(1);
                path
            };

            let before = workspace.view().expect("open").root.clone();
            workspace.select(overlay.clone(), cx);
            workspace.move_selected_node(true, cx);
            workspace.unwrap_selected(cx);

            let after = &workspace.view().expect("open").root;
            assert_eq!(&before, after, "neither command touched the assembly");
            assert!(
                crate::registry::is_scrollbar_wrapper(after.at(&column).expect("still there")),
                "and the switch still recognises it"
            );
        });
    }

    /// Two stateful components dropped in are two entities, not one shared.
    ///
    /// Dragging is the path this went wrong on: the palette's own command asked
    /// the catalogue whether the entry has state, the drop asked whether its
    /// name was `input`. So every other stateful entry — a dropdown, a slider,
    /// a colour picker — came in carrying the unnumbered `&self.field` that
    /// `instantiate` leaves behind, and a second one bound the same entity. It
    /// compiles; the two boxes simply echo each other once the project runs,
    /// which is a long way from here.
    ///
    /// Written on the dropdown rather than the text input precisely because the
    /// input was the one case the old test would have passed.
    #[gpui::test]
    fn two_dropped_dropdowns_do_not_share_one_entity(cx: &mut TestAppContext) {
        let scratch = Scratch::new("drop-state");
        let workspace = workspace_on_a_view(&scratch, cx);

        workspace.update(cx, |workspace, cx| {
            workspace.drop_at(&[], 0, crate::designer::Dragged::Component("select"), cx);
            workspace.drop_at(&[], 1, crate::designer::Dragged::Component("select"), cx);

            let root = &workspace.view().expect("open").root;
            let bindings: Vec<_> = root
                .children
                .iter()
                .filter_map(|child| match &child.base {
                    crate::model::Base::Known { args, .. } => args.first(),
                    _ => None,
                })
                .filter_map(|arg| match arg {
                    crate::model::Arg::Verbatim(text) if text.starts_with("&self.") => {
                        Some(text.clone())
                    }
                    _ => None,
                })
                .collect();

            assert_eq!(bindings.len(), 2, "both dropdowns are bound: {bindings:?}");
            assert_ne!(bindings[0], bindings[1], "each dropdown owns its own field: {bindings:?}");
        });
    }

    /// A component dropped at an index lands at that index, not at the end.
    ///
    /// The tree is the second way into the canvas, and the whole point of the
    /// drop zones is that they place a node *between* two others. Appending
    /// would look like it worked everywhere but the one place it matters.
    #[gpui::test]
    fn a_component_dropped_between_two_children_lands_between_them(cx: &mut TestAppContext) {
        let scratch = Scratch::new("drop-between");
        let workspace = workspace_on_a_view(&scratch, cx);

        workspace.update(cx, |workspace, cx| {
            // The generated view is not empty — it opens on a Label — so the
            // count is read rather than assumed.
            let before = workspace.view().expect("open").root.children.len();

            workspace.drop_at(&[], 0, crate::designer::Dragged::Component("label"), cx);
            workspace.drop_at(&[], 1, crate::designer::Dragged::Component("button"), cx);
            // Between the two, and this is the whole feature in one assertion:
            // a second drop at index 1 must push the button along, not follow it.
            workspace.drop_at(&[], 1, crate::designer::Dragged::Component("button"), cx);

            let root = &workspace.view().expect("open").root;
            let names: Vec<_> = root
                .children
                .iter()
                .map(|child| child.base.path().unwrap_or("opaque").to_string())
                .collect();
            assert_eq!(names.len(), before + 3, "three more children: {names:?}");
            assert_eq!(names[0], "Label::new", "the first drop is still first: {names:?}");
            assert_eq!(names[1], "Button::new", "the last drop took index 1: {names:?}");
            assert_eq!(names[2], "Button::new", "and pushed the other one along: {names:?}");
        });
    }

    /// Undo puts back exactly the tree that was there.
    ///
    /// A checkpoint taken after the edit rather than before it looks right on
    /// the first `⌘Z` and loses one step on the second; comparing the whole
    /// shape rather than a count is what makes that visible.
    #[gpui::test]
    fn undo_and_redo_walk_the_same_trees(cx: &mut TestAppContext) {
        let scratch = Scratch::new("undo");
        let workspace = workspace_on_a_view(&scratch, cx);

        workspace.update(cx, |workspace, cx| {
            let empty = shape(&workspace.view().expect("open").root);

            workspace.drop_at(&[], 0, crate::designer::Dragged::Component("label"), cx);
            let one = shape(&workspace.view().expect("open").root);
            workspace.drop_at(&[], 1, crate::designer::Dragged::Component("button"), cx);
            let two = shape(&workspace.view().expect("open").root);
            assert_ne!(one, empty);
            assert_ne!(two, one);

            workspace.undo(cx);
            assert_eq!(shape(&workspace.view().expect("open").root), one, "one step back");
            workspace.undo(cx);
            assert_eq!(shape(&workspace.view().expect("open").root), empty, "two steps back");
            workspace.redo(cx);
            assert_eq!(shape(&workspace.view().expect("open").root), one, "and forward again");
            workspace.redo(cx);
            assert_eq!(shape(&workspace.view().expect("open").root), two);
        });
    }

    /// A duplicated input gets a state field of its own.
    ///
    /// The copy would otherwise share the original's `&self.field`: two boxes
    /// on the screen, one `Entity` behind them, and typing in either showing in
    /// both. It compiles, so only this says it.
    #[gpui::test]
    fn a_duplicated_input_does_not_share_the_original_s_state(cx: &mut TestAppContext) {
        let scratch = Scratch::new("duplicate");
        let workspace = workspace_on_a_view(&scratch, cx);

        workspace.update(cx, |workspace, cx| {
            workspace.drop_at(&[], 0, crate::designer::Dragged::Component("input"), cx);
            workspace.select(vec![0], cx);
            workspace.duplicate_selected(cx);

            let root = &workspace.view().expect("open").root;
            let inputs: Vec<_> = root
                .children
                .iter()
                .filter(|child| child.base.path() == Some("Input::new"))
                .collect();
            assert_eq!(inputs.len(), 2, "the copy sits beside the original");

            let field = |node: &Node| match &node.base {
                Base::Known { args, .. } => args.first().map(|arg| arg.to_source()),
                Base::Opaque(_) => None,
            };
            let original = field(inputs[0]).expect("the original binds a field");
            let copy = field(inputs[1]).expect("the copy binds a field");
            assert_ne!(original, copy, "two boxes cannot share one entity: {original} / {copy}");
        });
    }

    /// Wrapping and unwrapping are inverses.
    ///
    /// The tree has to come back identical, not merely similar: the wrapper is
    /// removed from `children` and from the child slot it left among `calls`,
    /// and forgetting the second leaves a node the code generator writes as an
    /// empty `.child()`.
    #[gpui::test]
    fn wrapping_then_unwrapping_gives_the_tree_back(cx: &mut TestAppContext) {
        let scratch = Scratch::new("wrap-roundtrip");
        let workspace = workspace_on_a_view(&scratch, cx);

        workspace.update(cx, |workspace, cx| {
            workspace.drop_at(&[], 0, crate::designer::Dragged::Component("button"), cx);
            workspace.select(vec![0], cx);
            let before = workspace.view().expect("open").root.clone();

            workspace.wrap_selected("column", cx);
            let root = &workspace.view().expect("open").root;
            assert_eq!(root.children[0].base.path(), Some("v_flex"), "a column took the place");
            assert_eq!(root.children[0].children.len(), 1, "with the node as its only child");
            assert_eq!(root.children[0].children[0].base.path(), Some("Button::new"));
            assert_eq!(
                workspace.view().expect("open").selected,
                vec![0, 0],
                "the selection follows the node, not the box"
            );

            // The box, not the node: the selection was left on the node, and
            // unwrapping is a gesture on the container.
            workspace.select(vec![0], cx);
            workspace.unwrap_selected(cx);
            assert_eq!(
                workspace.view().expect("open").root,
                before,
                "the tree is the one we started from, calls and child slots included"
            );
            assert_eq!(
                workspace.view().expect("open").selected,
                vec![0],
                "and the promoted child is what stays selected"
            );
        });
    }

    /// The wrapped node keeps its own calls.
    ///
    /// The question the backlog left open, answered here rather than in prose:
    /// a `bg` written on a button is a fact about the button. Lifting the calls
    /// onto the wrapper would repaint the box and leave the button bare, and it
    /// would be undone by hand every single time.
    #[gpui::test]
    fn wrapping_leaves_the_node_s_style_on_the_node(cx: &mut TestAppContext) {
        let scratch = Scratch::new("wrap-style");
        let workspace = workspace_on_a_view(&scratch, cx);

        workspace.update(cx, |workspace, cx| {
            workspace.drop_at(&[], 0, crate::designer::Dragged::Component("button"), cx);
            workspace.select(vec![0], cx);
            workspace
                .view_mut()
                .expect("open")
                .root
                .at_mut(&[0])
                .expect("the button")
                .set_call("label", crate::model::Arg::Str("Go".into()));

            workspace.wrap_selected("row", cx);

            let root = &workspace.view().expect("open").root;
            let wrapper = &root.children[0];
            assert!(wrapper.call("label").is_none(), "the box gains nothing");
            let inner = &wrapper.children[0];
            assert_eq!(
                inner.call("label").map(|call| call.args[0].to_source()),
                Some("\"Go\"".to_string()),
                "and the node keeps what was written on it"
            );
        });
    }

    /// Unwrapping refuses a container that is not holding exactly one node.
    ///
    /// Promoting one of two children means deleting the other, and picking
    /// which one is not something maxx gets to decide on its own. The refusal
    /// is said out loud, the way `DeleteFile` says its own.
    #[gpui::test]
    fn unwrapping_refuses_a_container_with_two_children(cx: &mut TestAppContext) {
        let scratch = Scratch::new("unwrap-refuses");
        let workspace = workspace_on_a_view(&scratch, cx);

        workspace.update(cx, |workspace, cx| {
            workspace.drop_at(&[], 0, crate::designer::Dragged::Component("column"), cx);
            workspace.drop_at(&[0], 0, crate::designer::Dragged::Component("label"), cx);
            workspace.drop_at(&[0], 1, crate::designer::Dragged::Component("button"), cx);
            workspace.select(vec![0], cx);

            let before = shape(&workspace.view().expect("open").root);
            let steps = workspace.view().expect("open").past.len();
            workspace.unwrap_selected(cx);
            assert_eq!(shape(&workspace.view().expect("open").root), before, "nothing moved");
            assert_eq!(workspace.view().expect("open").past.len(), steps, "and nothing to undo");
            assert!(workspace.message.is_some(), "the refusal is said");

            // The empty container is refused for the same reason, from the
            // other side: there is nothing to promote.
            workspace.select(vec![0, 0], cx);
            workspace.delete_selected(cx);
            workspace.select(vec![0, 0], cx);
            workspace.delete_selected(cx);
            workspace.select(vec![0], cx);
            workspace.message = None;
            workspace.unwrap_selected(cx);
            assert!(workspace.message.is_some(), "an empty box is refused too");

            // And so is the root, which has no parent to be promoted into.
            workspace.select(vec![], cx);
            workspace.message = None;
            workspace.unwrap_selected(cx);
            assert!(workspace.message.is_some(), "the root is not unwrapped");
        });
    }

    /// Wrapping is one undo step, not two.
    ///
    /// It is built out of a removal and an insertion, which is exactly the
    /// shape that ends up costing two `⌘Z`: the first would put the node back
    /// beside an empty column, which is a state the developer never saw.
    #[gpui::test]
    fn wrapping_is_a_single_undo_step(cx: &mut TestAppContext) {
        let scratch = Scratch::new("wrap-undo");
        let workspace = workspace_on_a_view(&scratch, cx);

        workspace.update(cx, |workspace, cx| {
            workspace.drop_at(&[], 0, crate::designer::Dragged::Component("button"), cx);
            workspace.select(vec![0], cx);
            let before = shape(&workspace.view().expect("open").root);
            let steps = workspace.view().expect("open").past.len();

            workspace.wrap_selected("column", cx);
            assert_eq!(
                workspace.view().expect("open").past.len(),
                steps + 1,
                "the lift and the drop are one gesture"
            );

            workspace.undo(cx);
            assert_eq!(
                shape(&workspace.view().expect("open").root),
                before,
                "and one ⌘Z takes the whole thing back"
            );
        });
    }

    /// The wrapper is written as an ordinary `v_flex`, with the node inside it.
    ///
    /// The tree is only half the promise: what the developer opens in Zed is
    /// the file, so the assertion is on the text the save would write, import
    /// line included.
    #[gpui::test]
    fn a_wrapped_node_is_written_as_a_v_flex_holding_it(cx: &mut TestAppContext) {
        let scratch = Scratch::new("wrap-source");
        let workspace = workspace_on_a_view(&scratch, cx);

        workspace.update(cx, |workspace, cx| {
            workspace.drop_at(&[], 0, crate::designer::Dragged::Component("button"), cx);
            workspace.select(vec![0], cx);
            workspace.wrap_selected("column", cx);

            let source = workspace.view().expect("open").render_source().expect("it renders");
            assert!(
                source.contains("use gpui_component::v_flex;"),
                "the column owes its import: {source}"
            );
            assert!(
                source
                    .contains(".child(v_flex().child(Button::new(\"button\").label(\"Button\")))"),
                "the button is written inside the column: {source}"
            );
        });
    }

    /// The form field template lands with a field of its own, every time.
    ///
    /// The reason templates were stateless: a second copy of `&self.field`
    /// compiles and then mirrors the first, so the two inputs of one form would
    /// type into each other. Dropped twice, the tree has to hold two bindings —
    /// and the file the save writes has to declare both, which is the half this
    /// lot reuses rather than writes: `render_source` already walks the tree for
    /// them.
    #[gpui::test]
    fn the_form_field_template_brings_a_state_field_of_its_own(cx: &mut TestAppContext) {
        let scratch = Scratch::new("template-state");
        let workspace = workspace_on_a_view(&scratch, cx);

        workspace.update(cx, |workspace, cx| {
            workspace.insert_subtree("form_field", cx);
            // Back to the root: the first drop leaves its own `v_flex` selected,
            // and what is being counted is two fields, not where they sit.
            workspace.select(vec![], cx);
            workspace.insert_subtree("form_field", cx);

            let view = workspace.view().expect("open");
            let mut bound = Vec::new();
            view.root.walk(&mut |_, node| {
                if node.base.path() == Some("Input::new")
                    && let Base::Known { args, .. } = &node.base
                    && let Some(arg) = args.first()
                {
                    bound.push(arg.to_source());
                }
            });
            assert_eq!(
                bound,
                ["&self.field", "&self.field_2"],
                "two inputs sharing one field would mirror each other"
            );

            let source = view.render_source().expect("it renders");
            for field in ["field", "field_2"] {
                assert!(
                    source.contains(&format!("pub {field}: Entity<InputState>")),
                    "{field} must be declared: {source}"
                );
                assert!(
                    source.contains(&format!("{field}: cx.new(|cx| InputState::new(window, cx))")),
                    "{field} must be built: {source}"
                );
            }
        });
    }

    /// Moving a node steps it through its siblings and stops at both ends.
    ///
    /// The two ends are the point: a move that wraps around looks exactly like a
    /// deletion followed by an insertion somewhere else, and `⌥↓` held down
    /// would shuffle the list rather than reach its bottom.
    #[gpui::test]
    fn a_node_moves_among_its_siblings_and_stops_at_the_ends(cx: &mut TestAppContext) {
        let scratch = Scratch::new("move-node");
        let workspace = workspace_on_a_view(&scratch, cx);

        workspace.update(cx, |workspace, cx| {
            // A container of its own, so the generated view's own children do
            // not decide what "first" and "last" mean here.
            workspace.drop_at(&[], 0, crate::designer::Dragged::Component("column"), cx);
            workspace.drop_at(&[0], 0, crate::designer::Dragged::Component("label"), cx);
            workspace.drop_at(&[0], 1, crate::designer::Dragged::Component("button"), cx);
            workspace.drop_at(&[0], 2, crate::designer::Dragged::Component("divider"), cx);

            let names = |workspace: &Workspace| -> Vec<String> {
                workspace.view().expect("open").root.children[0]
                    .children
                    .iter()
                    .map(|child| child.base.path().unwrap_or("opaque").to_string())
                    .collect()
            };
            assert_eq!(names(workspace), ["Label::new", "Button::new", "Divider::horizontal"]);

            // The middle one, up: it swaps with the label and the selection
            // goes with it.
            workspace.select(vec![0, 1], cx);
            workspace.move_selected_node(true, cx);
            assert_eq!(names(workspace), ["Button::new", "Label::new", "Divider::horizontal"]);
            assert_eq!(
                workspace.view().expect("open").selected,
                vec![0, 0],
                "the selection follows the node, or the gesture is unrepeatable"
            );

            // And down twice, which walks it to the far end.
            workspace.move_selected_node(false, cx);
            workspace.move_selected_node(false, cx);
            assert_eq!(names(workspace), ["Label::new", "Divider::horizontal", "Button::new"]);
            assert_eq!(workspace.view().expect("open").selected, vec![0, 2]);

            // The bottom refuses, out loud, and costs no undo step.
            let steps = workspace.view().expect("open").past.len();
            workspace.message = None;
            workspace.move_selected_node(false, cx);
            assert_eq!(names(workspace), ["Label::new", "Divider::horizontal", "Button::new"]);
            assert_eq!(workspace.view().expect("open").past.len(), steps, "nothing to undo");
            assert!(workspace.message.is_some(), "the refusal is said");

            // So does the top, from the other side.
            workspace.select(vec![0, 0], cx);
            workspace.message = None;
            workspace.move_selected_node(true, cx);
            assert_eq!(names(workspace), ["Label::new", "Divider::horizontal", "Button::new"]);
            assert_eq!(workspace.view().expect("open").past.len(), steps);
            assert!(workspace.message.is_some());

            // And so does the root, which has no siblings at all.
            workspace.select(vec![], cx);
            workspace.message = None;
            workspace.move_selected_node(true, cx);
            assert!(workspace.message.is_some(), "the root is not moved");
            assert_eq!(workspace.view().expect("open").past.len(), steps);
        });
    }

    /// One move is one undo step, and `⌘Z` puts the order back.
    ///
    /// It is built out of a removal and an insertion — the shape that ends up
    /// costing two `⌘Z`, with a tree in between that nobody ever saw.
    #[gpui::test]
    fn moving_a_node_is_a_single_undo_step(cx: &mut TestAppContext) {
        let scratch = Scratch::new("move-undo");
        let workspace = workspace_on_a_view(&scratch, cx);

        workspace.update(cx, |workspace, cx| {
            workspace.drop_at(&[], 0, crate::designer::Dragged::Component("label"), cx);
            workspace.drop_at(&[], 1, crate::designer::Dragged::Component("button"), cx);
            workspace.select(vec![1], cx);
            let before = shape(&workspace.view().expect("open").root);
            let steps = workspace.view().expect("open").past.len();

            workspace.move_selected_node(true, cx);
            assert_ne!(shape(&workspace.view().expect("open").root), before, "it did move");
            assert_eq!(
                workspace.view().expect("open").past.len(),
                steps + 1,
                "the lift and the drop are one gesture"
            );

            workspace.undo(cx);
            assert_eq!(
                shape(&workspace.view().expect("open").root),
                before,
                "and one ⌘Z takes the whole thing back"
            );
        });
    }

    /// The text prop a component of the catalogue offers first, and the value
    /// standing in it.
    ///
    /// Read through the registry rather than off the node, because that is what
    /// the inspector's field does: a test reaching into the tree by hand would
    /// pass on a chain the inspector cannot actually edit.
    fn text_prop(node: &Node) -> &'static crate::registry::Prop {
        let spec = crate::registry::of(node).expect("the component is the catalogue's");
        crate::registry::props(spec)
            .into_iter()
            .find(|prop| matches!(prop.kind, crate::registry::Kind::Text))
            .expect("a label has text to type in")
    }

    /// Typing a word in the inspector is one undo step, not one per keystroke.
    ///
    /// The grain is the whole point, and it is held by two halves that have to
    /// agree: `InputEvent::Focus` takes the snapshot, `close_text_edit` turns it
    /// into a step on blur. Neither can be reached without a window, so what is
    /// driven here is what the subscription does — take the snapshot, one
    /// `edit_prop_text` per keystroke, then close.
    ///
    /// A checkpoint per keystroke is the shape this rules out: it floods the
    /// stack, and — through the revision counter that rebuilds the fields —
    /// destroys the box under the caret.
    #[gpui::test]
    fn typing_in_the_inspector_is_one_undo_step(cx: &mut TestAppContext) {
        let scratch = Scratch::new("text-undo");
        let workspace = workspace_on_a_view(&scratch, cx);

        workspace.update(cx, |workspace, cx| {
            workspace.drop_at(&[], 0, crate::designer::Dragged::Component("label"), cx);
            workspace.select(vec![0], cx);
            let prop = text_prop(workspace.view().expect("open").selected());
            let value = |workspace: &Workspace| {
                crate::registry::read(workspace.view().expect("open").selected(), prop)
            };

            let before = value(workspace);
            let steps = workspace.view().expect("open").past.len();
            let revision = workspace.revision;

            // The field takes the focus. An `InputState` cannot be built
            // without a window, and what tells one field from another is its
            // identity, so a stand-in entity plays the part.
            let field = cx.new(|_| ()).entity_id();
            workspace.begin_text_edit(field, TextSurface::Inspector, cx);
            for typed in ["H", "He", "Hel", "Hell", "Hello"] {
                workspace.edit_prop_text(prop, typed, cx);
            }
            assert_eq!(value(workspace).as_deref(), Some("Hello"));
            assert_eq!(
                workspace.view().expect("open").past.len(),
                steps,
                "typing pushes nothing on its own, or every keystroke would be a step"
            );
            // The other half of the same promise, and the one that cannot be
            // seen from the undo stack: `sync_prop_inputs` is keyed on
            // `(revision, selection)`, so a bump here rebuilds `prop_inputs`
            // between two keystrokes and the box under the caret goes with it.
            assert_eq!(
                workspace.revision, revision,
                "typing must not bump the revision, or the field is rebuilt under the caret"
            );

            // And leaves it.
            workspace.close_text_edit_of(field, cx);
            assert_eq!(
                workspace.view().expect("open").past.len(),
                steps + 1,
                "one visit to the field is one step"
            );
            assert_eq!(
                workspace.revision,
                revision + 1,
                "and leaving it moves the revision on, so the code panel catches up"
            );
            // The other side of that bump, and the caret's whole protection: the
            // key is adopted here, so `sync_prop_inputs` has nothing to answer
            // and no box is rebuilt under a caret that may have just landed in
            // the next field.
            assert_eq!(
                workspace.synced,
                Some((workspace.revision, workspace.view().expect("open").selected.clone())),
                "leaving a field rebuilds nothing: the fields already hold the tree"
            );

            workspace.undo(cx);
            assert_eq!(value(workspace), before, "and one ⌘Z takes the whole word back");
            assert!(
                !workspace.view().expect("open").future.is_empty(),
                "which leaves somewhere to redo to"
            );

            // A second visit ends that redo path, like any other edit.
            workspace.begin_text_edit(field, TextSurface::Inspector, cx);
            workspace.edit_prop_text(prop, "Other", cx);
            workspace.close_text_edit_of(field, cx);
            assert!(
                workspace.view().expect("open").future.is_empty(),
                "a new edit ends the redo path"
            );
        });
    }

    /// Entering a field and leaving it without typing is not a step.
    ///
    /// Otherwise clicking through the inspector fills the stack with steps that
    /// undo nothing, and `⌘Z` stops meaning anything: the first few presses
    /// would appear to do nothing at all.
    #[gpui::test]
    fn a_visit_that_changed_nothing_is_not_a_step(cx: &mut TestAppContext) {
        let scratch = Scratch::new("text-untouched");
        let workspace = workspace_on_a_view(&scratch, cx);

        workspace.update(cx, |workspace, cx| {
            workspace.drop_at(&[], 0, crate::designer::Dragged::Component("label"), cx);
            workspace.select(vec![0], cx);
            let steps = workspace.view().expect("open").past.len();

            let field = cx.new(|_| ()).entity_id();
            workspace.begin_text_edit(field, TextSurface::Inspector, cx);
            workspace.close_text_edit_of(field, cx);

            assert_eq!(workspace.view().expect("open").past.len(), steps);
            assert!(workspace.edit_snapshot.is_none(), "and the snapshot is spent either way");
        });
    }

    /// A snapshot belonging to another file is dropped, never applied here.
    ///
    /// The tab can change while a field holds the focus — `⌘⌥→`, a click in the
    /// tree, a file opened from the palette. Pushing the pending snapshot onto
    /// whichever view is in front would put one file's tree into another one's
    /// undo stack, and the next `⌘Z` would replace the view wholesale.
    #[gpui::test]
    fn a_snapshot_left_by_another_file_is_dropped(cx: &mut TestAppContext) {
        let scratch = Scratch::new("text-other-file");
        let workspace = workspace_on_a_view(&scratch, cx);

        workspace.update(cx, |workspace, cx| {
            let stranger = workspace.view().expect("open").root.clone();
            let field = cx.new(|_| ()).entity_id();
            workspace.edit_snapshot =
                Some((field, TextSurface::Inspector, PathBuf::from("somewhere/else.rs"), stranger));

            // Something moves in the view that is actually in front, so the two
            // trees genuinely differ and only the path can rule the snapshot out.
            workspace.drop_at(&[], 0, crate::designer::Dragged::Component("label"), cx);
            let steps = workspace.view().expect("open").past.len();
            let shape_now = shape(&workspace.view().expect("open").root);

            workspace.close_text_edit(cx);

            assert_eq!(
                workspace.view().expect("open").past.len(),
                steps,
                "another file's tree is no step of this one"
            );
            assert_eq!(shape(&workspace.view().expect("open").root), shape_now);
        });
    }

    /// Two fields visited in turn are two steps, whichever half speaks first.
    ///
    /// gpui hands one focus event to every listener in subscription order, so
    /// which of the two halves arrives first — the `Blur` of the field being
    /// left, the `Focus` of the one being entered — follows only the order the
    /// fields were built in: down the panel the blur comes first, up the panel
    /// the focus does. Naming the field that owns the session is what makes
    /// both orders end the same way. Without it, the edit made on the way up
    /// was closed by the wrong half and left no step at all: the tree kept the
    /// word, and `⌘Z` could never reach it.
    #[gpui::test]
    fn two_fields_are_two_steps_in_either_order(cx: &mut TestAppContext) {
        let scratch = Scratch::new("text-two-fields");
        let workspace = workspace_on_a_view(&scratch, cx);

        workspace.update(cx, |workspace, cx| {
            workspace.drop_at(&[], 0, crate::designer::Dragged::Component("label"), cx);
            workspace.select(vec![0], cx);
            let node = workspace.view().expect("open").selected();
            let text = text_prop(node);
            let width = crate::registry::props(crate::registry::of(node).expect("a label"))
                .into_iter()
                .find(|prop| matches!(prop.kind, crate::registry::Kind::Number))
                .expect("a label has a width to type in");
            let read = |workspace: &Workspace, prop: &'static crate::registry::Prop| {
                crate::registry::read(workspace.view().expect("open").selected(), prop)
                    .unwrap_or_default()
            };
            // Stand-ins for the two `InputState`s: building one takes a window,
            // and all the session knows of a field is its identity.
            let first = cx.new(|_| ()).entity_id();
            let second = cx.new(|_| ()).entity_id();
            let before = read(workspace, text);
            let steps = workspace.view().expect("open").past.len();

            // Down the panel: the field being left speaks first.
            workspace.begin_text_edit(first, TextSurface::Inspector, cx);
            workspace.edit_prop_text(text, "Hello", cx);
            workspace.close_text_edit_of(first, cx);
            workspace.begin_text_edit(second, TextSurface::Inspector, cx);
            workspace.edit_prop_text(width, "120", cx);
            workspace.close_text_edit_of(second, cx);

            assert_eq!(
                workspace.view().expect("open").past.len(),
                steps + 2,
                "two fields visited is two steps"
            );
            workspace.undo(cx);
            assert_eq!(read(workspace, width), "", "the first ⌘Z takes back the second field");
            assert_eq!(read(workspace, text), "Hello", "and leaves the first one standing");
            workspace.undo(cx);
            assert_eq!(read(workspace, text), before, "the second takes back the first");

            // Up the panel: the field being entered speaks first, and its
            // arrival is what closes the session before it. The blur that
            // follows belongs to a field that no longer owns anything.
            let steps = workspace.view().expect("open").past.len();
            let third = cx.new(|_| ()).entity_id();
            let fourth = cx.new(|_| ()).entity_id();
            workspace.begin_text_edit(third, TextSurface::Inspector, cx);
            workspace.edit_prop_text(width, "200", cx);
            workspace.begin_text_edit(fourth, TextSurface::Inspector, cx);
            workspace.close_text_edit_of(third, cx);
            workspace.edit_prop_text(text, "Typed", cx);
            workspace.close_text_edit_of(fourth, cx);

            assert_eq!(
                workspace.view().expect("open").past.len(),
                steps + 2,
                "the order the two halves arrive in changes nothing"
            );
            workspace.undo(cx);
            assert_eq!(read(workspace, text), before, "the word typed last comes back first");
            assert_eq!(read(workspace, width), "200", "the field before it is untouched");
            workspace.undo(cx);
            assert_eq!(read(workspace, width), "", "and its own step is there to be undone");
        });
    }

    /// A command met mid-word leaves the two steps in the order they happened.
    ///
    /// A command reaches the workspace with the field still holding the caret —
    /// `⌘D` travels through a focused input, a click in a menu blurs nothing —
    /// and the rebuild it triggers takes the field away before any `Blur` can
    /// be dispatched. So `checkpoint` is where the typing's step has to be
    /// taken: pushed later it would sit on top of the command's snapshot, and
    /// the first `⌘Z` would undo the two together while the second put the word
    /// back — a stack that walks backwards.
    #[gpui::test]
    fn a_command_met_mid_word_stacks_after_it(cx: &mut TestAppContext) {
        let scratch = Scratch::new("text-then-command");
        let workspace = workspace_on_a_view(&scratch, cx);

        workspace.update(cx, |workspace, cx| {
            workspace.drop_at(&[], 0, crate::designer::Dragged::Component("label"), cx);
            workspace.select(vec![0], cx);
            let prop = text_prop(workspace.view().expect("open").selected());
            // Read at the path and not at the selection: the command below
            // drops a node beside this one and selects it.
            let value = |workspace: &Workspace| {
                workspace
                    .view()
                    .expect("open")
                    .root
                    .at(&[0])
                    .and_then(|node| crate::registry::read(node, prop))
                    .unwrap_or_default()
            };
            let field = cx.new(|_| ()).entity_id();
            let before = value(workspace);
            let steps = workspace.view().expect("open").past.len();
            let shape_before = shape(&workspace.view().expect("open").root);

            workspace.begin_text_edit(field, TextSurface::Inspector, cx);
            workspace.edit_prop_text(prop, "Hello", cx);
            workspace.drop_at(&[], 1, crate::designer::Dragged::Component("label"), cx);

            assert_eq!(
                workspace.view().expect("open").past.len(),
                steps + 2,
                "the word and the command are two steps, not one"
            );
            workspace.undo(cx);
            assert_eq!(
                shape(&workspace.view().expect("open").root),
                shape_before,
                "the first ⌘Z takes back the command"
            );
            assert_eq!(value(workspace), "Hello", "and leaves the word that came before it");
            workspace.undo(cx);
            assert_eq!(value(workspace), before, "the second takes back the word");
        });
    }

    /// `⌘S` closes the step and leaves the field its session.
    ///
    /// Saving is an exit like the others — what was written before it should
    /// not come back with a later `⌘Z` — but the caret has not moved, so what
    /// is typed after it has to be a step of its own rather than typing on into
    /// nothing.
    ///
    /// `write_view` is what `⌘S` reaches, and `split_text_edit` its first line;
    /// driven here is the line, not the write, which formats through the
    /// settings store no windowless test has.
    #[gpui::test]
    fn saving_cuts_the_text_edit_in_two(cx: &mut TestAppContext) {
        let scratch = Scratch::new("text-save");
        let workspace = workspace_on_a_view(&scratch, cx);

        workspace.update(cx, |workspace, cx| {
            workspace.drop_at(&[], 0, crate::designer::Dragged::Component("label"), cx);
            workspace.select(vec![0], cx);
            let prop = text_prop(workspace.view().expect("open").selected());
            let value = |workspace: &Workspace| {
                crate::registry::read(workspace.view().expect("open").selected(), prop)
                    .unwrap_or_default()
            };
            let field = cx.new(|_| ()).entity_id();
            let steps = workspace.view().expect("open").past.len();

            workspace.begin_text_edit(field, TextSurface::Inspector, cx);
            workspace.edit_prop_text(prop, "Saved", cx);
            workspace.split_text_edit(cx);
            assert_eq!(
                workspace.view().expect("open").past.len(),
                steps + 1,
                "the save is a boundary"
            );
            assert_eq!(
                value(workspace),
                "Saved",
                "and the tree carries what the caret is still holding, so the file will too"
            );

            // The caret never left, so the keystrokes after the save are theirs.
            workspace.edit_prop_text(prop, "Saved then more", cx);
            workspace.close_text_edit_of(field, cx);
            assert_eq!(
                workspace.view().expect("open").past.len(),
                steps + 2,
                "what follows the save is its own step, not one lost for want of a session"
            );
            workspace.undo(cx);
            assert_eq!(value(workspace), "Saved");
        });
    }

    /// Dragging an edge is one undo step, whatever the drag writes on the way.
    ///
    /// The path a handle takes, driven without pixels: taking hold of it, then
    /// the sizes the moves would have written. A checkpoint per frame is the
    /// shape this rules out — sixty steps behind a gesture the hand made once,
    /// and a `⌘Z` that walks the node back a pixel at a time.
    #[gpui::test]
    fn dragging_a_handle_is_a_single_undo_step(cx: &mut TestAppContext) {
        let scratch = Scratch::new("handle-undo");
        let workspace = workspace_on_a_view(&scratch, cx);

        workspace.update(cx, |workspace, cx| {
            workspace.drop_at(&[], 0, crate::designer::Dragged::Component("label"), cx);
            workspace.select(vec![0], cx);
            let width = |workspace: &Workspace| {
                workspace
                    .view()
                    .expect("open")
                    .selected()
                    .call("w")
                    .map(|call| call.args[0].to_source())
            };
            assert_eq!(width(workspace), None, "a label arrives with no width of its own");
            let steps = workspace.view().expect("open").past.len();

            workspace.grab_handle();
            for size in [104., 137.4, 180.] {
                workspace.resize_selection(gpui::Axis::Horizontal, size, cx);
            }
            assert_eq!(
                width(workspace),
                Some("px(180.)".to_string()),
                "pixels are the only thing a drag can say"
            );
            assert_eq!(
                workspace.view().expect("open").past.len(),
                steps + 1,
                "the whole drag is one gesture"
            );

            workspace.undo(cx);
            assert_eq!(width(workspace), None, "and one ⌘Z takes the whole drag back");

            // The height is the other handle, and a separate call: a gesture on
            // one edge must not write the other.
            workspace.grab_handle();
            workspace.resize_selection(gpui::Axis::Vertical, 42., cx);
            assert_eq!(width(workspace), None);
            assert_eq!(
                workspace
                    .view()
                    .expect("open")
                    .selected()
                    .call("h")
                    .map(|call| call.args[0].to_source()),
                Some("px(42.)".to_string())
            );

            // A component the catalogue gives no box to — a spinner is not
            // `Styled` — is left alone rather than handed a call the generated
            // project cannot compile.
            workspace.drop_at(&[], 1, crate::designer::Dragged::Component("spinner"), cx);
            workspace.select(vec![1], cx);
            let steps = workspace.view().expect("open").past.len();
            workspace.grab_handle();
            workspace.resize_selection(gpui::Axis::Horizontal, 120., cx);
            assert!(workspace.view().expect("open").selected().call("w").is_none());
            assert_eq!(workspace.view().expect("open").past.len(), steps, "and nothing to undo");
        });
    }

    /// The box on the canvas writes what the inspector writes, in the same words.
    ///
    /// Two things at once, and they are the same thing said twice: the property
    /// a double click opens is the one holding what the component *says* — a
    /// button's first `Kind::Text` property is its element id, and typing a
    /// label into that is the opposite of the gesture — and once it is open the
    /// keystrokes go through `edit_prop_text`, so the tree, and therefore the
    /// file, cannot tell which panel typed them.
    #[gpui::test]
    fn typing_on_the_canvas_writes_what_the_inspector_writes(cx: &mut TestAppContext) {
        let scratch = Scratch::new("canvas-text");
        let workspace = workspace_on_a_view(&scratch, cx);

        workspace.update(cx, |workspace, cx| {
            workspace.drop_at(&[], 0, crate::designer::Dragged::Component("button"), cx);
            workspace.select(vec![0], cx);
            let node = workspace.view().expect("open").selected().clone();

            let spoken = crate::registry::spoken_text(&node).expect("a button says something");
            assert!(
                matches!(spoken.target, crate::registry::Target::Method("label")),
                "the words, not the element's own name"
            );
            assert!(
                !matches!(text_prop(&node).target, crate::registry::Target::Method("label")),
                "which the first text property is not: that one is the id"
            );

            // What the field's subscription does, keystroke by keystroke, with
            // a stand-in entity for the identity an `InputState` cannot have
            // without a window.
            let field = cx.new(|_| ()).entity_id();
            let steps = workspace.view().expect("open").past.len();
            workspace.begin_text_edit(field, TextSurface::Inspector, cx);
            for typed in ["E", "En", "Env", "Envoyer"] {
                workspace.edit_prop_text(spoken, typed, cx);
            }
            workspace.close_text_edit_of(field, cx);

            let node = workspace.view().expect("open").selected().clone();
            assert_eq!(crate::registry::read(&node, spoken).as_deref(), Some("Envoyer"));
            assert_eq!(
                workspace.view().expect("open").past.len(),
                steps + 1,
                "one visit to the box is one step, wherever the box was drawn"
            );

            // And it is the inspector's own field, not one beside it: the panel
            // finds its box by the address of the `&'static Prop`, so a
            // property that is one of `props` is one whose row the inspector is
            // already showing — the two panels write the same call, in the same
            // place in the chain, because it is the same entry of the
            // catalogue.
            let spec = crate::registry::of(&node).expect("the catalogue's");
            assert!(
                crate::registry::props(spec).iter().any(|prop| std::ptr::eq(*prop, spoken)),
                "the box on the canvas types into a field the inspector also offers"
            );
            assert!(crate::codegen::render(&node, 0).contains(".label(\"Envoyer\")"));
        });
    }
}
