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
    use crate::workspace::Workspace;

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
            assert_eq!(showing(workspace), 0, "and it is not a mode: nothing covers it");

            // The reader is a document, so a mode covers it and does not
            // destroy it. Made a variant of `Center`, `⌘,` twice left the
            // designer with the file gone and its tab with it.
            workspace.toggle_preferences(cx);
            assert!(workspace.preferences(), "the settings cover it");
            assert!(workspace.code().is_some(), "the file is still there underneath");
            workspace.toggle_preferences(cx);
            assert!(!workspace.preferences());
            assert!(workspace.code().is_some(), "and comes back when the cover lifts");

            // Clicking the read tab brings it forward rather than closing it.
            workspace.toggle_preferences(cx);
            workspace.activate_code(cx);
            assert!(!workspace.preferences(), "the tab left the settings");
            assert!(workspace.code().is_some(), "and kept the file");

            // The gesture the reviewer found broken: the tab strip is drawn
            // above every mode precisely so it is the way back.
            workspace.select_file(root.join("src/theme.rs"), cx);
            assert!(workspace.palette().is_some());
            workspace.activate_view(0, cx);
            assert_eq!(showing(workspace), 0, "a tab click comes back to the view");
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
}
