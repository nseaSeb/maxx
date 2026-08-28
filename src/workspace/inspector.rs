//! The inspector: selection, properties, state fields, insertion and undo.

use super::*;

impl Workspace {
    /// Selects a node of the tree being designed.
    pub fn select(&mut self, path: NodePath, cx: &mut Context<Self>) {
        if let Some(view) = self.view_mut() {
            view.selected = path;
            cx.notify();
        }
    }

    /// The fields of the view able to back the selected component.
    ///
    /// Filtered on the type the component needs: offering the field of a text
    /// input to a dropdown would be offering something that will not compile.
    pub(crate) fn state_fields(&self) -> Vec<String> {
        let Some(view) = self.view() else {
            return Vec::new();
        };
        let Some(state) =
            view.root.at(&view.selected).and_then(registry::of).and_then(|spec| spec.state)
        else {
            return Vec::new();
        };
        view.state_fields_of_type(state.ty)
    }

    /// The inspector field bound to `prop`, if it has been built.
    pub(crate) fn prop_input(
        &self,
        prop: &'static crate::registry::Prop,
    ) -> Option<&Entity<InputState>> {
        self.prop_inputs
            .iter()
            .find(|(candidate, _)| std::ptr::eq(*candidate, prop))
            .map(|(_, state)| state)
    }

    /// Rebuilds the inspector's text fields when the selection or the tree has
    /// changed. Called once per frame from `render`, which is the only place
    /// holding both `&mut self` and a `Window`.
    pub(super) fn sync_prop_inputs(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        // Built before the early return: the palette's search box does not
        // depend on the selection, and a workspace with no view open still
        // shows the palette.
        if self.palette_filter.is_none() {
            let filter =
                cx.new(|cx| InputState::new(window, cx).placeholder(crate::tr("designer.search")));
            cx.subscribe(&filter, |_, _, _: &InputEvent, cx| cx.notify()).detach();
            self.palette_filter = Some(filter);
        }

        let key = self.view().map(|view| (self.revision, view.selected.clone()));
        if key == self.synced {
            return;
        }
        if self.state_name_input.is_none() {
            self.state_name_input = Some(cx.new(|cx| {
                InputState::new(window, cx).placeholder(crate::tr("designer.field_name"))
            }));
        }

        self.synced = key;
        self.prop_inputs.clear();

        // The selected node is cloned rather than borrowed: `self.view()`
        // borrows the whole workspace, and the loop below needs
        // `self.prop_inputs` mutably.
        let Some(node) = self.view().map(|view| view.selected().clone()) else {
            return;
        };
        let Some(spec) = crate::registry::of(&node) else {
            return;
        };

        // How many pixels the picture really has, which is what makes a width
        // thinkable: 400 and 4000 ask for different numbers, and the field says
        // neither. Read here rather than in `render`, which runs on every
        // frame — the guard above is what keeps it to one read per selection.
        self.image_size = None;
        if spec.id == "image"
            && let Some(root) = self.project().map(|project| project.root.clone())
            && let Some(prop) = spec.props.first()
            && let Some(value) = crate::registry::read(&node, prop).filter(|v| !v.is_empty())
        {
            self.image_size = image::image_dimensions(root.join(value)).ok();
        }

        for prop in crate::registry::props(spec) {
            if !matches!(
                prop.kind,
                Kind::Text
                    | Kind::Field
                    | Kind::Handler
                    | Kind::Number
                    | Kind::Color
                    | Kind::Ratio
                    | Kind::Count
                    | Kind::Path
            ) || !crate::registry::editable(&node, prop)
            {
                continue;
            }
            let value = crate::registry::read(&node, prop).unwrap_or_default();
            let state = cx.new(|cx| InputState::new(window, cx).default_value(value));
            cx.subscribe(&state, move |this, state, event: &InputEvent, cx| match event {
                InputEvent::Change => {
                    let value = state.read(cx).value().to_string();
                    this.edit_prop_text(prop, &value, cx);
                }
                // A checkpoint per keystroke would flood the undo stack and,
                // through the revision counter, rebuild the field under the
                // caret. One per visit to the field is the right grain.
                InputEvent::Focus => {
                    this.edit_snapshot =
                        this.view().map(|view| (view.path.clone(), view.root.clone()));
                }
                InputEvent::Blur => this.close_text_edit(cx),
                InputEvent::PressEnter { .. } => this.close_text_edit(cx),
            })
            .detach();
            self.prop_inputs.push((prop, state));
        }
    }

