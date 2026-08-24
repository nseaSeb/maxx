//! The workspace: the root view of every window, and the logic that decides
//! which window a freshly opened folder lands in.

use gpui::prelude::*;
use gpui::{
    AnyElement, App, Bounds, Context, Entity, Global, SharedString, TitlebarOptions, WeakEntity,
    Window, WindowBounds, WindowId, WindowOptions, div, point, px, rgb, size, uniform_list,
};
use gpui::{ScrollHandle, ScrollStrategy, Task, UniformListScrollHandle};
use gpui_component::Root;
use gpui_component::input::{InputEvent, InputState};
use gpui_component::scroll::Scrollbar;
use gpui_component::spinner::Spinner;
use gpui_component::Sizable as _;
use std::collections::HashMap;

/// Which workspace lives in which window.
///
/// The window's root view is `gpui_component::Root` — several components walk
/// up to it and panic if it is anything else — so the workspace can no longer
/// be reached by downcasting the window handle.
#[derive(Default)]
struct Workspaces(HashMap<WindowId, WeakEntity<Workspace>>);

impl Global for Workspaces {}

use crate::actions::OpenFolder;
use crate::model::{Node, Path as NodePath};
use crate::project::{Entry, Project, flatten};
use crate::registry;
use crate::registry::Kind;
use crate::theme;
use crate::menu_model::ItemDef;
use crate::menufile::{MenuFile, Selection};
use crate::view::View;
use std::collections::HashSet;
use std::path::PathBuf;

/// Which box of the menu panel a value belongs to.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum MenuField {
    /// The title of a menu.
    Name,
    /// The label of an entry.
    Label,
    /// The action an entry dispatches.
    Action,
}

/// Whether a name can be a Rust type, which is what an action is.
fn is_type_name(value: &str) -> bool {
    let mut chars = value.chars();
    matches!(chars.next(), Some(first) if first.is_alphabetic() || first == '_')
        && chars.all(|c| c.is_alphanumeric() || c == '_')
}

/// Walks the selection back to the nearest node that still exists.
///
/// Undoing used to drop the selection to the root, which loses your place for
/// no reason when the node is still there.
fn clamp_selection(view: &mut View) {
    while !view.selected.is_empty() && view.root.at(&view.selected).is_none() {
        view.selected.pop();
    }
}

/// The workspace of the frontmost window, if there is one.
fn active_workspace(cx: &App) -> Option<Entity<Workspace>> {
    let handle = cx.active_window()?;
    cx.try_global::<Workspaces>()?
        .0
        .get(&handle.window_id())?
        .upgrade()
}

/// Runs `f` against the workspace of the frontmost window.
///
/// Only safe outside a window update — from an async task, typically. An
/// action handler is dispatched *during* one, and GPUI refuses to re-enter it;
/// use [`defer_active`] there.
pub fn with_active<R>(
    cx: &mut App,
    f: impl FnOnce(&mut Workspace, &mut Window, &mut Context<Workspace>) -> R,
) -> Option<R> {
    let handle = cx.active_window()?;
    let workspace = active_workspace(cx)?;
    cx.update_window(handle, |_, window, cx| {
        workspace.update(cx, |workspace, cx| f(workspace, window, cx))
    })
    .ok()
}

/// Runs `f` against the frontmost workspace once the current update is over.
///
/// This is what an action handler must use: it is called from inside the
/// window's own update, where `update_window` fails, and deferring puts the
/// work just after it instead.
pub fn defer_active(
    cx: &mut App,
    f: impl FnOnce(&mut Workspace, &mut Window, &mut Context<Workspace>) + 'static,
) {
    cx.defer(move |cx| {
        with_active(cx, f);
    });
}

/// Reads the frontmost workspace. Safe anywhere: it never updates a window.
pub fn read_active<R>(cx: &App, f: impl FnOnce(&Workspace) -> R) -> Option<R> {
    let workspace = active_workspace(cx)?;
    Some(f(workspace.read(cx)))
}

