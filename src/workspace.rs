//! The workspace: the root view of every window, and the logic that decides
//! which window a freshly opened folder lands in.

use gpui::prelude::*;
use gpui::{
    AnyElement, App, Bounds, Context, Entity, Global, SharedString, TitlebarOptions, WeakEntity,
    Window, WindowBounds, WindowId, WindowOptions, div, point, px, rgb, size, uniform_list,
};
use gpui::{ScrollHandle, ScrollStrategy, Task, UniformListScrollHandle};
use gpui_component::Root;
use gpui_component::Sizable as _;
use gpui_component::input::{InputEvent, InputState};
use gpui_component::menu::ContextMenuExt as _;
use gpui_component::resizable::{ResizableState, h_resizable, resizable_panel};
use gpui_component::scroll::Scrollbar;
use gpui_component::spinner::Spinner;
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
use crate::menu_model::ItemDef;
use crate::menufile::{MenuFile, Selection};
use crate::model::{Node, Path as NodePath};
use crate::project::{Entry, Project, flatten};
use crate::registry;
use crate::registry::Kind;
use crate::theme;
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
    cx.try_global::<Workspaces>()?.0.get(&handle.window_id())?.upgrade()
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
    /// The project's menu bar, when `src/menus.rs` is the file being edited.
    pub menu_file: Option<MenuFile>,
    /// Whether the preferences screen has taken over the main area.
    pub preferences: bool,
    /// Where the split between the project panel and the rest sits.
    pub(crate) panel_split: Entity<ResizableState>,
    /// Where the split between the canvas and the inspector sits.
    pub(crate) inspector_split: Entity<ResizableState>,
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
    fn new(project: Option<Project>, cx: &mut Context<Self>) -> Self {
        // A project handed straight to a fresh window never passes through
        // `set_project`, so the notice has to be raised here too.
        let outdated = project
            .as_ref()
            .map(|project| crate::scaffold::outdated_modules(&project.root))
            .unwrap_or_default();

        let mut workspace = Self {
            project,
            entries: Vec::new(),
            expanded: HashSet::new(),
            selected: None,
            menu_file: None,
            preferences: false,
            panel_split: cx.new(|_| ResizableState::default()),
            inspector_split: cx.new(|_| ResizableState::default()),
            menu_inputs: Vec::new(),
            menu_synced: None,
            views: Vec::new(),
            active: None,
            message: (!outdated.is_empty()).then(|| {
                SharedString::from(format!(
                    "{} a une version plus récente — Fichier ▸ Ajouter au projet ▸ Mettre à jour",
                    outdated.join(", ")
                ))
            }),
            prop_inputs: Vec::new(),
            revision: 0,
            synced: None,
            run_output: Vec::new(),
            run_state: crate::run::State::Idle,
            run_pid: None,
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
        if self.discard_menu_edits(cx) {
            return;
        }
        // The menu editor and the preferences are modes: clicking a tab has
        // to leave them, or the tab strip stays without effect.
        self.menu_file = None;
        self.preferences = false;
        if index < self.views.len() {
            self.active = Some(index);
            self.selected = Some(self.views[index].path.clone());
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
        remember_project(&path, cx);
        self.announce_outdated_modules(&path);
        self.menu_file = None;
        self.preferences = false;
        self.menu_synced = None;
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
            self.message =
                Some(SharedString::from("vue non enregistrée — ⌘S avant de fermer le projet"));
            cx.notify();
            return;
        }
        self.project = None;
        self.views.clear();
        self.active = None;
        self.menu_file = None;
        self.preferences = false;
        self.menu_synced = None;
        self.entries.clear();
        self.expanded.clear();
        self.selected = None;
        window.set_window_title("maxx");
        cx.notify();
    }

    /// Toggles the project panel (View > Project Panel, `cmd-b`).
    pub fn toggle_project_panel(&mut self, cx: &mut Context<Self>) {
        crate::settings::update_prefs(cx, |preferences| {
            preferences.show_project_panel = !preferences.show_project_panel;
        });
        notify_all(cx);
    }

    /// Toggles the status bar (View > Status Bar).
    pub fn toggle_status_bar(&mut self, cx: &mut Context<Self>) {
        crate::settings::update_prefs(cx, |preferences| {
            preferences.show_status_bar = !preferences.show_status_bar;
        });
        notify_all(cx);
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
        // Opening anything leaves the preferences, for the same reason.
        self.preferences = false;
        if MenuFile::is_menu_file(&path) {
            // Already open and edited: reloading would drop those edits.
            if self.menu_file.as_ref().is_some_and(|menus| menus.path == path) {
                self.selected = Some(path);
                cx.notify();
                return;
            }
            if self.discard_menu_edits(cx) {
                return;
            }
            self.menu_file = None;
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
        if self.discard_menu_edits(cx) {
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

    /// Refuses to drop menu edits that have not been written.
    ///
    /// Returns `true` when the caller must stop.
    fn discard_menu_edits(&mut self, cx: &mut Context<Self>) -> bool {
        if self.menu_file.as_ref().is_some_and(|menus| menus.dirty()) {
            self.message =
                Some(SharedString::from("menus non enregistrés — ⌘S avant de changer de fichier"));
            cx.notify();
            return true;
        }
        false
    }

    /// The text box bound to a field of the menu panel.
    pub(crate) fn menu_input(&self, field: MenuField) -> Option<&Entity<InputState>> {
        self.menu_inputs.iter().find(|(candidate, _)| *candidate == field).map(|(_, state)| state)
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
            Some(Selection::Item(..)) | Some(Selection::SubItem(..)) => {
                match menus.selected_item() {
                    Some(ItemDef::Action { label, action, .. }) => {
                        fields.push((MenuField::Label, label.clone()));
                        fields.push((MenuField::Action, action.clone()));
                    }
                    // Un sous-menu porte un titre, sous le même champ Libellé
                    // que l'inspecteur affiche.
                    Some(ItemDef::Submenu(inner)) => {
                        fields.push((MenuField::Label, inner.name.clone()));
                    }
                    _ => {}
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
            (Selection::Item(..) | Selection::SubItem(..), _) => {
                match menus.selected_item_mut() {
                    Some(ItemDef::Action { label, action, .. }) => match field {
                        MenuField::Label => *label = value.to_string(),
                        // An action name is a Rust type: refuse what would not
                        // compile rather than write it.
                        MenuField::Action if is_type_name(value) => *action = value.to_string(),
                        _ => {}
                    },
                    Some(ItemDef::Submenu(inner)) if field == MenuField::Label => {
                        inner.name = value.to_string();
                    }
                    _ => {}
                }
            }
            _ => {}
        }
        cx.notify();
    }

    /// Opens the handler of the selected entry in Zed, on its own line.
    pub fn open_menu_handler(&mut self, cx: &mut Context<Self>) {
        let Some(menus) = self.menu_file.as_ref() else {
            return;
        };
        let Some(ItemDef::Action { action, os_action, .. }) = menus.selected_item() else {
            return;
        };
        if os_action.is_some() {
            self.message = Some(SharedString::from(
                "cette entrée est déléguée au système — elle n'a pas de gestionnaire",
            ));
            cx.notify();
            return;
        }
        if action.contains("::") {
            self.message = Some(SharedString::from(format!(
                "« {action} » vit dans un autre module — maxx ne sait pas où il est écrit"
            )));
            cx.notify();
            return;
        }

        match menus.handler_line(action) {
            Some(line) => crate::tools::open_in_editor(cx, &menus.path, Some(line)),
            None => {
                self.message = Some(SharedString::from(format!(
                    "« {action} » n'est pas encore câblée — ⌘S l'ajoute au fichier"
                )));
                cx.notify();
            }
        }
    }

    /// Selects a menu or one of its entries.
    pub fn select_menu(&mut self, selection: Selection, cx: &mut Context<Self>) {
        if let Some(menus) = self.menu_file.as_mut() {
            menus.selected = Some(selection);
            cx.notify();
        }
    }

    /// Leaves the menu editor.
    pub fn close_menu_file(&mut self, cx: &mut Context<Self>) {
        if self.discard_menu_edits(cx) {
            return;
        }
        self.menu_file = None;
        self.menu_synced = None;
        cx.notify();
    }

    /// Copies the system module into the project and points the explorer at it.
    ///
    /// Pointed at, not opened: `systeme.rs` carries no managed region, so the
    /// designer has nothing to show for it — it is read and edited in the
    /// editor, like any other hand-written module.
    ///
    /// What every desktop application ends up writing on its second day —
    /// where its files go, and what "delete" means — and what nobody wants to
    /// write a third time. Copied source, not a dependency: a generated
    /// project owes nothing to maxx.
    pub fn add_system_module(&mut self, cx: &mut Context<Self>) {
        self.add_module(
            "systeme",
            crate::scaffold::add_system_module,
            "module système ajouté au projet et déclaré dans main.rs",
            cx,
        );
    }

    /// Copies a module into the project, declares it, and points at it.
    ///
    /// Shared by the modules maxx knows how to add, so they all leave the
    /// window in the same state — no menu editor left in front of a file that
    /// was just written, and no unsaved menu edits dropped on the way.
    fn add_module(
        &mut self,
        module: &str,
        add: fn(&std::path::Path) -> std::io::Result<()>,
        added: &'static str,
        cx: &mut Context<Self>,
    ) {
        let Some(project) = self.project.as_ref() else {
            self.message = Some(SharedString::from("aucun projet ouvert"));
            cx.notify();
            return;
        };
        let root = project.root.clone();
        let path = root.join(format!("src/{module}.rs"));
        let declaration = format!("mod {module};");
        let had_file = path.exists();
        let had_declaration = std::fs::read_to_string(root.join("src/main.rs"))
            .is_ok_and(|source| source.lines().any(|line| line.trim() == declaration));

        // Unsaved menu edits come first: this leaves the menu editor, and
        // dropping them silently would be the worst way to add a file.
        if self.discard_menu_edits(cx) {
            return;
        }

        if let Err(error) = add(&root) {
            self.message = Some(SharedString::from(error.to_string()));
            cx.notify();
            return;
        }

        self.menu_file = None;
        self.menu_synced = None;
        self.preferences = false;
        self.refresh_entries();
        self.selected = Some(path);
        self.message = Some(SharedString::from(match (had_file, had_declaration) {
            (true, true) => format!("src/{module}.rs est déjà là"),
            // The file was there but nothing declared it — which is exactly
            // the state a half-finished delete leaves behind.
            (true, false) => {
                format!("src/{module}.rs était là, il est maintenant déclaré dans main.rs")
            }
            _ => added.to_string(),
        }));
        cx.notify();
    }

    /// Copies the settings module into the project.
    ///
    /// It brings the system module with it, and declares two crates in the
    /// project's `Cargo.toml` — both already compiled in the tree through
    /// gpui, so nothing gets slower.
    pub fn add_settings_module(&mut self, cx: &mut Context<Self>) {
        self.add_module(
            "reglages",
            crate::scaffold::add_settings_module,
            "réglages ajoutés au projet, avec le module système et leurs deux crates",
            cx,
        );
    }

    /// Replaces the copied modules a newer maxx has fixed.
    ///
    /// Only those the project has not touched: an edited file belongs to the
    /// developer, and maxx says so rather than deciding for them.
    pub fn update_modules(&mut self, cx: &mut Context<Self>) {
        let Some(project) = self.project.as_ref() else {
            self.message = Some(SharedString::from("aucun projet ouvert"));
            cx.notify();
            return;
        };
        let root = project.root.clone();
        let outdated = crate::scaffold::outdated_modules(&root);

        if outdated.is_empty() {
            self.message = Some(SharedString::from("les modules de ce projet sont à jour"));
            cx.notify();
            return;
        }

        let mut updated = Vec::new();
        let mut failed = Vec::new();
        for module in &outdated {
            match crate::scaffold::update_module(&root, module) {
                Ok(()) => updated.push(module.clone()),
                Err(error) => failed.push(format!("{module} : {error}")),
            }
        }

        self.refresh_entries();
        self.message = Some(SharedString::from(if failed.is_empty() {
            format!("mis à jour : {}", updated.join(", "))
        } else {
            failed.join(" · ")
        }));
        cx.notify();
    }

    /// Says so when the project carries a module maxx has since fixed.
    ///
    /// A message and nothing more: replacing a file because someone opened a
    /// folder would be a poor way to earn trust.
    fn announce_outdated_modules(&mut self, root: &std::path::Path) {
        let outdated = crate::scaffold::outdated_modules(root);
        if outdated.is_empty() {
            return;
        }
        self.message = Some(SharedString::from(format!(
            "{} a une version plus récente — Fichier ▸ Ajouter au projet ▸ Mettre à jour",
            outdated.join(", ")
        )));
    }

    /// Shows the preferences, or leaves them when they are already up.
    ///
    /// A toggle rather than an open: `⌘,` pressed twice is how you check a
    /// setting and go straight back to what you were drawing.
    pub fn toggle_preferences(&mut self, cx: &mut Context<Self>) {
        self.preferences = !self.preferences;
        cx.notify();
    }

    /// Leaves the preferences screen.
    pub fn close_preferences(&mut self, cx: &mut Context<Self>) {
        if self.preferences {
            self.preferences = false;
            cx.notify();
        }
    }

    /// Opens the project's menu bar for editing.
    ///
    /// `src/menus.rs` is a file like any other, so a click in the explorer
    /// opens it — but nothing in the window said so, and the editor was
    /// unfindable for anyone who had not been told.
    pub fn open_menu_bar(&mut self, cx: &mut Context<Self>) {
        self.preferences = false;
        let Some(project) = self.project.as_ref() else {
            self.message = Some(SharedString::from("aucun projet ouvert"));
            cx.notify();
            return;
        };
        if self.menu_file.is_some() {
            // Already open — but the preferences may have been covering it.
            cx.notify();
            return;
        }
        let root = project.root.clone();
        let path = root.join("src/menus.rs");

        // A project made before maxx generated a menu bar — or one whose bar
        // was deleted — gets one now rather than a refusal.
        let added = !path.exists();
        if added {
            if let Err(error) = crate::scaffold::add_menu_bar(&root) {
                self.message = Some(SharedString::from(error.to_string()));
                cx.notify();
                return;
            }
            self.refresh_entries();
        }

        self.select_file(path, cx);
        if added && self.message.is_none() {
            self.message =
                Some(SharedString::from("barre de menus ajoutée au projet et câblée dans main.rs"));
        }
    }

    /// Takes the menu bar away from the project: `src/menus.rs` to the Trash,
    /// `main.rs` unwired.
    pub fn remove_menu_bar(&mut self, cx: &mut Context<Self>) {
        let Some(project) = self.project.as_ref() else {
            self.message = Some(SharedString::from("aucun projet ouvert"));
            cx.notify();
            return;
        };
        let path = project.root.join("src/menus.rs");
        if !path.exists() {
            self.message = Some(SharedString::from("ce projet n'a pas de barre de menus"));
            cx.notify();
            return;
        }
        // Through the panel's own delete, so the file goes to the Trash and
        // `main.rs` is unwired exactly once, in one place.
        self.selected = Some(path);
        self.delete_selected_entry(cx);
    }

    /// Adds a menu to the bar.
    pub fn add_menu(&mut self, cx: &mut Context<Self>) {
        self.open_menu_bar(cx);
        if let Some(menus) = self.menu_file.as_mut() {
            menus.add_menu();
            cx.notify();
        }
    }

    /// Adds an entry to the selected menu.
    pub fn add_menu_item(&mut self, separator: bool, cx: &mut Context<Self>) {
        self.open_menu_bar(cx);
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
            ItemDef::Action { label: "Entrée".into(), action: "MonAction".into(), os_action: None }
        };
        menus.add_item(item);
        cx.notify();
    }

    /// Moves the selected menu or entry one place up, or down.
    pub fn move_menu_selection(&mut self, up: bool, cx: &mut Context<Self>) {
        let Some(menus) = self.menu_file.as_mut() else {
            return;
        };
        if menus.selected.is_none() {
            self.message = Some(SharedString::from("sélectionnez d'abord une entrée"));
            cx.notify();
            return;
        }
        if menus.move_selected(up) {
            // Sans cela, le « déjà en dernier » d'un coup bloqué survivait à
            // tous les déplacements suivants.
            self.message = None;
        } else {
            // Already at the end of its list: saying so beats a click that
            // looks broken.
            self.message = Some(SharedString::from(if up {
                "déjà en premier"
            } else {
                "déjà en dernier"
            }));
        }
        cx.notify();
    }

    /// Adds a submenu to the selected menu.
    ///
    /// Only inside a menu of the bar: a submenu of a submenu is a place nobody
    /// finds twice, and the model stops at one level on purpose.
    pub fn add_submenu(&mut self, cx: &mut Context<Self>) {
        self.open_menu_bar(cx);
        let Some(menus) = self.menu_file.as_mut() else {
            return;
        };
        // Un sous-menu sélectionné accueillerait l'entrée à l'intérieur de
        // lui-même, ce que `add_item` fait exprès pour les autres entrées : ici
        // cela donnerait le sous-menu de sous-menu que le modèle ne sait pas
        // afficher, et qu'on ne pourrait donc plus ni sélectionner ni retirer.
        let dans_un_sous_menu = matches!(menus.selected, Some(Selection::SubItem(..)))
            || matches!(menus.selected_item(), Some(ItemDef::Submenu(_)));
        if menus.selected.is_none() || dans_un_sous_menu {
            self.message = Some(SharedString::from(
                "sélectionnez un menu ou une de ses entrées — un sous-menu ne va pas dans un sous-menu",
            ));
            cx.notify();
            return;
        }
        menus.add_item(ItemDef::Submenu(crate::menu_model::MenuDef::named("Sous-menu")));
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
        let key = self.view().map(|view| (self.revision, view.selected.clone()));
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
                self.message = Some(SharedString::from("aucun projet ouvert"));
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
            self.message = Some(SharedString::from("aucune action sur ce nœud"));
            cx.notify();
            return;
        };
        match view.method_line(&name) {
            Some(line) => crate::tools::open_in_editor(cx, &view.path, Some(line)),
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
        let accepts_children =
            view.root.at(&selected).and_then(registry::of).is_some_and(|spec| spec.container);
        let child_count = view.root.at(&selected).map(|node| node.children.len()).unwrap_or(0);

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
            self.message =
                Some(SharedString::from("une exécution est déjà en cours — ⌘. pour l'arrêter"));
            cx.notify();
            return;
        }

        let root = project.root.clone();
        self.run_output.clear();
        self.run_state = crate::run::State::Running;
        self.run_pid = None;
        crate::settings::update_prefs(cx, |preferences| preferences.show_output = true);
        self.message = None;

        let receiver = if prewarm { crate::run::prewarm(root) } else { crate::run::start(root) };
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
                            workspace.output_scroll.scroll_to_item(last, ScrollStrategy::Top);
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
                cx.background_executor().timer(std::time::Duration::from_millis(80)).await;
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
        crate::settings::update_prefs(cx, |preferences| {
            preferences.show_output = !preferences.show_output;
        });
        notify_all(cx);
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
                    self.message =
                        Some(SharedString::from("un nœud ne peut pas être déposé dans lui-même"));
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

    /// Highlights an entry of the project panel without opening it.
    ///
    /// A right click has to land on the entry it is about — the delete and
    /// reveal actions all read `selected` — but it must not open the file the
    /// way a left click does.
    pub fn select_entry(&mut self, path: PathBuf, cx: &mut Context<Self>) {
        self.selected = Some(path);
        cx.notify();
    }

    /// The entry the project panel is on, falling back to the project root.
    pub fn selected_entry(&self) -> Option<PathBuf> {
        self.selected.clone().or_else(|| self.project.as_ref().map(|project| project.root.clone()))
    }

    /// Moves the selected entry to the Trash, unregistering it when it is a
    /// view.
    ///
    /// Nothing is erased: the file lands in `~/.Trash`, so a wrong click costs
    /// a trip to the Finder and not the afternoon.
    pub fn delete_selected_entry(&mut self, cx: &mut Context<Self>) {
        let Some(project) = self.project.as_ref() else {
            return;
        };
        let root = project.root.clone();
        let Some(path) = self.selected.clone() else {
            self.message =
                Some(SharedString::from("sélectionnez d'abord un élément dans l'explorateur"));
            cx.notify();
            return;
        };

        if let Some(reason) = protected_entry(&root, &path) {
            self.message = Some(SharedString::from(reason));
            cx.notify();
            return;
        }

        let name = path
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_else(|| path.to_string_lossy().into_owned());

        // A directory takes everything under it: the modules have to be read
        // off the disk while it is still there.
        let is_dir = path.is_dir();
        let modules: Vec<String> = if is_dir {
            std::fs::read_dir(&path)
                .map(|entries| {
                    entries
                        .flatten()
                        .filter_map(|entry| view_module(&root, &entry.path()))
                        .collect()
                })
                .unwrap_or_default()
        } else {
            view_module(&root, &path).into_iter().collect()
        };

        if let Err(error) = crate::run::move_to_trash(&path) {
            self.message = Some(SharedString::from(error));
            cx.notify();
            return;
        }

        // A view carries a `pub mod` line in `src/ui/mod.rs`; leaving it there
        // breaks the build the file was deleted to keep clean. A directory
        // inside `src/ui/` is itself a module, on top of the views it held.
        for module in &modules {
            unregister_view(&root, module);
        }
        if is_dir
            && let Some(module) = path
                .strip_prefix(root.join("src/ui"))
                .ok()
                .and_then(|relative| relative.to_str())
                .filter(|relative| !relative.contains('/'))
        {
            unregister_view(&root, module);
        }
        // Same for the menu bar: the file is gone, so `main.rs` must stop
        // calling into it.
        if path == root.join("src/menus.rs") {
            let _ = crate::scaffold::remove_menu_bar(&root);
        } else if let Some(module) = top_level_module(&root, &path) {
            // Any other `src/<module>.rs`: its `mod` line would now name a
            // file that is gone, and the project would stop compiling — which
            // is the opposite of what deleting a file is for.
            let _ = crate::scaffold::remove_module(&root, &module);
        }

        // Every tab under it, and the menu editor, are now looking at a file
        // that is gone.
        let gone = |candidate: &std::path::Path| {
            candidate == path || (is_dir && candidate.starts_with(&path))
        };
        while let Some(index) = self.views.iter().position(|view| gone(&view.path)) {
            self.views.remove(index);
            self.active = match self.active {
                Some(_) if self.views.is_empty() => None,
                Some(active) if active >= index && active > 0 => Some(active - 1),
                Some(active) => Some(active.min(self.views.len() - 1)),
                None => None,
            };
            self.revision += 1;
        }
        if self.menu_file.as_ref().is_some_and(|menus| gone(&menus.path)) {
            self.menu_file = None;
            self.menu_synced = None;
        }

        self.selected = None;
        self.expanded.retain(|expanded| !gone(expanded));
        self.refresh_entries();
        self.message = Some(SharedString::from(format!("{name} déplacé vers la corbeille")));
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
            let path = menus.path.clone();
            self.message = match menus.save(force) {
                Ok(()) => Some(SharedString::from(format!("{} enregistré", menus.name()))),
                Err(error) => Some(SharedString::from(error)),
            };
            self.format_after_save(&path, cx);
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
        self.format_after_save(&path, cx);
        self.conflicts.remove(&path);
        self.revision += 1;
        cx.notify();
    }

    /// Passes the freshly written file through `rustfmt`, when asked to.
    ///
    /// The re-read afterwards is not optional: maxx holds a copy of the file
    /// and compares it with the disk to notice edits made elsewhere. Leaving
    /// that copy behind would make the very next save believe someone had
    /// changed the file underneath — maxx accusing itself.
    fn format_after_save(&mut self, path: &std::path::Path, cx: &mut Context<Self>) {
        if !crate::settings::prefs(cx).format_on_save {
            return;
        }
        // Nothing to format if the save itself failed.
        if self.message.as_deref().is_some_and(|message| !message.ends_with("enregistré")) {
            return;
        }

        match crate::run::format_rust(path) {
            Ok(false) => {}
            Ok(true) => {
                let reloaded = match self.menu_file.as_mut() {
                    Some(menus) if menus.path == path => menus.reload().err(),
                    _ => self.view_mut().and_then(|view| view.reload().err()),
                };
                if let Some(error) = reloaded {
                    self.message = Some(SharedString::from(error));
                } else {
                    self.menu_synced = None;
                    self.revision += 1;
                }
            }
            Err(error) => self.message = Some(SharedString::from(error)),
        }
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
            self.message =
                Some(SharedString::from("sélectionnez un fichier .rs dans l'explorateur"));
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
                    .child(div().text_xs().text_color(rgb(theme::TEXT_MUTED)).child(subtitle)),
            )
    }

    fn render_project_panel(&self, cx: &mut Context<Self>) -> impl IntoElement {
        // One menu for the whole panel rather than one per row: `ContextMenu`
        // hard-codes its element id, so a menu per row would have every row
        // sharing the same open/position state.
        //
        // No entry is greyed out from the selection: `ContextMenu` builds the
        // menu from the builder of the frame it was painted with, so anything
        // computed here would be one right click behind. `DeleteFile` reports
        // its own refusal.
        //
        // The editor's name is read here rather than inside the builder, which
        // is `'static` and cannot hold the application. Changing the editor
        // repaints every workspace, so the label follows on the next frame.
        let editor = crate::tools::editor_label(cx);

        div()
            .flex()
            .flex_col()
            // Pas de largeur ici : c'est le volet redimensionnable qui la
            // donne. Une largeur fixe à l'intérieur laissait l'arborescence à
            // 240 px dans un volet plus large, et la bande vide entre les deux
            // se voyait.
            .size_full()
            .bg(rgb(theme::PANEL_BG))
            .border_r_1()
            .border_color(rgb(theme::BORDER))
            .child(
                div()
                    .flex()
                    .items_center()
                    .h(px(28.))
                    .pl(px(12.))
                    .pr(px(4.))
                    .child(
                        div()
                            .flex_1()
                            .text_xs()
                            .text_color(rgb(theme::TEXT_MUTED))
                            .child("EXPLORATEUR"),
                    )
                    .child(panel_icon("panel-new-view", "＋", "Nouvelle vue", cx, |this, cx| {
                        this.new_view(cx)
                    }))
                    .child(panel_icon("panel-delete", "🗑", "Supprimer", cx, |this, cx| {
                        this.delete_selected_entry(cx)
                    })),
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
            .context_menu(move |menu, _window, _cx| {
                menu.menu("Nouvelle vue", Box::new(crate::actions::NewView))
                    .menu("Supprimer", Box::new(crate::actions::DeleteFile))
                    .separator()
                    .menu("Révéler dans le Finder", Box::new(crate::actions::RevealInFinder))
                    .menu(format!("Ouvrir dans {editor}"), Box::new(crate::actions::OpenInZed))
            })
    }

    fn render_entry(&self, entry: Entry, cx: &mut Context<Self>) -> AnyElement {
        let is_selected = self.selected.as_deref() == Some(entry.path.as_path());
        let is_expanded = self.expanded.contains(&entry.path);
        let marker = if entry.is_dir { if is_expanded { "▾" } else { "▸" } } else { " " };
        let path = entry.path.clone();
        let is_dir = entry.is_dir;

        let menu_path = entry.path.clone();

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
            // The menu acts on `selected`, so the right click has to move the
            // selection before the menu is built — which it does, the menu
            // being deferred to the next frame.
            .on_mouse_down(
                gpui::MouseButton::Right,
                cx.listener(move |this, _, _window, cx| {
                    this.select_entry(menu_path.clone(), cx);
                }),
            )
            .into_any_element()
    }

    fn render_main(&self, cx: &mut Context<Self>) -> AnyElement {
        // Before the welcome screen: the preferences must be reachable when no
        // project is open, which is exactly when someone is setting maxx up.
        if self.preferences {
            return self.render_designer(cx);
        }
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
                div().text_color(rgb(theme::TEXT_MUTED)).child("Ouvrez un dossier pour commencer."),
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
            .child(div().text_xs().text_color(rgb(theme::TEXT_MUTED)).child("⌘O"))
            .children(self.render_recent_projects(cx))
            .into_any_element()
    }

    /// The recent projects, on the welcome screen.
    ///
    /// The same list as the one in the File menu, put where someone who has
    /// just launched maxx is already looking.
    fn render_recent_projects(&self, cx: &mut Context<Self>) -> Option<AnyElement> {
        let recent = crate::settings::state(cx).recent_projects.clone();
        if recent.is_empty() {
            return None;
        }

        let rows = recent.into_iter().enumerate().map(|(index, path)| {
            let name = path
                .file_name()
                .map(|name| name.to_string_lossy().into_owned())
                .unwrap_or_else(|| path.to_string_lossy().into_owned());
            let parent = path
                .parent()
                .map(|parent| parent.to_string_lossy().into_owned())
                .unwrap_or_default();

            div()
                .id(SharedString::from(format!("recent-{index}")))
                .flex()
                .items_baseline()
                .gap_2()
                .px_3()
                .py_1()
                .rounded_sm()
                .cursor_pointer()
                .hover(|this| this.bg(rgb(theme::HOVER_BG)))
                .child(div().child(name))
                .child(div().text_xs().text_color(rgb(theme::TEXT_MUTED)).child(parent))
                .on_click(cx.listener(move |_, _, window, cx| {
                    window.dispatch_action(Box::new(crate::actions::OpenRecent { index }), cx);
                }))
        });

        Some(
            div()
                .flex()
                .flex_col()
                .mt_4()
                .gap_1()
                .items_start()
                .child(
                    div()
                        .px_3()
                        .text_xs()
                        .text_color(rgb(theme::TEXT_MUTED))
                        .child("Projets récents"),
                )
                .children(rows)
                .into_any_element(),
        )
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
                    .child(
                        uniform_list(
                            "run-output",
                            lines.len(),
                            cx.processor(
                                move |this, range: std::ops::Range<usize>, _window, _cx| {
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
                                },
                            ),
                        )
                        .track_scroll(self.output_scroll.clone())
                        .size_full(),
                    )
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
        let conflict = self.view().is_some_and(|view| self.conflicts.contains(&view.path));
        if let Some(menus) = self.menu_file.as_ref() {
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
            (None, None, Some(project)) => {
                SharedString::from(format!("{} · {} éléments", project.name, self.entries.len()))
            }
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

        // Only the active window, and only in memory: there is a single saved
        // geometry, so letting every window stage its own would persist
        // whichever repainted last. Writing the file on every frame of a drag
        // would be absurd anyway — `settings::flush` at quit puts it away.
        let bounds = window.bounds();
        if active {
            crate::settings::stage_state(cx, |state| {
                state.window = Some(crate::settings::WindowGeometry {
                    x: bounds.origin.x.into(),
                    y: bounds.origin.y.into(),
                    width: bounds.size.width.into(),
                    height: bounds.size.height.into(),
                });
            });
        }

        self.sync_prop_inputs(window, cx);
        self.sync_menu_inputs(window, cx);
        let visible = crate::settings::prefs(cx).clone();
        let show_panel = visible.show_project_panel && self.project.is_some();
        let panel_width = crate::settings::state(cx).panel_width.unwrap_or(240.);

        // La poignée déplace la découpe dans l'entité de gpui-component ; c'est
        // ici qu'on la relit pour la retenir. En mémoire seulement, comme la
        // géométrie de la fenêtre : un fichier par image de glissement serait
        // absurde, et `settings::flush` l'écrit à l'extinction.
        if show_panel && let Some(largeur) = self.panel_split.read(cx).sizes().first().copied() {
            let largeur = f32::from(largeur);
            if largeur > 0. {
                crate::settings::stage_state(cx, |state| state.panel_width = Some(largeur));
            }
        }

        div()
            .flex()
            .flex_col()
            .size_full()
            .bg(rgb(theme::BG))
            .text_color(rgb(theme::TEXT))
            .text_sm()
            .child(self.render_titlebar())
            .child(
                // Sans le panneau, pas de poignée : un groupe redimensionnable
                // à un seul volet coûterait un état pour rien.
                div()
                    .flex()
                    .flex_1()
                    .overflow_hidden()
                    .when(!show_panel, |this| this.child(self.render_main(cx)))
                    .when(show_panel, |this| {
                        this.child(
                            h_resizable("panneaux")
                                .with_state(&self.panel_split)
                                .child(
                                    resizable_panel()
                                        .size(px(panel_width))
                                        // En dessous, l'arborescence devient
                                        // illisible ; au-delà, elle mange le
                                        // canvas.
                                        .size_range(px(160.)..px(520.))
                                        .child(fillable(self.render_project_panel(cx))),
                                )
                                .child(resizable_panel().child(fillable(self.render_main(cx)))),
                        )
                    }),
            )
            .when(visible.show_output, |this| this.child(self.render_output(cx)))
            .when(visible.show_status_bar, |this| this.child(self.render_status_bar()))
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
    // A project handed to a fresh window never passes through `set_project` —
    // neither the one opened from the command line, nor the one that gets a
    // window of its own because the current one is busy.
    if let Some(path) = path.as_deref() {
        remember_project(path, cx);
    }
    let project = path.clone().map(Project::open);
    let title =
        project.as_ref().map(|project| project.name.clone()).unwrap_or_else(|| "maxx".into());

    // Only the first window takes the saved geometry: handing it to a second
    // one would place it exactly over the first, pixel for pixel, hiding the
    // window someone is still using.
    let first_window = cx.default_global::<Workspaces>().0.is_empty();
    let geometry = crate::settings::state(cx).window.filter(|_| first_window);
    // Une géométrie enregistrée sur un écran qui n'est plus branché rendrait la
    // fenêtre invisible ; gpui rabat une fenêtre hors champ sur l'écran
    // principal, donc il n'y a rien de plus à faire ici.
    let bounds = match geometry {
        Some(geometry) => Bounds {
            origin: point(px(geometry.x), px(geometry.y)),
            size: size(px(geometry.width), px(geometry.height)),
        },
        None => Bounds::centered(None, size(px(1100.), px(720.)), cx),
    };

    let options = WindowOptions {
        window_bounds: Some(WindowBounds::Windowed(bounds)),
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
        let workspace = cx.new(|cx| Workspace::new(project, cx));
        *slot.borrow_mut() = Some(workspace.clone());
        cx.new(|cx| Root::new(workspace, window, cx))
    });

    let Ok(handle) = opened else {
        return;
    };
    if let Some(workspace) = created.borrow().as_ref() {
        cx.default_global::<Workspaces>().0.insert(handle.window_id(), workspace.downgrade());
    }
    if let Some(path) = path {
        cx.add_recent_document(&path);
    }
}

/// Why `path` must not be deleted from the project panel, if it must not.
///
/// The generated project names `accueil` in `main.rs` and `menus` in both
/// `main.rs` and the menu bar it installs: deleting either leaves a project
/// that no longer compiles, which is a worse outcome than a refusal.
pub fn protected_entry(root: &std::path::Path, path: &std::path::Path) -> Option<String> {
    if path == root {
        return Some("la racine du projet ne se supprime pas ici".into());
    }
    if !path.starts_with(root) {
        return Some("cet élément est hors du projet".into());
    }
    let relative = path.strip_prefix(root).ok()?.to_string_lossy().into_owned();
    let kept = [
        ("Cargo.toml", "Cargo.toml décrit le projet"),
        ("src/main.rs", "main.rs est le point d'entrée"),
        ("src/ui/mod.rs", "ui/mod.rs déclare les vues"),
        ("src/ui/accueil.rs", "accueil est la vue ouverte par main.rs"),
        ("src", "le dossier src porte tout le code"),
        ("src/ui", "le dossier ui porte les vues"),
    ];
    kept.iter()
        .find(|(candidate, _)| *candidate == relative)
        .map(|(_, reason)| format!("suppression refusée : {reason}"))
}

/// The module name of `path` when it is a `src/<module>.rs` declared in
/// `main.rs`.
///
/// `main.rs` itself is not a module, and `src/ui/` has its own `mod.rs`.
pub fn top_level_module(root: &std::path::Path, path: &std::path::Path) -> Option<String> {
    let relative = path.strip_prefix(root).ok()?;
    let mut parts = relative.components().map(|part| part.as_os_str());
    if parts.next()? != "src" {
        return None;
    }
    let file = parts.next()?.to_string_lossy().into_owned();
    if parts.next().is_some() {
        return None;
    }
    let module = file.strip_suffix(".rs")?;
    (module != "main").then(|| module.to_string())
}

/// The module name of `path` when it is one of the project's views.
pub fn view_module(root: &std::path::Path, path: &std::path::Path) -> Option<String> {
    let relative = path.strip_prefix(root).ok()?;
    let mut parts = relative.components().map(|part| part.as_os_str());
    if parts.next()? != "src" || parts.next()? != "ui" {
        return None;
    }
    let file = parts.next()?.to_string_lossy().into_owned();
    if parts.next().is_some() {
        return None;
    }
    file.strip_suffix(".rs").map(|module| module.to_string())
}

/// Drops `pub mod <module>;` from `src/ui/mod.rs`.
///
/// Textual, the way `scaffold::create_view` adds it, so the rest of the file
/// keeps whatever the developer wrote in it.
pub fn unregister_view(root: &std::path::Path, module: &str) {
    let mod_path = root.join("src/ui/mod.rs");
    let Ok(source) = std::fs::read_to_string(&mod_path) else {
        return;
    };
    let declaration = format!("pub mod {module};");
    let kept: Vec<&str> = source.lines().filter(|line| line.trim() != declaration).collect();
    let mut out = kept.join("\n");
    if !out.is_empty() {
        out.push('\n');
    }
    let _ = std::fs::write(&mod_path, out);
}

/// A small clickable glyph in the project panel header.
fn panel_icon(
    id: &'static str,
    glyph: &'static str,
    tooltip: &'static str,
    cx: &mut Context<Workspace>,
    action: impl Fn(&mut Workspace, &mut Context<Workspace>) + 'static,
) -> impl IntoElement {
    div()
        .id(id)
        .flex()
        .items_center()
        .justify_center()
        .w(px(22.))
        .h(px(22.))
        .rounded_sm()
        .text_xs()
        .cursor_pointer()
        .text_color(rgb(theme::TEXT_MUTED))
        .hover(|this| this.bg(rgb(theme::HOVER_BG)))
        .tooltip(move |window, cx| gpui_component::tooltip::Tooltip::new(tooltip).build(window, cx))
        .child(glyph)
        .on_click(cx.listener(move |this, _, _window, cx| action(this, cx)))
}

/// Wraps the content of a resizable panel so it can actually shrink.
///
/// Without this, dragging the handle only *pushes* what is beside it: a flex
/// item defaults to `min-width: auto`, so it refuses to go below the width of
/// its own content and overflows instead of compressing. A width of zero as a
/// floor, and the content follows the handle.
pub fn fillable(content: impl IntoElement) -> impl IntoElement {
    div().flex().size_full().min_w(px(0.)).overflow_hidden().child(content)
}

/// Asks every workspace to repaint.
///
/// A preference is global, so a window that is not the focused one has to
/// follow — otherwise it keeps the old layout until something else happens to
/// make it redraw.
pub fn notify_all(cx: &mut App) {
    // Deferred, and that is not a precaution: every caller runs inside the
    // update of one of these very workspaces — a menu action, or a click in
    // the preferences screen. Leasing an entity that is already leased aborts
    // the process, so the notifications wait for the current update to finish.
    cx.defer(|cx: &mut App| {
        let workspaces: Vec<WeakEntity<Workspace>> =
            cx.default_global::<Workspaces>().0.values().cloned().collect();
        for workspace in workspaces {
            if let Some(workspace) = workspace.upgrade() {
                workspace.update(cx, |_, cx| cx.notify());
            }
        }
    });
}

/// Puts `path` at the head of the recent projects and refreshes the menu bar.
fn remember_project(path: &std::path::Path, cx: &mut App) {
    let before = crate::settings::state(cx).recent_projects.clone();
    crate::settings::update_state(cx, |state| {
        state.remember_project(path);
    });
    if crate::settings::state(cx).recent_projects != before {
        // The recent list is a submenu, and a gpui menu bar is a value handed
        // over once: changing it means handing over a new one.
        cx.set_menus(crate::menus::app_menus(cx));
    }
}