    /// Binds a text property to a state field, or unbinds it.
    pub fn toggle_binding(&mut self, prop: &'static crate::registry::Prop, cx: &mut Context<Self>) {
        let Some(view) = self.view() else {
            return;
        };
        let selected = view.selected.clone();
        let bound = view.root.at(&selected).and_then(|node| registry::read_binding(node, prop));
        let fields = view.state_fields();

        let expression = match bound {
            Some(_) => None,
            None => match fields.first() {
                Some(field) => Some(field.read_expression()),
                None => {
                    self.message = Some(crate::tr("message.no_state_field"));
                    cx.notify();
                    return;
                }
            },
        };

        self.checkpoint();
        let view = self.view_mut().expect("just borrowed");
        if let Some(node) = view.root.at_mut(&selected) {
            registry::write_binding(node, prop, expression.as_deref());
        }
        cx.notify();
    }

    /// Moves a bound property to the next state field.
    pub fn cycle_binding(&mut self, prop: &'static crate::registry::Prop, cx: &mut Context<Self>) {
        let Some(view) = self.view() else {
            return;
        };
        let selected = view.selected.clone();
        let fields = view.state_fields();
        if fields.is_empty() {
            return;
        }
        let current = view.root.at(&selected).and_then(|node| registry::read_binding(node, prop));
        let index = current
            .and_then(|name| fields.iter().position(|field| field.name == name))
            .map(|index| (index + 1) % fields.len())
            .unwrap_or(0);
        let expression = fields[index].read_expression();

        self.checkpoint();
        let view = self.view_mut().expect("just borrowed");
        if let Some(node) = view.root.at_mut(&selected) {
            registry::write_binding(node, prop, Some(&expression));
        }
        cx.notify();
    }

    /// Adds a field to the view's struct.
    pub fn add_state_field(&mut self, cx: &mut Context<Self>) {
        let Some(state) = self.state_name_input.as_ref() else {
            return;
        };
        let name = state.read(cx).value().trim().to_string();
        if name.is_empty() {
            self.message = Some(crate::tr("message.name_the_field"));
            cx.notify();
            return;
        }
        let (_, ty, initial) = crate::view::STATE_TYPES[self.state_type];

        self.message = match self.view_mut() {
            Some(view) => match view.add_state_field(&name, ty, initial) {
                Ok(()) => {
                    Some(SharedString::from(t!("message.field_added", name = name).into_owned()))
                }
                Err(error) => Some(SharedString::from(error)),
            },
            None => None,
        };
        self.revision += 1;
        cx.notify();
    }

    /// Steps through the kinds of field the state panel offers.
    pub fn cycle_state_type(&mut self, cx: &mut Context<Self>) {
        self.state_type = (self.state_type + 1) % crate::view::STATE_TYPES.len();
        cx.notify();
    }

    /// The field-name box of the state panel.
    pub(crate) fn state_name_input(&self) -> Option<&Entity<InputState>> {
        self.state_name_input.as_ref()
    }

    /// The palette's search box.
    pub(crate) fn palette_filter(&self) -> Option<&Entity<InputState>> {
        self.palette_filter.as_ref()
    }

    /// Which kind of field the state panel will add.
    pub(crate) fn state_type(&self) -> usize {
        self.state_type
    }