/// Root view of a window. A workspace without a project shows the welcome
/// screen and can be reused by the next `Open Folder…`.
pub struct Workspace {
    project: Option<Project>,
    entries: Vec<Entry>,
    expanded: HashSet<PathBuf>,
    selected: Option<PathBuf>,
    show_panel: bool,
    show_status_bar: bool,
    /// The project's menu bar, when `src/menus.rs` is the file being edited.
    pub menu_file: Option<MenuFile>,
    /// Text boxes of the menu panel.
    menu_inputs: Vec<(MenuField, Entity<InputState>)>,
    /// Selection the menu boxes were built for.
    menu_synced: Option<Option<Selection>>,
    /// Views open in the workshop, in tab order.
    views: Vec<View>,
    /// Index of the view being designed.
    active: Option<usize>,
    /// Last error shown in the status bar.
    message: Option<SharedString>,
    /// Live text fields of the inspector, one per editable text property of the
    /// selected node.
    prop_inputs: Vec<(&'static crate::registry::Prop, Entity<InputState>)>,
    /// Bumped by every structural edit. Typing in an inspector field does not
    /// bump it, so the field is not rebuilt under the caret.
    revision: u64,
    /// The revision and selection the inspector fields were built for.
    synced: Option<(u64, NodePath)>,
    /// Lines produced by the last `cargo run`, newest last.
    run_output: Vec<SharedString>,
    /// Where that run is in its life.
    run_state: crate::run::State,
    /// Pid of the running process, so it can be stopped.
    run_pid: Option<u32>,
    /// Whether the output panel is shown.
    show_output: bool,
    /// The task draining the runner's channel. Dropping it stops the drain.
    run_task: Option<Task<()>>,
    /// Name box of the state panel.
    state_name_input: Option<Entity<InputState>>,
    /// Index into `view::STATE_TYPES` for the field about to be added.
    state_type: usize,
    /// The tree as it was when the focused inspector field was entered, and
    /// the view it belongs to.
    edit_snapshot: Option<(PathBuf, Node)>,
    /// Views changed both on disk and in the designer, awaiting a decision.
    conflicts: HashSet<PathBuf>,
    /// Whether the window held the focus on the previous frame, to notice the
    /// moment it comes back.
    was_active: bool,
    /// Scroll position of the right-hand panels.
    pub(crate) side_scroll: ScrollHandle,
    /// Scroll position of the output panel.
    pub(crate) output_scroll: UniformListScrollHandle,
}

impl Workspace {
    fn new(project: Option<Project>) -> Self {
        let mut workspace = Self {
            project,
            entries: Vec::new(),
            expanded: HashSet::new(),
            selected: None,
            show_panel: true,
            show_status_bar: true,
            menu_file: None,
            menu_inputs: Vec::new(),
            menu_synced: None,
            views: Vec::new(),
            active: None,
            message: None,
            prop_inputs: Vec::new(),
            revision: 0,
            synced: None,
            run_output: Vec::new(),
            run_state: crate::run::State::Idle,
            run_pid: None,
            show_output: false,
            run_task: None,
            state_name_input: None,
            state_type: 0,
            edit_snapshot: None,
            conflicts: HashSet::new(),
            was_active: false,
            side_scroll: ScrollHandle::new(),
            output_scroll: UniformListScrollHandle::new(),
        };
        workspace.refresh_entries();
        workspace
    }

    /// The view being designed.
    pub fn view(&self) -> Option<&View> {
        self.views.get(self.active?)
    }

    /// The view being designed, mutably.
    pub fn view_mut(&mut self) -> Option<&mut View> {
        self.views.get_mut(self.active?)
    }

    /// Every open view, in tab order.
    pub fn open_views(&self) -> &[View] {
        &self.views
    }

    /// Index of the view being designed.
    pub fn active_index(&self) -> Option<usize> {
        self.active
    }

    /// Brings the view at `index` to the front.
    pub fn activate_view(&mut self, index: usize, cx: &mut Context<Self>) {
        self.edit_snapshot = None;
        if index < self.views.len() {
            self.active = Some(index);
            self.revision += 1;
            self.message = None;
            cx.notify();
        }
    }

    /// Closes the view at `index`. A view with unsaved edits is kept, with a
    /// message, rather than discarded.
    pub fn close_view(&mut self, index: usize, cx: &mut Context<Self>) {
        let Some(view) = self.views.get(index) else {
            return;
        };
        if view.dirty() {
            self.message = Some(SharedString::from(format!(
                "{} n'est pas enregistré — ⌘S avant de fermer",
                view.name()
            )));
            cx.notify();
            return;
        }
        self.views.remove(index);
        self.active = match self.active {
            Some(_) if self.views.is_empty() => None,
            Some(active) if active >= index && active > 0 => Some(active - 1),
            Some(active) => Some(active.min(self.views.len() - 1)),
            None => None,
        };
        self.revision += 1;
        cx.notify();
    }

    /// The project this workspace holds, if any.
    pub fn project(&self) -> Option<&Project> {
        self.project.as_ref()
    }

    /// Loads `path` as this workspace's project, replacing any previous one.
    pub fn set_project(&mut self, path: PathBuf, window: &mut Window, cx: &mut Context<Self>) {
        let project = Project::open(path);
        window.set_window_title(&project.name);
        self.project = Some(project);
        self.expanded.clear();
        self.selected = None;
        self.refresh_entries();
        cx.notify();
    }

    /// Drops the project and returns the window to the welcome screen, so a
    /// later `Open Folder…` can reuse it.
    pub fn close_project(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.view().is_some_and(|view| view.dirty()) {
            self.message = Some(SharedString::from(
                "vue non enregistrée — ⌘S avant de fermer le projet",
            ));
            cx.notify();
            return;
        }
        self.project = None;
        self.views.clear();
        self.active = None;
        
        
        self.entries.clear();
        self.expanded.clear();
        self.selected = None;
        window.set_window_title("maxx");
        cx.notify();
    }

    /// Toggles the project panel (View > Project Panel, `cmd-b`).
    pub fn toggle_project_panel(&mut self, cx: &mut Context<Self>) {
        self.show_panel = !self.show_panel;
        cx.notify();
    }

    /// Toggles the status bar (View > Status Bar).
    pub fn toggle_status_bar(&mut self, cx: &mut Context<Self>) {
        self.show_status_bar = !self.show_status_bar;
        cx.notify();
    }

    fn toggle_expanded(&mut self, path: PathBuf, cx: &mut Context<Self>) {
        if !self.expanded.remove(&path) {
            self.expanded.insert(path);
        }
        self.refresh_entries();
        cx.notify();
    }

