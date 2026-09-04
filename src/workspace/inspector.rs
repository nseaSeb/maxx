//! The inspector: selection, properties, state fields and the text being typed
//! into them.

use super::*;

// For `InputState::focus_handle`: `⏎` in the tree hands the caret to the
// inspector's box, and the handle is what `Window::focus` takes.
use gpui::Focusable as _;

impl Workspace {
    /// Selects a node of the tree being designed.
    ///
    /// A selection that moves ends the text edit in progress: the field about
    /// to be rebuilt for another node is the one holding the session, and a
    /// snapshot outliving it would be pushed against a tree it no longer
    /// describes.
    pub fn select(&mut self, path: NodePath, cx: &mut Context<Self>) {
        self.close_text_edit(cx);
        // The box on the canvas belongs to the node it stands over. Left open,
        // it would go on writing into `view.selected` — which is about to be
        // some other node.
        self.canvas_edit = None;
        if let Some(view) = self.view_mut() {
            view.selected = path;
            cx.notify();
        }
    }

    /// Moves the selection to the previous or next row of the structure tree.
    ///
    /// The order is the tree's own — depth first, a parent before its children —
    /// because that is the order the rows are painted in, and the arrow keys
    /// have to agree with what the eye sees. Nothing folds in the tree yet, so
    /// every node is a visible row and there is no hidden one to skip.
    ///
    /// Both ends stop rather than wrap: a cursor that leaps from the last row to
    /// the first is a cursor you then have to go looking for.
    pub fn step_selection(&mut self, forward: bool, cx: &mut Context<Self>) {
        let Some(view) = self.view() else {
            return;
        };
        let mut rows: Vec<NodePath> = Vec::new();
        view.root.walk(&mut |path, _| rows.push(path.to_vec()));
        let Some(current) = rows.iter().position(|path| *path == view.selected) else {
            return;
        };
        let Some(next) = (if forward { current.checked_add(1) } else { current.checked_sub(1) })
        else {
            return;
        };
        let Some(path) = rows.get(next).cloned() else {
            return;
        };
        self.select(path, cx);
    }

    /// Moves the selection to the parent of the selected node, or to its first
    /// child.
    ///
    /// `←` and `→` fold and unfold a row in most trees; this one has nothing to
    /// fold — every node is always shown — so the two keys walk the depth
    /// instead, which is the other thing a hand reaches for them to do.
    pub fn step_depth(&mut self, inward: bool, cx: &mut Context<Self>) {
        let Some(view) = self.view() else {
            return;
        };
        let mut path = view.selected.clone();
        if inward {
            let Some(node) = view.root.at(&path) else {
                return;
            };
            if node.children.is_empty() {
                return;
            }
            path.push(0);
        } else if path.pop().is_none() {
            return;
        }
        self.select(path, cx);
    }