    /// Opens what is being edited in Zed: the file if one is open, the project
    /// otherwise.
    ///
    /// Opening the folder when a view is on screen means finding the file again
    /// by hand, which is the whole gesture one wanted to avoid.
    pub fn open_in_editor(&mut self, cx: &mut Context<Self>) {
        // The explorer selection comes first: it is what the context menu is
        // about, and a left click sets it to the open view anyway.
        let path = self
            .selected
            .clone()
            .or_else(|| self.menu_file.as_ref().map(|menus| menus.path.clone()))
            .or_else(|| self.view().map(|view| view.path.clone()))
            .or_else(|| self.project().map(|project| project.root.clone()));

        match path {
            Some(path) => crate::tools::open_in_editor(cx, &path, None),
            None => {
                self.message = Some(crate::tr("message.no_project"));
                cx.notify();
            }
        }
    }

    /// Opens the handler of a property in Zed, on its own line.
    pub fn open_handler(&mut self, prop: &'static crate::registry::Prop, cx: &mut Context<Self>) {
        let Some(view) = self.view() else {
            return;
        };
        let node = view.selected();
        let Some(name) = registry::read(node, prop).filter(|name| !name.is_empty()) else {
            self.message = Some(crate::tr("message.no_action"));
            cx.notify();
            return;
        };
        match view.method_line(&name) {
            Some(line) => crate::tools::open_in_editor(cx, &view.path, Some(line)),
            None => {
                self.message = Some(SharedString::from(
                    t!("message.handler_unwritten", name = name).into_owned(),
                ));
                cx.notify();
            }
        }
    }

    /// Fills the selected node's handler with the body that opens a box.
    ///
    /// Written straight to the file rather than into the tree: a handler is not
    /// part of the managed region — it is a method of the view, beside it — so
    /// there is nothing here for `⌘S` to carry. The view is re-read afterwards,
    /// for the reason `format_after_save` gives: maxx holds a copy of the file
    /// and compares it with the disk, and a copy left behind would make the
    /// next save believe someone else had written.
    pub fn fill_handler(
        &mut self,
        prop: &'static crate::registry::Prop,
        kind: &'static str,
        cx: &mut Context<Self>,
    ) {
        let Some(view) = self.view() else {
            return;
        };
        let Some(name) = registry::read(view.selected(), prop).filter(|name| !name.is_empty())
        else {
            self.message = Some(crate::tr("message.no_action"));
            cx.notify();
            return;
        };

        let filled = match crate::view::fill_handler(&view.source, &name, kind) {
            Ok(filled) => filled,
            Err(error) => {
                self.message = Some(SharedString::from(error));
                cx.notify();
                return;
            }
        };

        let path = view.path.clone();
        if let Err(error) = std::fs::write(&path, &filled) {
            self.message = Some(SharedString::from(error.to_string()));
            cx.notify();
            return;
        }
        if let Some(view) = self.view_mut()
            && let Err(error) = view.reload()
        {
            self.message = Some(SharedString::from(error));
            cx.notify();
            return;
        }
        self.message = Some(SharedString::from(
            t!("message.handler_filled", name = name, kind = kind).into_owned(),
        ));
        self.revision += 1;
        cx.notify();
    }

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

    /// Closes an inspector text edit, turning it into a single undo step.
    fn close_text_edit(&mut self, cx: &mut Context<Self>) {
        let Some((path, before)) = self.edit_snapshot.take() else {
            return;
        };
        let Some(view) = self.view_mut() else {
            return;
        };
        // The tab may have changed, or the view may have been reloaded, since
        // the field took the focus; that snapshot belongs to neither.
        if view.path != path || view.root == before {
            return;
        }
        view.past.push(before);
        view.future.clear();
        self.revision += 1;
        cx.notify();
    }