    fn select_file(&mut self, path: PathBuf, cx: &mut Context<Self>) {
        self.message = None;
        if MenuFile::is_menu_file(&path) {
            match MenuFile::load(&path) {
                Ok(menus) => {
                    self.menu_file = Some(menus);
                    self.menu_synced = None;
                }
                Err(error) => self.message = Some(SharedString::from(error)),
            }
            self.selected = Some(path);
            cx.notify();
            return;
        }
        self.menu_file = None;
        if path.extension().is_some_and(|extension| extension == "rs") {
            // Already open: just bring its tab forward.
            if let Some(index) = self.views.iter().position(|view| view.path == path) {
                self.active = Some(index);
                self.revision += 1;
            } else {
                match View::load(&path) {
                    Ok(view) => {
                        self.views.push(view);
                        self.active = Some(self.views.len() - 1);
                        self.revision += 1;
                    }
                    Err(error) => self.message = Some(SharedString::from(error)),
                }
            }
        }
        self.selected = Some(path);
        cx.notify();
    }

    /// Selects a node of the tree being designed.
    pub fn select(&mut self, path: NodePath, cx: &mut Context<Self>) {
        if let Some(view) = self.view_mut() {
            view.selected = path;
            cx.notify();
        }
    }

    /// The fields of the view able to back a text input.
    pub(crate) fn input_fields(&self) -> Vec<String> {
        self.view()
            .map(|view| view.input_state_fields())
            .unwrap_or_default()
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

    /// The text box bound to a field of the menu panel.
    pub(crate) fn menu_input(&self, field: MenuField) -> Option<&Entity<InputState>> {
        self.menu_inputs
            .iter()
            .find(|(candidate, _)| *candidate == field)
            .map(|(_, state)| state)
    }

    /// Rebuilds the menu panel's boxes when its selection changes.
    fn sync_menu_inputs(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let key = self.menu_file.as_ref().map(|menus| menus.selected);
        if key == self.menu_synced {
            return;
        }
        self.menu_synced = key;
        self.menu_inputs.clear();

        let Some(menus) = self.menu_file.as_ref() else {
            return;
        };
        let mut fields = Vec::new();
        match menus.selected {
            Some(Selection::Menu(_)) => {
                if let Some(menu) = menus.selected_menu() {
                    fields.push((MenuField::Name, menu.name.clone()));
                }
            }
            Some(Selection::Item(_, _)) => {
                if let Some(ItemDef::Action { label, action, .. }) = menus.selected_item() {
                    fields.push((MenuField::Label, label.clone()));
                    fields.push((MenuField::Action, action.clone()));
                }
            }
            None => {}
        }

        for (field, value) in fields {
            let state = cx.new(|cx| InputState::new(window, cx).default_value(value));
            cx.subscribe(&state, move |this, state, event: &InputEvent, cx| {
                if matches!(event, InputEvent::Change) {
                    let value = state.read(cx).value().to_string();
                    this.edit_menu_field(field, &value, cx);
                }
            })
            .detach();
            self.menu_inputs.push((field, state));
        }
    }

    /// Writes one field of the menu panel.
    fn edit_menu_field(&mut self, field: MenuField, value: &str, cx: &mut Context<Self>) {
        let Some(menus) = self.menu_file.as_mut() else {
            return;
        };
        let Some(selection) = menus.selected else {
            return;
        };
        match (selection, field) {
            (Selection::Menu(index), MenuField::Name) => {
                if let Some(menu) = menus.menus.get_mut(index) {
                    menu.name = value.to_string();
                }
            }
            (Selection::Item(menu, item), _) => {
                if let Some(ItemDef::Action { label, action, .. }) = menus
                    .menus
                    .get_mut(menu)
                    .and_then(|menu| menu.items.get_mut(item))
                {
                    match field {
                        MenuField::Label => *label = value.to_string(),
                        // An action name is a Rust type: refuse what would not
                        // compile rather than write it.
                        MenuField::Action if is_type_name(value) => *action = value.to_string(),
                        _ => {}
                    }
                }
            }
            _ => {}
        }
        cx.notify();
    }

    /// Selects a menu or one of its entries.
    pub fn select_menu(&mut self, selection: Selection, cx: &mut Context<Self>) {
        if let Some(menus) = self.menu_file.as_mut() {
            menus.selected = Some(selection);
            cx.notify();
        }
    }

    /// Adds a menu to the bar.
    pub fn add_menu(&mut self, cx: &mut Context<Self>) {
        if let Some(menus) = self.menu_file.as_mut() {
            menus.add_menu();
            cx.notify();
        }
    }

    /// Adds an entry to the selected menu.
    pub fn add_menu_item(&mut self, separator: bool, cx: &mut Context<Self>) {
        let Some(menus) = self.menu_file.as_mut() else {
            return;
        };
        if menus.selected.is_none() {
            self.message = Some(SharedString::from("sélectionnez d'abord un menu"));
            cx.notify();
            return;
        }
        let item = if separator {
            ItemDef::Separator
        } else {
            ItemDef::Action {
                label: "Entrée".into(),
                action: "MonAction".into(),
                os_action: None,
            }
        };
        menus.add_item(item);
        cx.notify();
    }

    /// Removes the selected menu or entry.
    pub fn remove_menu_selection(&mut self, cx: &mut Context<Self>) {
        if let Some(menus) = self.menu_file.as_mut() {
            menus.remove_selected();
            cx.notify();
        }
    }

    /// Rebuilds the inspector's text fields when the selection or the tree has
    /// changed. Called once per frame from `render`, which is the only place
    /// holding both `&mut self` and a `Window`.
    fn sync_prop_inputs(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let key = self
            .view()
            .map(|view| (self.revision, view.selected.clone()));
        if key == self.synced {
            return;
        }
        if self.state_name_input.is_none() {
            self.state_name_input =
                Some(cx.new(|cx| InputState::new(window, cx).placeholder("nom du champ")));
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

        for prop in crate::registry::props(spec) {
            if !matches!(
                prop.kind,
                Kind::Text | Kind::Field | Kind::Handler | Kind::Number | Kind::Color
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
                    this.edit_snapshot = this
                        .view()
                        .map(|view| (view.path.clone(), view.root.clone()));
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
        let bound = view
            .root
            .at(&selected)
            .and_then(|node| registry::read_binding(node, prop));
        let fields = view.state_fields();

        let expression = match bound {
            Some(_) => None,
            None => match fields.first() {
                Some(field) => Some(field.read_expression()),
                None => {
                    self.message = Some(SharedString::from(
                        "aucun champ d'état — ajoutez-en un dans « État »",
                    ));
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
        let current = view
            .root
            .at(&selected)
            .and_then(|node| registry::read_binding(node, prop));
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
            self.message = Some(SharedString::from("donnez un nom au champ"));
            cx.notify();
            return;
        }
        let (_, ty, initial) = crate::view::STATE_TYPES[self.state_type];

        self.message = match self.view_mut() {
            Some(view) => match view.add_state_field(&name, ty, initial) {
                Ok(()) => Some(SharedString::from(format!("champ « {name} » ajouté"))),
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

    /// Which kind of field the state panel will add.
    pub(crate) fn state_type(&self) -> usize {
        self.state_type
    }

    /// Opens the handler of a property in Zed, on its own line.
    pub fn open_handler(&mut self, prop: &'static crate::registry::Prop, cx: &mut Context<Self>) {
        let Some(view) = self.view() else {
            return;
        };
        let node = view.selected();
        let Some(name) = registry::read(node, prop).filter(|name| !name.is_empty()) else {
            self.message = Some(SharedString::from("aucune action sur ce nœud"));
            cx.notify();
            return;
        };
        match view.method_line(&name) {
            Some(line) => crate::run::open_editor_at(&view.path, line),
            None => {
                self.message = Some(SharedString::from(format!(
                    "« {name} » n'est pas encore écrite — ⌘S l'ajoute au fichier"
                )));
                cx.notify();
            }
        }
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
        let Some(view) = self.view() else {
            return;
        };
        let fields = view.input_state_fields();
        if fields.is_empty() {
            return;
        }
        let selected = view.selected.clone();
        let current = view
            .root
            .at(&selected)
            .and_then(|node| registry::read(node, prop));
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
    fn edit_prop_text(&mut self, prop: &'static crate::registry::Prop, value: &str, cx: &mut Context<Self>) {
        let Some(view) = self.view_mut() else {
            return;
        };
        let selected = view.selected.clone();
        if let Some(node) = view.root.at_mut(&selected) {
            let current = crate::registry::read(node, prop);
            if current.as_deref() == Some(value) {
                return;
            }
            crate::registry::write(node, prop, value);
        }
        self.message = crate::registry::validate(prop, value).map(SharedString::from);
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

    /// Writes a property of the selected node.
    pub fn edit_prop(&mut self, prop: &'static crate::registry::Prop, value: &str, cx: &mut Context<Self>) {
        let Some(view) = self.view_mut() else {
            return;
        };
        let selected = view.selected.clone();
        if view.root.at(&selected).is_some() {
            self.checkpoint();
            let view = self.view_mut().expect("just borrowed");
            if let Some(node) = view.root.at_mut(&selected) {
                registry::write(node, prop, value);
            }
        }
        cx.notify();
    }

    /// Inserts a component into the selected container, or beside the selected
    /// node when it cannot hold children.
    pub fn insert_component(&mut self, id: &str, cx: &mut Context<Self>) {
        let Some(mut node) = registry::instantiate(id) else {
            return;
        };
        let Some(view) = self.view() else {
            return;
        };

        // Two inputs sharing `&self.champ` compile but mirror each other at
        // runtime, so each one gets its own field.
        if id == "input" {
            let field = registry::unique_input_field(&view.root);
            if let crate::model::Base::Known { args, .. } = &mut node.base {
                *args = vec![crate::model::Arg::Verbatim(format!("&self.{field}"))];
            }
        }

        let selected = view.selected.clone();
        let accepts_children = view
            .root
            .at(&selected)
            .and_then(registry::of)
            .is_some_and(|spec| spec.container);
        let child_count = view
            .root
            .at(&selected)
            .map(|node| node.children.len())
            .unwrap_or(0);

        let destination = if accepts_children {
            let mut path = selected.clone();
            path.push(child_count);
            path
        } else if selected.is_empty() {
            // The root is not a container: there is nowhere to put it.
            self.message = Some(SharedString::from(
                "la racine n'accepte pas d'enfant — sélectionnez une colonne ou une ligne",
            ));
            cx.notify();
            return;
        } else {
            let mut path = selected.clone();
            let last = path.last_mut().expect("not empty");
            *last += 1;
            path
        };

        self.checkpoint();
        let view = self.view_mut().expect("just borrowed");
        if view.root.insert(&destination, node) {
            view.selected = destination;
        }
        cx.notify();
    }

    /// Builds the dependency tree in the background, so the first run does not
    /// have to.
    pub fn prewarm_project(&mut self, cx: &mut Context<Self>) {
        self.start_cargo(true, cx);
    }

    /// Runs `cargo run` on the open project and streams its output into the
    /// bottom panel.
    pub fn run_project(&mut self, cx: &mut Context<Self>) {
        self.start_cargo(false, cx);
    }

    fn start_cargo(&mut self, prewarm: bool, cx: &mut Context<Self>) {
        let Some(project) = self.project.as_ref() else {
            return;
        };
        if self.run_state == crate::run::State::Running {
            self.message = Some(SharedString::from(
                "une exécution est déjà en cours — ⌘. pour l'arrêter",
            ));
            cx.notify();
            return;
        }

        let root = project.root.clone();
        self.run_output.clear();
        self.run_state = crate::run::State::Running;
        self.run_pid = None;
        self.show_output = true;
        self.message = None;

        let receiver = if prewarm {
            crate::run::prewarm(root)
        } else {
            crate::run::start(root)
        };
        self.run_task = Some(cx.spawn(async move |workspace, cx| {
            loop {
                let mut lines = Vec::new();
                let mut pid = None;
                let mut finished = None;
                while let Ok(message) = receiver.try_recv() {
                    match message {
                        crate::run::Message::Started(id) => pid = Some(id),
                        crate::run::Message::Line(line) => lines.push(SharedString::from(line)),
                        crate::run::Message::Finished(ok) => finished = Some(ok),
                    }
                }

                if !lines.is_empty() || pid.is_some() || finished.is_some() {
                    let updated = workspace.update(cx, |workspace, cx| {
                        workspace.run_output.extend(lines);
                        // The panel is a log, not a buffer: an application left
                        // running for an hour must not grow the process.
                        let overflow = workspace.run_output.len().saturating_sub(500);
                        workspace.run_output.drain(..overflow);
                        if let Some(pid) = pid {
                            workspace.run_pid = Some(pid);
                        }
                        if let Some(ok) = finished {
                            workspace.run_state = crate::run::State::Finished { ok };
                            workspace.run_pid = None;
                        }
                        // Follow the tail, the way a terminal does.
                        if let Some(last) = workspace.run_output.len().checked_sub(1) {
                            workspace
                                .output_scroll
                                .scroll_to_item(last, ScrollStrategy::Top);
                        }
                        cx.notify();
                    });
                    if updated.is_err() {
                        return;
                    }
                }

                if finished.is_some() {
                    return;
                }
                cx.background_executor()
                    .timer(std::time::Duration::from_millis(80))
                    .await;
            }
        }));
        cx.notify();
    }

    /// Stops the running process.
    pub fn stop_project(&mut self, cx: &mut Context<Self>) {
        let Some(pid) = self.run_pid.take() else {
            return;
        };
        crate::run::stop(pid);
        self.message = Some(SharedString::from("exécution interrompue"));
        cx.notify();
    }

    /// Shows or hides the output panel.
    pub fn toggle_output(&mut self, cx: &mut Context<Self>) {
        self.show_output = !self.show_output;
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
                    self.message = Some(SharedString::from(
                        "un nœud ne peut pas être déposé dans lui-même",
                    ));
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
        let Some(prop) = spec
            .props
            .iter()
            .find(|prop| matches!(prop.kind, crate::registry::Kind::Handler))
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
        self.message = Some(SharedString::from(format!(
            "action « {name} » — ⌘S écrit la méthode dans le fichier"
        )));
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

    /// Adds a view to the open project and opens it.
    ///
    /// The name is generated rather than asked for: a modal text prompt lands
    /// with the editor, and `vue_2` is renamable in Zed in two seconds.
    pub fn new_view(&mut self, cx: &mut Context<Self>) {
        let Some(project) = self.project.as_ref() else {
            return;
        };
        let root = project.root.clone();

        let mut index = 1;
        let module = loop {
            let candidate = format!("vue_{index}");
            if !root.join(format!("src/ui/{candidate}.rs")).exists() {
                break candidate;
            }
            index += 1;
        };

        match crate::scaffold::create_view(&root, &module) {
            Ok(()) => {
                self.refresh_entries();
                let path = root.join(format!("src/ui/{module}.rs"));
                self.select_file(path, cx);
                // `select_file` reports its own failure; only claim success when
                // it stayed quiet.
                if self.message.is_none() {
                    self.message = Some(SharedString::from(format!("{module}.rs créé")));
                }
            }
            Err(error) => self.message = Some(SharedString::from(error.to_string())),
        }
        cx.notify();
    }

    /// Writes the view back to its file, refusing when the file changed
    /// underneath.
    pub fn save_view(&mut self, cx: &mut Context<Self>) {
        self.write_view(false, cx);
    }

    /// Writes the view even though the file changed on disk, losing what was
    /// written there.
    pub fn overwrite_view(&mut self, cx: &mut Context<Self>) {
        self.write_view(true, cx);
    }

    fn write_view(&mut self, force: bool, cx: &mut Context<Self>) {
        if let Some(menus) = self.menu_file.as_mut() {
            self.message = match menus.save() {
                Ok(()) => Some(SharedString::from(format!("{} enregistré", menus.name()))),
                Err(error) => Some(SharedString::from(error)),
            };
            cx.notify();
            return;
        }
        let Some(view) = self.view() else {
            return;
        };
        let path = view.path.clone();

        if !force && view.disk_changed() {
            if !view.dirty() {
                // Nothing to lose on this side: take what is on disk.
                self.reload_view(cx);
                return;
            }
            self.conflicts.insert(path);
            self.message = Some(SharedString::from(
                "fichier modifié en dehors de maxx — Fichier > Recharger, ou Écraser",
            ));
            cx.notify();
            return;
        }

        let view = self.view_mut().expect("just borrowed");
        self.message = match view.save() {
            Ok(()) => Some(SharedString::from(format!("{} enregistré", view.name()))),
            Err(error) => Some(SharedString::from(error)),
        };
        self.conflicts.remove(&path);
        self.revision += 1;
        cx.notify();
    }

    /// Drops what the designer holds and re-reads the file.
    pub fn reload_view(&mut self, cx: &mut Context<Self>) {
        self.edit_snapshot = None;
        if let Some(menus) = self.menu_file.as_mut() {
            self.message = match menus.reload() {
                Ok(()) => Some(SharedString::from("menus rechargés")),
                Err(error) => Some(SharedString::from(error)),
            };
            self.menu_synced = None;
            cx.notify();
            return;
        }
        let Some(view) = self.view_mut() else {
            return;
        };
        let path = view.path.clone();
        let name = view.name();
        self.message = match view.reload() {
            Ok(()) => Some(SharedString::from(format!("{name} rechargé"))),
            Err(error) => Some(SharedString::from(error)),
        };
        self.conflicts.remove(&path);
        self.revision += 1;
        cx.notify();
    }

    /// Notices files changed outside maxx.
    ///
    /// A view the designer has not touched is reloaded without asking — the
    /// habit every editor gives you for an unmodified buffer. One changed on
    /// both sides is a real conflict and waits for a decision.
    fn check_disk(&mut self, cx: &mut Context<Self>) {
        let mut reloaded = Vec::new();
        let mut conflicted = Vec::new();

        for index in 0..self.views.len() {
            let view = &self.views[index];
            if !view.disk_changed() {
                continue;
            }
            if view.dirty() {
                conflicted.push(view.path.clone());
            } else {
                reloaded.push(index);
            }
        }

        // Only a view that actually moved invalidates the snapshot; clearing it
        // on every return to the window would swallow the undo step for a text
        // edit interrupted by an alt-tab.
        if !reloaded.is_empty() || !conflicted.is_empty() {
            self.edit_snapshot = None;
        }

        for index in reloaded {
            let view = &mut self.views[index];
            let name = view.name();
            if view.reload().is_ok() {
                self.message = Some(SharedString::from(format!(
                    "{name} rechargé — modifié en dehors de maxx"
                )));
                self.revision += 1;
            }
        }
        for path in conflicted {
            if self.conflicts.insert(path) {
                self.message = Some(SharedString::from(
                    "modifié des deux côtés — Fichier > Recharger, ou Écraser",
                ));
                self.revision += 1;
            }
        }
        cx.notify();
    }

    /// Puts maxx's markers around the expression a hand-written `render`
    /// returns, then opens the view.
    pub fn adopt_view(&mut self, cx: &mut Context<Self>) {
        let Some(path) = self
            .selected
            .clone()
            .filter(|path| path.extension().is_some_and(|extension| extension == "rs"))
        else {
            self.message = Some(SharedString::from(
                "sélectionnez un fichier .rs dans l'explorateur",
            ));
            cx.notify();
            return;
        };

        let source = match std::fs::read_to_string(&path) {
            Ok(source) => source,
            Err(error) => {
                self.message = Some(SharedString::from(error.to_string()));
                cx.notify();
                return;
            }
        };

        match crate::parser::adopt(&source) {
            Ok(adopted) => match std::fs::write(&path, &adopted) {
                Ok(()) => {
                    self.message = None;
                    self.select_file(path, cx);
                    if self.message.is_none() {
                        self.message = Some(SharedString::from("vue adoptée"));
                    }
                }
                Err(error) => self.message = Some(SharedString::from(error.to_string())),
            },
            Err(error) => self.message = Some(SharedString::from(error.to_string())),
        }
        cx.notify();
    }

    /// Steps back one edit.
    pub fn undo(&mut self, cx: &mut Context<Self>) {
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

    fn refresh_entries(&mut self) {
        self.entries = match &self.project {
            Some(project) => flatten(&project.root, &self.expanded),
            None => Vec::new(),
        };
    }

    fn render_titlebar(&self) -> impl IntoElement {
        let (title, subtitle) = match &self.project {
            Some(project) => (
                project.name.clone(),
                SharedString::from(project.root.to_string_lossy().into_owned()),
            ),
            None => (SharedString::from("maxx"), SharedString::from("")),
        };

        div()
            .flex()
            .items_center()
            .h(px(32.))
            // The system titlebar is transparent, so the traffic lights are
            // drawn on top of this row: keep their corner clear.
            .pl(px(80.))
            .pr(px(12.))
            .gap_2()
            .bg(rgb(theme::TITLEBAR_BG))
            .border_b_1()
            .border_color(rgb(theme::BORDER))
            .child(
                div()
                    .flex()
                    .flex_1()
                    .items_center()
                    .justify_center()
                    .gap_2()
                    .child(div().child(title))
                    .child(
                        div()
                            .text_xs()
                            .text_color(rgb(theme::TEXT_MUTED))
                            .child(subtitle),
                    ),
            )
    }

    fn render_project_panel(&self, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .flex()
            .flex_col()
            .w(px(240.))
            .flex_none()
            .bg(rgb(theme::PANEL_BG))
            .border_r_1()
            .border_color(rgb(theme::BORDER))
            .child(
                div()
                    .px_3()
                    .py_2()
                    .text_xs()
                    .text_color(rgb(theme::TEXT_MUTED))
                    .child("EXPLORATEUR"),
            )
            .child(
                uniform_list(
                    "project-entries",
                    self.entries.len(),
                    cx.processor(|this, range: std::ops::Range<usize>, _window, cx| {
                        range
                            .filter_map(|ix| this.entries.get(ix).cloned())
                            .map(|entry| this.render_entry(entry, cx))
                            .collect::<Vec<_>>()
                    }),
                )
                .flex_1(),
            )
    }

    fn render_entry(&self, entry: Entry, cx: &mut Context<Self>) -> AnyElement {
        let is_selected = self.selected.as_deref() == Some(entry.path.as_path());
        let is_expanded = self.expanded.contains(&entry.path);
        let marker = if entry.is_dir {
            if is_expanded { "▾" } else { "▸" }
        } else {
            " "
        };
        let path = entry.path.clone();
        let is_dir = entry.is_dir;

        div()
            .id(SharedString::from(entry.path.to_string_lossy().into_owned()))
            .flex()
            .items_center()
            .gap_1()
            .h(px(22.))
            .pr_2()
            .pl(px(8. + 12. * entry.depth as f32))
            .cursor_pointer()
            .when(is_selected, |this| this.bg(rgb(theme::SELECTED_BG)))
            .hover(|this| this.bg(rgb(theme::HOVER_BG)))
            .child(
                div()
                    .w(px(12.))
                    .flex_none()
                    .text_xs()
                    .text_color(rgb(theme::TEXT_MUTED))
                    .child(marker),
            )
            .child(
                div()
                    .when(is_dir, |this| this.text_color(rgb(theme::ACCENT)))
                    .child(entry.name.clone()),
            )
            .on_click(cx.listener(move |this, _, _window, cx| {
                if is_dir {
                    this.toggle_expanded(path.clone(), cx);
                } else {
                    this.select_file(path.clone(), cx);
                }
            }))
            .into_any_element()
    }

    fn render_main(&self, cx: &mut Context<Self>) -> AnyElement {
        if self.project.is_none() {
            return self.render_welcome(cx);
        }
        self.render_designer(cx)
    }

    fn render_welcome(&self, cx: &mut Context<Self>) -> AnyElement {
        div()
            .flex()
            .flex_1()
            .flex_col()
            .items_center()
            .justify_center()
            .gap_4()
            .child(div().text_2xl().child("maxx"))
            .child(
                div()
                    .text_color(rgb(theme::TEXT_MUTED))
                    .child("Ouvrez un dossier pour commencer."),
            )
            .child(
                div()
                    .id("welcome-open-folder")
                    .px_4()
                    .py_2()
                    .rounded_md()
                    .cursor_pointer()
                    .bg(rgb(theme::ACCENT))
                    .text_color(rgb(theme::ON_ACCENT))
                    .hover(|this| this.opacity(0.85))
                    .child("Ouvrir un dossier…")
                    .on_click(cx.listener(|_, _, window, cx| {
                        window.dispatch_action(Box::new(OpenFolder), cx);
                    })),
            )
            .child(
                div()
                    .text_xs()
                    .text_color(rgb(theme::TEXT_MUTED))
                    .child("⌘O"),
            )
            .into_any_element()
    }

    /// The output of the last run.
    fn render_output(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let (label, colour) = match self.run_state {
            crate::run::State::Idle => ("prêt", theme::TEXT_MUTED),
            crate::run::State::Running => ("exécution…", theme::ACCENT),
            crate::run::State::Finished { ok: true } => ("terminé", theme::TEXT_MUTED),
            crate::run::State::Finished { ok: false } => ("échec", 0xe06c75),
        };
        let lines = self.run_output.clone();
        // What cargo is doing right now is more useful than a bar with no total.
        let current = self
            .run_output
            .iter()
            .rev()
            .find(|line| !line.trim().is_empty())
            .cloned()
            .unwrap_or_default();

        div()
            .flex()
            .flex_col()
            .h(px(200.))
            .flex_none()
            .bg(rgb(theme::PANEL_BG))
            .border_t_1()
            .border_color(rgb(theme::BORDER))
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .px_3()
                    .py_1()
                    .text_xs()
                    .when(self.run_state == crate::run::State::Running, |this| {
                        this.child(Spinner::new().small())
                    })
                    .child(div().text_color(rgb(colour)).child(label))
                    .child(
                        div()
                            .flex_1()
                            .overflow_hidden()
                            .text_color(rgb(theme::TEXT_MUTED))
                            .child(current),
                    )
                    .when(self.run_state == crate::run::State::Running, |this| {
                        this.child(
                            div()
                                .id("run-stop")
                                .px_2()
                                .rounded_sm()
                                .cursor_pointer()
                                .hover(|this| this.bg(rgb(theme::HOVER_BG)))
                                .child("Arrêter")
                                .on_click(cx.listener(|this, _, _, cx| this.stop_project(cx))),
                        )
                    })
                    .child(
                        div()
                            .id("run-close")
                            .px_2()
                            .rounded_sm()
                            .cursor_pointer()
                            .hover(|this| this.bg(rgb(theme::HOVER_BG)))
                            .child("Fermer")
                            .on_click(cx.listener(|this, _, _, cx| this.toggle_output(cx))),
                    ),
            )
            .child(
                div()
                    .relative()
                    .flex_1()
                    .overflow_hidden()
                    .child(uniform_list(
                    "run-output",
                    lines.len(),
                    cx.processor(move |this, range: std::ops::Range<usize>, _window, _cx| {
                        range
                            .filter_map(|index| this.run_output.get(index).cloned())
                            .map(|line| {
                                div()
                                    .px_3()
                                    .text_xs()
                                    .font_family("Menlo")
                                    .text_color(rgb(if line.contains("error") {
                                        0xe06c75
                                    } else if line.contains("warning") {
                                        0xe5c07b
                                    } else {
                                        theme::TEXT_MUTED
                                    }))
                                    .child(line)
                            })
                            .collect::<Vec<_>>()
                    }),
                    )
                    .track_scroll(self.output_scroll.clone())
                    .size_full())
                    .child(
                        div()
                            .absolute()
                            .top_0()
                            .right_0()
                            .bottom_0()
                            .child(Scrollbar::vertical(&self.output_scroll)),
                    ),
            )
    }

    fn render_status_bar(&self) -> impl IntoElement {
        let conflict = self
            .view()
            .is_some_and(|view| self.conflicts.contains(&view.path));
        if self.menu_file.is_some() {
            let menus = self.menu_file.as_ref().expect("just checked");
            let label = match &self.message {
                Some(message) => message.clone(),
                None => SharedString::from(format!(
                    "{}{} · {} menus",
                    menus.name(),
                    if menus.dirty() { " •" } else { "" },
                    menus.menus.len()
                )),
            };
            return div()
                .flex()
                .items_center()
                .h(px(24.))
                .px_3()
                .flex_none()
                .bg(rgb(theme::PANEL_BG))
                .border_t_1()
                .border_color(rgb(theme::BORDER))
                .text_xs()
                .text_color(rgb(theme::TEXT_MUTED))
                .child(label);
        }
        let label = match (&self.message, &self.view(), &self.project) {
            (Some(message), _, _) => message.clone(),
            (None, Some(view), _) => SharedString::from(format!(
                "{}{}{} · {} nœuds",
                view.name(),
                if view.dirty() { " •" } else { "" },
                if conflict { " ⚠ modifié en dehors de maxx" } else { "" },
                view.root.count()
            )),
            (None, None, Some(project)) => SharedString::from(format!(
                "{} · {} éléments",
                project.name,
                self.entries.len()
            )),
            (None, None, None) => SharedString::from("Aucun projet"),
        };

        div()
            .flex()
            .items_center()
            .h(px(24.))
            .px_3()
            .flex_none()
            .bg(rgb(theme::PANEL_BG))
            .border_t_1()
            .border_color(rgb(theme::BORDER))
            .text_xs()
            .text_color(rgb(theme::TEXT_MUTED))
            .child(label)
    }
}

impl Render for Workspace {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        // Coming back from Zed is the moment to notice what changed there.
        let active = window.is_window_active();
        if active && !self.was_active {
            self.check_disk(cx);
        }
        self.was_active = active;

        self.sync_prop_inputs(window, cx);
        self.sync_menu_inputs(window, cx);
        let show_panel = self.show_panel && self.project.is_some();

        div()
            .flex()
            .flex_col()
            .size_full()
            .bg(rgb(theme::BG))
            .text_color(rgb(theme::TEXT))
            .text_sm()
            .child(self.render_titlebar())
            .child(
                div()
                    .flex()
                    .flex_1()
                    .overflow_hidden()
                    .when(show_panel, |this| this.child(self.render_project_panel(cx)))
                    .child(self.render_main(cx)),
            )
            .when(self.show_output, |this| this.child(self.render_output(cx)))
            .when(self.show_status_bar, |this| {
                this.child(self.render_status_bar())
            })
    }
}

/// Opens `path` as a project, reusing the frontmost window when it has no
/// project yet — the same behaviour as Zed.
pub fn open_folder(path: PathBuf, cx: &mut App) {
    let reuse_path = path.clone();
    let reused = with_active(cx, move |workspace, window, cx| {
        if workspace.project.is_some() {
            return false;
        }
        workspace.set_project(reuse_path, window, cx);
        window.activate_window();
        true
    })
    .unwrap_or(false);

    if reused {
        cx.add_recent_document(&path);
        return;
    }

    open_workspace_window(Some(path), cx);
}

/// Opens a new window, either on `path` or on the welcome screen.
pub fn open_workspace_window(path: Option<PathBuf>, cx: &mut App) {
    let project = path.clone().map(Project::open);
    let title = project
        .as_ref()
        .map(|project| project.name.clone())
        .unwrap_or_else(|| "maxx".into());

    let options = WindowOptions {
        window_bounds: Some(WindowBounds::Windowed(Bounds::centered(
            None,
            size(px(1100.), px(720.)),
            cx,
        ))),
        titlebar: Some(TitlebarOptions {
            title: Some(title),
            appears_transparent: true,
            traffic_light_position: Some(point(px(9.), px(9.))),
        }),
        window_min_size: Some(size(px(640.), px(420.))),
        app_id: Some("dev.maxx.Maxx".into()),
        ..Default::default()
    };

    let created: std::rc::Rc<std::cell::RefCell<Option<Entity<Workspace>>>> = Default::default();
    let slot = created.clone();
    let opened = cx.open_window(options, move |window, cx| {
        let workspace = cx.new(|_| Workspace::new(project));
        *slot.borrow_mut() = Some(workspace.clone());
        cx.new(|cx| Root::new(workspace, window, cx))
    });

    let Ok(handle) = opened else {
        return;
    };
    if let Some(workspace) = created.borrow().as_ref() {
        cx.default_global::<Workspaces>()
            .0
            .insert(handle.window_id(), workspace.downgrade());
    }
    if let Some(path) = path {
        cx.add_recent_document(&path);
    }
}