    /// Puts the caret in the inspector's text box for the selected node.
    ///
    /// `⏎` on a row is "let me type the label", and the box that answers is the
    /// one the inspector already built for the first Text property of the
    /// component. A node with no text — a divider, a column — does nothing,
    /// rather than moving the focus somewhere the key never promised.
    pub fn focus_prop_text(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        // The property the component *says*, not the first one that happens to
        // be text: a button's first is `prop.id`, so `⏎` renamed the element —
        // taking the tooltip and the handler hung on that id with it — and left
        // the label untouched. `spoken_text` is the same question the canvas's
        // double-click asks, answered from the table of groups.
        let Some(node) = self.view().map(|view| view.selected().clone()) else {
            return;
        };
        let Some(wanted) = crate::registry::spoken_text(&node) else {
            return;
        };
        let Some((_, state)) =
            self.prop_inputs.iter().find(|(prop, _)| std::ptr::eq(*prop, wanted))
        else {
            return;
        };
        window.focus(&state.read(cx).focus_handle(cx));
        cx.notify();
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
        // Beside it and for the same reason: built once, since the box is not a
        // function of the selection and rebuilding it would take the caret away
        // from whoever is typing in it.
        if self.prop_filter.is_none() {
            let filter = cx.new(|cx| {
                InputState::new(window, cx).placeholder(crate::tr("designer.find_a_property"))
            });
            cx.subscribe(&filter, |_, _, _: &InputEvent, cx| cx.notify()).detach();
            self.prop_filter = Some(filter);
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
                            this.begin_text_edit(state.entity_id(), TextSurface::Inspector, cx)
                        }
                        InputEvent::Blur => this.close_text_edit_of(state.entity_id(), cx),
                        // `⏎` keeps the caret where it is, so it opens the next
                        // session as it closes this one — like the save does.
                        InputEvent::PressEnter { .. } => {
                            this.begin_text_edit(state.entity_id(), TextSurface::Inspector, cx)
                        }
                    })
                    .detach();
                    self.brick_inputs.push((prop.method.clone(), state));
                }
            }
            return;
        };

        self.sync_image_size();

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
            let value = match prop.target {
                // The node holds nothing of this one; `new` does.
                crate::registry::Target::Initializer(_) => self.initializer_value(prop),
                _ => crate::registry::read(&node, prop),
            }
            .unwrap_or_default();
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
                    this.begin_text_edit(state.entity_id(), TextSurface::Inspector, cx)
                }
                InputEvent::Blur => this.close_text_edit_of(state.entity_id(), cx),
                // `⏎` keeps the caret where it is, so it opens the next session
                // as it closes this one — like the save does.
                InputEvent::PressEnter { .. } => {
                    this.begin_text_edit(state.entity_id(), TextSurface::Inspector, cx)
                }
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

    /// How many pixels the selected picture really has.
    ///
    /// Which is what makes a width thinkable: 400 and 4000 ask for different
    /// numbers, and the field says neither. Read here rather than in `render`,
    /// which runs on every frame — once per selection, and once more when a
    /// text edit closes, since the path may be exactly what was typed.
    fn sync_image_size(&mut self) {
        self.image_size = None;
        let Some(node) = self.view().map(|view| view.selected().clone()) else {
            return;
        };
        let Some(spec) = crate::registry::of(&node) else {
            return;
        };
        // Asked of the property and not of the entry's name: the avatar carries
        // a picture too, and an identifier here would have left it without its
        // pixel count for no reason a reader could find.
        if let Some(root) = self.project().map(|project| project.root.clone())
            && let Some(prop) = spec.props.iter().find(|prop| matches!(prop.kind, Kind::Path))
            && let Some(value) = crate::registry::read(&node, prop).filter(|v| !v.is_empty())
            // A drawing has no pixels to count: an SVG is a description, and
            // `image_dimensions` answers with an unsupported-format error
            // rather than a size. Nothing would be shown either way — the read
            // is fallible and its failure is already the empty case — but
            // asking first says why, and stops a future decoder from quietly
            // reporting the raster size of a file that has none.
            && !value.to_ascii_lowercase().ends_with(".svg")
        {
            self.image_size = image::image_dimensions(root.join(value)).ok();
        }
    }

    /// The field the selected component is bound to, when it has one.
    fn bound_field(&self) -> Option<String> {
        let view = self.view()?;
        let node = view.root.at(&view.selected)?;
        crate::registry::of(node)?.state?;
        let crate::model::Base::Known { args, .. } = &node.base else {
            return None;
        };
        args.first()?.to_source().strip_prefix("&self.").map(str::to_string)
    }

    /// What a property living in the state field's initializer currently holds.
    ///
    /// `None` when the line in `new` is not one maxx wrote: the same rule the
    /// handlers follow — what the developer has changed is theirs, and the
    /// inspector shows it rather than offering to overwrite it.
    pub(crate) fn initializer_value(&self, prop: &'static crate::registry::Prop) -> Option<String> {
        let crate::registry::Target::Initializer(init) = prop.target else {
            return None;
        };
        let field = self.bound_field()?;
        let source = self.view()?.state_initializer(&field)?;
        init.read(prop.kind, &source)
    }

    /// Writes such a property back into `new`.
    ///
    /// No checkpoint: the undo stack holds trees, and this line is not in the
    /// tree. Saying it plainly rather than taking one that would restore
    /// nothing — the same place `add_state_field` already stands.
    pub(super) fn edit_initializer(
        &mut self,
        prop: &'static crate::registry::Prop,
        value: &str,
        cx: &mut Context<Self>,
    ) {
        let crate::registry::Target::Initializer(init) = prop.target else {
            return;
        };
        // Only what maxx itself wrote is replaced: reading it back is the proof.
        if self.initializer_value(prop).is_none() {
            self.message = Some(crate::tr("message.initializer_is_yours"));
            cx.notify();
            return;
        }
        let Some(field) = self.bound_field() else {
            return;
        };
        // Said here and not in `edit_prop_text`, which hands this one over
        // before it gets to `validate`: a keystroke swallowed with no word is
        // exactly the silence `validate` exists to break.
        self.message = crate::registry::validate(prop, value).map(crate::tr);
        let Some(text) = init.write(prop.kind, value) else {
            cx.notify();
            return;
        };
        if let Some(view) = self.view_mut() {
            view.set_state_initializer(&field, &text);
        }
        cx.notify();
    }

    /// Opens a box over a node of the canvas, on the words it says out loud.
    ///
    /// The whole point is that the words are typed where they are read, so the
    /// box is the inspector's box moved onto the board: the same `Input`, the
    /// same subscription, and above all the same session — [`Self::begin_text_edit`]
    /// keyed on the field's own [`EntityId`]. A second mechanism beside that one
    /// would be a second grain of undo for the same act of typing.
    ///
    /// Built here and not in a `sync_…` pass, unlike every other field of the
    /// window: this one exists only while it is open, and a field rebuilt once
    /// per frame is a caret dropped once per frame.
    ///
    /// Answers whether it opened, so the double click can fall back to what it
    /// meant before on a node with nothing to say.
    pub fn edit_text_on_canvas(
        &mut self,
        path: NodePath,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        let Some(node) = self.view().and_then(|view| view.root.at(&path)).cloned() else {
            return false;
        };
        let Some(prop) = registry::spoken_text(&node) else {
            return false;
        };
        let value = registry::read(&node, prop).unwrap_or_default();
        let state = cx.new(|cx| InputState::new(window, cx).default_value(value));
        cx.subscribe(&state, move |this, state, event: &InputEvent, cx| match event {
            InputEvent::Change => {
                let value = state.read(cx).value().to_string();
                this.edit_prop_text(prop, &value, cx);
            }
            InputEvent::Focus => this.begin_text_edit(state.entity_id(), TextSurface::Canvas, cx),
            // The two ways out, and both take the box away with the step: what
            // was typed is already in the tree, the caret having put it there.
            // `Escape` is not among them — gpui-component lets it bubble
            // without emitting anything, so nothing here could hear it.
            InputEvent::Blur | InputEvent::PressEnter { .. } => {
                this.close_text_edit_of(state.entity_id(), cx);
                this.canvas_edit = None;
                cx.notify();
            }
        })
        .detach();

        let focus = state.read(cx).focus_handle(cx);
        window.focus(&focus);
        // Preselected, so the first keystroke replaces the old words — which is
        // what a double click on a label promises. On the next frame because an
        // action travels the dispatch tree of the frame that was *drawn*, and
        // the box is not in that one yet.
        window.on_next_frame(move |window, cx| {
            focus.dispatch_action(&gpui_component::input::SelectAll, window, cx);
        });
        self.canvas_edit = Some((path, prop, state));
        cx.notify();
        true
    }

    /// The box open over the canvas, for the board that draws it.
    pub(crate) fn canvas_edit(&self) -> Option<(&[usize], &Entity<InputState>)> {
        self.canvas_edit.as_ref().map(|(path, _, state)| (path.as_slice(), state))
    }

    /// Opens a text edit on `field`, closing whatever session was open.
    ///
    /// Closing here and not only on blur is what holds the grain in both
    /// directions: gpui hands one focus event to every listener in subscription
    /// order, so a caret moving *up* the inspector reaches this before the
    /// field it left says a word.
    pub(super) fn begin_text_edit(
        &mut self,
        field: EntityId,
        surface: TextSurface,
        cx: &mut Context<Self>,
    ) {
        self.close_text_edit(cx);
        self.edit_snapshot =
            self.view().map(|view| (field, surface, view.path.clone(), view.root.clone()));
    }

    /// Closes the text edit `field` opened, and only that one.
    ///
    /// A blur arriving after the next field has already claimed the session
    /// must leave it alone, or the word about to be typed there would be
    /// written with nothing to take it back.
    pub(super) fn close_text_edit_of(&mut self, field: EntityId, cx: &mut Context<Self>) {
        if self.edit_snapshot.as_ref().is_some_and(|(owner, ..)| *owner == field) {
            self.close_text_edit(cx);
        }
    }

    /// Ends the text edit at a save and opens the next one on the same field.
    ///
    /// `⌘S` is an exit like the others — what was typed before it is one step —
    /// but the caret has not moved, so the field keeps a session for what
    /// follows instead of typing on into nothing.
    pub(super) fn split_text_edit(&mut self, cx: &mut Context<Self>) {
        let Some((field, surface)) =
            self.edit_snapshot.as_ref().map(|(field, surface, ..)| (*field, *surface))
        else {
            return;
        };
        self.begin_text_edit(field, surface, cx);
    }

    /// Closes an inspector text edit, turning it into a single undo step.
    pub(super) fn close_text_edit(&mut self, cx: &mut Context<Self>) {
        if self.take_text_step() {
            cx.notify();
        }
    }

    /// Records the text edit in progress as a step, and says whether it made
    /// one.
    ///
    /// The half of [`Workspace::close_text_edit`] that asks for no context, so
    /// that `checkpoint` — which has none — can take the step before its own.
    /// A structural command can reach the workspace with a field still holding
    /// the caret: `⌘D` travels through a focused input, and a click in a menu
    /// blurs nothing. Pushed afterwards, the typing would sit *on top of* the
    /// command's snapshot, and one `⌘Z` would undo both while the next put the
    /// text back.
    pub(super) fn take_text_step(&mut self) -> bool {
        let Some((_, surface, path, before)) = self.edit_snapshot.take() else {
            return false;
        };
        // Which box was typing decides whether the panel is already up to
        // date. The claim below — "the fields hold what the tree holds" — is
        // true of the inspector's own boxes and of no other: the canvas box
        // shares this session mechanism but is a different entity, so adopting
        // the key after a double-click edit left the inspector showing the old
        // label, and one more character typed there wrote it back over what
        // the canvas had just said.
        let ours = matches!(surface, TextSurface::Inspector);
        let Some(view) = self.view_mut() else {
            return false;
        };
        // The tab may have changed, or the view may have been reloaded, since
        // the field took the focus; that snapshot belongs to neither.
        if view.path != path || view.root == before {
            return false;
        }
        view.past.push(before);
        view.future.clear();
        self.revision += 1;
        // The bump above is for the code panel, which is keyed on the revision;
        // the inspector's fields already hold what the tree holds, the caret
        // having put it there. So the new key is adopted rather than left for
        // `sync_prop_inputs` to answer — otherwise the next frame rebuilds the
        // box the caret has just moved into, which is the one thing the whole
        // mechanism exists to avoid.
        //
        // Only for a box of the inspector's own, though: when the writing came
        // from elsewhere the panel has nothing showing the new value, and the
        // key is left for the next frame to answer.
        if ours {
            self.synced = self.view().map(|view| (self.revision, view.selected.clone()));
        }
        self.sync_image_size();
        true
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
        // The same detour as `edit_prop`: a list of entries is typed into a
        // text field, and what it feeds is the constructor in `new`.
        if matches!(prop.target, crate::registry::Target::Initializer(_)) {
            self.edit_initializer(prop, value, cx);
            return;
        }
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
        // file. Cleared here, and read again by the edit closing, which is why
        // that read is a step of its own rather than the rebuild's.
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
        // Not a call on the node but an argument in the view's `new`.
        if matches!(prop.target, registry::Target::Initializer(_)) {
            self.edit_initializer(prop, value, cx);
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