    /// Moves a text input to the next field of the view able to back it.
    pub fn cycle_input_field(
        &mut self,
        prop: &'static crate::registry::Prop,
        cx: &mut Context<Self>,
    ) {
        let fields = self.state_fields();
        let Some(view) = self.view() else {
            return;
        };
        if fields.is_empty() {
            return;
        }
        let selected = view.selected.clone();
        let current = view.root.at(&selected).and_then(|node| registry::read(node, prop));
        let index = current
            .and_then(|name| fields.iter().position(|field| *field == name))
            .map(|index| (index + 1) % fields.len())
            .unwrap_or(0);
        let next = fields[index].clone();

        self.checkpoint();
        let view = self.view_mut().expect("just borrowed");
        if let Some(node) = view.root.at_mut(&selected) {
            registry::write(node, prop, &next);
        }
        cx.notify();
    }

    /// Writes a text property without disturbing the field being typed in: no
    /// undo checkpoint per keystroke, and no revision bump, so `sync` leaves the
    /// caret alone.
    fn edit_prop_text(
        &mut self,
        prop: &'static crate::registry::Prop,
        value: &str,
        cx: &mut Context<Self>,
    ) {
        let Some(view) = self.view_mut() else {
            return;
        };
        let selected = view.selected.clone();
        // A tooltip lives on a stateful element, so it needs an id — and an id
        // has to be unique among siblings, which no node can know. Read here,
        // where the tree is, exactly like the scroll's.
        let tooltip_id = matches!(prop.target, crate::registry::Target::Tooltip)
            .then(|| crate::registry::unique_element_id(&view.root));
        if let Some(node) = view.root.at_mut(&selected) {
            let current = crate::registry::read(node, prop);
            if current.as_deref() == Some(value) {
                return;
            }
            if let Some(id) = tooltip_id
                && !value.trim().is_empty()
                && node.call("id").is_none()
            {
                node.set_call("id", crate::model::Arg::Str(id));
            }
            crate::registry::write(node, prop, value);
        }
        // The pixel size on screen belongs to the path that was there: keeping
        // it beside a path being typed would put a true number next to a wrong
        // file. Cleared here, read again when the edit closes — which bumps the
        // revision, and that is what `sync_prop_inputs` waits for.
        if matches!(prop.kind, Kind::Path) {
            self.image_size = None;
        }
        self.message = crate::registry::validate(prop, value).map(crate::tr);
        cx.notify();
    }

    /// Records the current tree so the change about to be made can be undone.
    fn checkpoint(&mut self) {
        self.revision += 1;
        if let Some(view) = self.view_mut() {
            let snapshot = view.root.clone();
            view.past.push(snapshot);
            view.future.clear();
        }
    }

