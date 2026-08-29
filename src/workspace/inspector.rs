//! The inspector: selection, properties, state fields and the text being typed
//! into them.

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
        if self.rename_input.is_none() {
            self.rename_input = Some(cx.new(|cx| {
                InputState::new(window, cx).placeholder(crate::tr("designer.view_name"))
            }));
        }
        if self.state_name_input.is_none() {
            self.state_name_input = Some(cx.new(|cx| {
                InputState::new(window, cx).placeholder(crate::tr("designer.field_name"))
            }));
        }

        self.synced = key;
        self.prop_inputs.clear();
        self.brick_inputs.clear();

        // The selected node is cloned rather than borrowed: `self.view()`
        // borrows the whole workspace, and the loop below needs
        // `self.prop_inputs` mutably.
        let Some(node) = self.view().map(|view| view.selected().clone()) else {
            return;
        };
        let Some(spec) = crate::registry::of(&node) else {
            // Not the catalogue's, but possibly the project's own. One field per
            // builder method that takes a string; a method taking none is a
            // switch and needs no field.
            if let Some(brick) = self.brick_of(&node).cloned() {
                for prop in brick.props.iter().filter(|prop| prop.text) {
                    // A literal, or nothing yet, is free to type in. Anything
                    // else is an expression the developer wrote —
                    // `.subtitle(self.title.clone())` — and no field is offered
                    // for it: an empty box over their code, whose first
                    // keystroke replaces it with a string, is the one thing
                    // this whole file exists to prevent. The same rule
                    // `registry::editable` states for the catalogue.
                    let value = match node.call(&prop.method).and_then(|call| call.args.first()) {
                        None => String::new(),
                        Some(crate::model::Arg::Str(value)) => value.clone(),
                        Some(_) => continue,
                    };
                    let method = prop.method.clone();
                    let state = cx.new(|cx| InputState::new(window, cx).default_value(value));
                    cx.subscribe(&state, move |this, state, event: &InputEvent, cx| match event {
                        InputEvent::Change => {
                            let value = state.read(cx).value().to_string();
                            this.edit_brick_prop(&method, &value, cx);
                        }
                        // One undo step per visit to the field, exactly like the
                        // catalogue's. Without it, typing here was never pushed
                        // onto the stack at all: the first ⌘Z undid whatever came
                        // before and took the typing with it, in one go.
                        InputEvent::Focus => {
                            this.edit_snapshot =
                                this.view().map(|view| (view.path.clone(), view.root.clone()));
                        }
                        InputEvent::Blur | InputEvent::PressEnter { .. } => {
                            this.close_text_edit(cx)
                        }
                    })
                    .detach();
                    self.brick_inputs.push((prop.method.clone(), state));
                }
            }
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

    /// Renames the view the canvas is showing.
    ///
    /// Every view maxx creates is called `view_1`, `view_2`, … so this is the
    /// step between a view being made and a view being named. The occurrences
    /// maxx does not own are said and left alone: elsewhere in the project the
    /// old name may be a field, a comment or a string, and a tool that rewrites
    /// those is a tool nobody lets near a project twice.
    pub fn rename_view(&mut self, cx: &mut Context<Self>) {
        let Some(state) = self.rename_input.as_ref() else {
            return;
        };
        let renamed = state.read(cx).value().trim().to_string();
        if renamed.is_empty() {
            self.message = Some(crate::tr("message.name_the_view"));
            cx.notify();
            return;
        }
        let (Some(root), Some(path)) = (
            self.project.as_ref().map(|project| project.root.clone()),
            self.view().map(|view| view.path.clone()),
        ) else {
            return;
        };
        let Some(module) = super::view_module(&root, &path) else {
            self.message = Some(crate::tr("message.not_a_view"));
            cx.notify();
            return;
        };
        // Asked before anything moves, not explained after: the rename is made
        // from what is on disk, so an unsaved canvas would be dropped with the
        // tab — and the developer would learn it from the empty canvas.
        if self.view().is_some_and(|view| view.dirty()) {
            self.message = Some(crate::tr("message.view_unsaved_rename"));
            cx.notify();
            return;
        }

        match crate::scaffold::rename_view(&root, &module, &renamed) {
            Ok(elsewhere) => {
                // The tab has to follow the file, or the next save writes to
                // a path that is no longer there. Dropped rather than closed:
                // `close_view` refuses a dirty one, and the file it would be
                // protecting has already moved.
                self.views.retain(|view| view.path != path);
                self.active = None;
                self.previous_view = None;
                self.refresh_entries();
                self.select_file(root.join(format!("src/ui/{renamed}.rs")), cx);
                self.message = Some(SharedString::from(if elsewhere.is_empty() {
                    t!("message.view_renamed", name = renamed).into_owned()
                } else {
                    let names: Vec<String> = elsewhere
                        .iter()
                        .filter_map(|path| path.strip_prefix(&root).ok())
                        .map(|path| path.to_string_lossy().into_owned())
                        .collect();
                    t!("message.view_renamed_elsewhere", name = renamed, files = names.join(", "))
                        .into_owned()
                }));
            }
            Err(error) => self.message = Some(SharedString::from(error.to_string())),
        }
        self.revision += 1;
        cx.notify();
    }

    /// The rename box of the view panel.
    pub(crate) fn rename_input(&self) -> Option<&Entity<InputState>> {
        self.rename_input.as_ref()
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

    /// Closes an inspector text edit, turning it into a single undo step.
    pub(super) fn close_text_edit(&mut self, cx: &mut Context<Self>) {
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
    pub(super) fn edit_prop_text(
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
    pub(super) fn toggle_scrollbar(&mut self, on: bool, cx: &mut Context<Self>) {
        self.set_scrollbar(on, true, cx);
    }

    /// The same, saying whether this is a gesture of its own.
    ///
    /// Turning the scroll off takes the bar down with it — one gesture — and a
    /// checkpoint per write would make the first `⌘Z` restore a state nobody
    /// ever saw: a wrapper and a bar around a box that no longer scrolls.
    pub(super) fn set_scrollbar(&mut self, on: bool, record: bool, cx: &mut Context<Self>) {
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
}