    /// Asks for an image and brings it into the project.
    ///
    /// A file from anywhere on the disk is copied under `assets/images/` rather
    /// than refused: the generated binary reads from the directory it starts
    /// in — the project root — so a path pointing outside would draw on this
    /// canvas and nowhere else. The project has to carry its own images.
    ///
    /// The dialog does not filter: `PathPromptOptions` has no extension list to
    /// give it. What is not an image gpui can decode is refused on the way in,
    /// by [`crate::scaffold::import_asset`], which reads gpui's own list.
    pub fn pick_path(&mut self, prop: &'static crate::registry::Prop, cx: &mut Context<Self>) {
        let Some(root) = self.project().map(|project| project.root.clone()) else {
            return;
        };
        // The node the dialog was opened from, remembered: the panel is not
        // modal everywhere, and `edit_prop` writes to whatever is selected when
        // it runs. Selecting something else while the dialog is open would land
        // the path — and the copy — somewhere nobody asked for.
        // The constructor path goes with it: an index path alone says where, not
        // what. Undo the insertion while the panel is open and the same index
        // holds another node — an argument-less `v_flex`, which takes the path
        // as its first argument without complaining, and the generated project
        // stops compiling on `v_flex(PathBuf::from(..))`.
        let opened_on = self.view().map(|view| {
            (
                view.path.clone(),
                view.selected.clone(),
                view.selected().base.path().map(str::to_string),
            )
        });
        let paths = cx.prompt_for_paths(gpui::PathPromptOptions {
            files: true,
            directories: false,
            multiple: false,
            prompt: Some(crate::tr("designer.choose")),
        });

        cx.spawn(async move |this, cx| {
            if let Ok(Ok(Some(paths))) = paths.await
                && let Some(path) = paths.into_iter().next()
            {
                let still_there = this
                    .update(cx, |this, _| {
                        this.view().map(|view| {
                            (
                                view.path.clone(),
                                view.selected.clone(),
                                view.selected().base.path().map(str::to_string),
                            )
                        }) == opened_on
                    })
                    .unwrap_or(false);
                if !still_there {
                    return;
                }
                // Copied off the interface thread: a photograph is read whole
                // and written whole, and the window would stop answering for
                // as long as that takes — which is exactly what the reader's
                // ceiling exists to avoid.
                let imported = cx
                    .background_spawn(async move { crate::scaffold::import_asset(&root, &path) })
                    .await;

                this.update(cx, |this, cx| {
                    match imported {
                        Ok(value) => {
                            this.edit_prop(prop, &value, cx);
                            // The copy created `assets/images/` a moment ago,
                            // and the panel lists what it read when the project
                            // opened: without this, maxx writes a file it does
                            // not show.
                            this.refresh_entries();
                        }
                        Err(error) => {
                            this.message = Some(SharedString::from(error));
                            cx.notify();
                        }
                    }
                })
                .ok();
            }
        })
        .detach();
    }

    /// Writes a property of the selected node.
    pub fn edit_prop(
        &mut self,
        prop: &'static crate::registry::Prop,
        value: &str,
        cx: &mut Context<Self>,
    ) {
        let Some(view) = self.view_mut() else {
            return;
        };
        let selected = view.selected.clone();
        // An element id has to be unique among siblings, and no node can know
        // what its siblings carry: the tree is here, so the id is assigned
        // here, before the catalogue writes the rest.
        let scroll_id = matches!(prop.target, registry::Target::Scrollable(_))
            .then(|| registry::unique_element_id(&view.root));
        // The bar is not a call on the node but a shape around it, so it is not
        // the catalogue's to write: it needs the parent, and two names only the
        // whole tree can give.
        if matches!(prop.target, registry::Target::Scrollbar) {
            self.toggle_scrollbar(value == "true", cx);
            return;
        }
        if view.root.at(&selected).is_some() {
            self.checkpoint();
            let view = self.view_mut().expect("just borrowed");
            if let Some(node) = view.root.at_mut(&selected) {
                if let Some(id) = scroll_id
                    && value == "true"
                    && node.call("id").is_none()
                {
                    node.set_call("id", crate::model::Arg::Str(id));
                }
                registry::write(node, prop, value);
            }
        }
        // A bar over a box that no longer scrolls is a bar watching something
        // that never moves: turning the scroll off takes it down with it.
        if matches!(prop.target, registry::Target::Scrollable(_)) && value != "true" {
            // The checkpoint above already covers this: one gesture, one step
            // back.
            self.set_scrollbar(false, false, cx);
        }
        cx.notify();
    }

    /// Puts a visible scrollbar around the selected box, or takes it away.
    ///
    /// The one property that changes the shape of the tree rather than a call:
    /// a bar has to be a *sibling* of the box under a `relative` parent, since
    /// gpui moves every child of a scrolling element — an absolute one included
    /// — by the scroll offset. So maxx wraps, and unwraps.
    ///
    /// The selection follows the box, not the wrapper: it is the box the
    /// developer clicked, and its properties are the ones the inspector was
    /// showing.
    fn toggle_scrollbar(&mut self, on: bool, cx: &mut Context<Self>) {
        self.set_scrollbar(on, true, cx);
    }

    /// The same, saying whether this is a gesture of its own.
    ///
    /// Turning the scroll off takes the bar down with it — one gesture — and a
    /// checkpoint per write would make the first `⌘Z` restore a state nobody
    /// ever saw: a wrapper and a bar around a box that no longer scrolls.
    fn set_scrollbar(&mut self, on: bool, record: bool, cx: &mut Context<Self>) {
        let Some(view) = self.view() else {
            return;
        };
        let selected = view.selected.clone();
        let Some(node) = view.root.at(&selected) else {
            return;
        };
        let already = node.call("track_scroll").is_some();
        if on == already {
            return;
        }

        if on {
            // Two, not one: the box becomes a stateful element — that is what
            // `overflow_*_scroll` and `track_scroll` need — and the bar is an
            // element of its own.
            let [box_id, bar_id] = registry::unique_element_ids(&view.root);
            let field = registry::unique_input_field(&view.root);
            if record {
                self.checkpoint();
            }
            let view = self.view_mut().expect("just borrowed");
            // The root has no parent to be replaced in, so it is replaced
            // itself: the wrapper becomes the view's outermost element.
            if selected.is_empty() {
                let assembly =
                    registry::scrollbar_assembly(view.root.clone(), [&box_id, &bar_id], &field);
                view.root = assembly;
                view.selected = vec![0];
            } else if let Some(box_node) = view.root.remove(&selected) {
                let assembly = registry::scrollbar_assembly(box_node, [&box_id, &bar_id], &field);
                view.root.insert(&selected, assembly);
                let mut inside = selected.clone();
                inside.push(0);
                view.selected = inside;
            }
            self.revision += 1;
            cx.notify();
            return;
        }

        // Off: the box is inside the wrapper, so what has to go is one level
        // up — and only if it is a wrapper maxx wrote. A `track_scroll` the
        // developer put there by hand is theirs, and stays.
        let Some((_, parent_path)) = selected.split_last() else {
            return;
        };
        let parent_path = parent_path.to_vec();
        let Some(unwrapped) = view.root.at(&parent_path).and_then(registry::unwrap_scrollbar)
        else {
            return;
        };

        if record {
            self.checkpoint();
        }
        let view = self.view_mut().expect("just borrowed");
        if parent_path.is_empty() {
            view.root = unwrapped;
            view.selected = Vec::new();
        } else {
            view.root.remove(&parent_path);
            view.root.insert(&parent_path, unwrapped);
            view.selected = parent_path;
        }
        self.revision += 1;
        cx.notify();
    }

    /// Inserts a component into the selected container, or beside the selected
    /// node when it cannot hold children.
    /// Where a new node goes, given the selection.
    ///
    /// Inside the selected node when it takes children, just after it
    /// otherwise. Answers `None`, having said why, when there is nowhere: the
    /// root is not a container, so nothing can be dropped beside it.
    fn insertion_point(&mut self, cx: &mut Context<Self>) -> Option<crate::model::Path> {
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

    /// Gives the selected node a handler, named after it, if its component has
    /// an action property and none is set yet. Bound to double-click.
    pub fn add_handler_to_selection(&mut self, cx: &mut Context<Self>) {
        let Some(view) = self.view() else {
            return;
        };
        let node = view.selected();
        let Some(spec) = registry::of(node) else {
            return;
        };
        let Some(prop) =
            spec.props.iter().find(|prop| matches!(prop.kind, crate::registry::Kind::Handler))
        else {
            return;
        };
        if registry::read(node, prop).is_some_and(|name| !name.is_empty()) {
            return;
        }

        let name = registry::suggested_handler(node);
        let selected = view.selected.clone();
        self.checkpoint();
        let view = self.view_mut().expect("just borrowed");
        if let Some(node) = view.root.at_mut(&selected) {
            registry::write(node, prop, &name);
        }
        self.message =
            Some(SharedString::from(t!("message.action_written", name = name).into_owned()));
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
