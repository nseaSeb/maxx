//! The workspace: the root view of every window, and the logic that decides
//! which window a freshly opened folder lands in.

use gpui::prelude::*;
use gpui::{
    AnyElement, App, Bounds, Context, Entity, Global, SharedString, TitlebarOptions, WeakEntity,
    Window, WindowBounds, WindowId, WindowOptions, div, point, px, rgb, size, uniform_list,
};
use gpui_component::Root;
use gpui_component::input::{InputEvent, InputState};
use std::collections::HashMap;

/// Which workspace lives in which window.
///
/// The window's root view is `gpui_component::Root` — several components walk
/// up to it and panic if it is anything else — so the workspace can no longer
/// be reached by downcasting the window handle.
#[derive(Default)]
struct Workspaces(HashMap<WindowId, WeakEntity<Workspace>>);

impl Global for Workspaces {}

/// Runs `f` against the workspace of the frontmost window.
pub fn with_active<R>(
    cx: &mut App,
    f: impl FnOnce(&mut Workspace, &mut Window, &mut Context<Workspace>) -> R,
) -> Option<R> {
    let handle = cx.active_window()?;
    let workspace = cx
        .try_global::<Workspaces>()?
        .0
        .get(&handle.window_id())?
        .upgrade()?;
    cx.update_window(handle, |_, window, cx| {
        workspace.update(cx, |workspace, cx| f(workspace, window, cx))
    })
    .ok()
}
use std::collections::HashSet;
use std::path::PathBuf;

use crate::actions::OpenFolder;
use crate::model::{Node, Path as NodePath};
use crate::registry::Kind;
use crate::project::{Entry, Project, flatten};
use crate::registry;
use crate::theme;
use crate::view::View;

/// Root view of a window. A workspace without a project shows the welcome
/// screen and can be reused by the next `Open Folder…`.
pub struct Workspace {
    project: Option<Project>,
    entries: Vec<Entry>,
    expanded: HashSet<PathBuf>,
    selected: Option<PathBuf>,
    show_panel: bool,
    show_status_bar: bool,
    /// The view currently being designed, if the selected file is one.
    pub view: Option<View>,
    /// Undo stack: whole-tree snapshots, the cheap and correct option at this
    /// size.
    past: Vec<Node>,
    /// Redo stack.
    future: Vec<Node>,
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
            view: None,
            past: Vec::new(),
            future: Vec::new(),
            message: None,
            prop_inputs: Vec::new(),
            revision: 0,
            synced: None,
        };
        workspace.refresh_entries();
        workspace
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
        if self.view.as_ref().is_some_and(|view| view.dirty()) {
            self.message = Some(SharedString::from(
                "vue non enregistrée — ⌘S avant de fermer le projet",
            ));
            cx.notify();
            return;
        }
        self.project = None;
        self.view = None;
        self.past.clear();
        self.future.clear();
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
        if self.view.as_ref().is_some_and(|view| view.dirty()) {
            self.message = Some(SharedString::from(
                "vue non enregistrée — ⌘S pour l'écrire, ⌘Z pour revenir en arrière",
            ));
            cx.notify();
            return;
        }
        self.message = None;
        if path.extension().is_some_and(|extension| extension == "rs") {
            match View::load(&path) {
                Ok(view) => {
                    self.view = Some(view);
                    self.past.clear();
                    self.future.clear();
                    self.revision += 1;
                }
                Err(error) => {
                    self.view = None;
                    self.message = Some(SharedString::from(error));
                }
            }
        }
        self.selected = Some(path);
        cx.notify();
    }

    /// Selects a node of the tree being designed.
    pub fn select(&mut self, path: NodePath, cx: &mut Context<Self>) {
        if let Some(view) = self.view.as_mut() {
            view.selected = path;
            cx.notify();
        }
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
    fn sync_prop_inputs(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let key = self
            .view
            .as_ref()
            .map(|view| (self.revision, view.selected.clone()));
        if key == self.synced {
            return;
        }
        self.synced = key;
        self.prop_inputs.clear();

        let Some(view) = self.view.as_ref() else {
            return;
        };
        let node = view.selected();
        let Some(spec) = crate::registry::of(node) else {
            return;
        };

        for prop in spec.props {
            if !matches!(prop.kind, Kind::Text | Kind::Field)
                || !crate::registry::editable(node, prop)
            {
                continue;
            }
            let value = crate::registry::read(node, prop).unwrap_or_default();
            let state = cx.new(|cx| InputState::new(window, cx).default_value(value));
            cx.subscribe(&state, move |this, state, event: &InputEvent, cx| {
                if matches!(event, InputEvent::Change) {
                    let value = state.read(cx).value().to_string();
                    this.edit_prop_text(prop, &value, cx);
                }
            })
            .detach();
            self.prop_inputs.push((prop, state));
        }
    }

    /// Writes a text property without disturbing the field being typed in: no
    /// undo checkpoint per keystroke, and no revision bump, so `sync` leaves the
    /// caret alone.
    fn edit_prop_text(&mut self, prop: &'static crate::registry::Prop, value: &str, cx: &mut Context<Self>) {
        let Some(view) = self.view.as_mut() else {
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
        cx.notify();
    }

    /// Records the current tree so the change about to be made can be undone.
    fn checkpoint(&mut self) {
        self.revision += 1;
        if let Some(view) = self.view.as_ref() {
            self.past.push(view.root.clone());
            self.future.clear();
        }
    }

    /// Writes a property of the selected node.
    pub fn edit_prop(&mut self, prop: &'static crate::registry::Prop, value: &str, cx: &mut Context<Self>) {
        let Some(view) = self.view.as_mut() else {
            return;
        };
        let selected = view.selected.clone();
        if view.root.at(&selected).is_some() {
            self.checkpoint();
            let view = self.view.as_mut().expect("just borrowed");
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
        let Some(view) = self.view.as_ref() else {
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
        let view = self.view.as_mut().expect("just borrowed");
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
                let Some(view) = self.view.as_ref() else {
                    return;
                };
                if id == "input" {
                    let field = registry::unique_input_field(&view.root);
                    if let crate::model::Base::Known { args, .. } = &mut node.base {
                        *args = vec![crate::model::Arg::Verbatim(format!("&self.{field}"))];
                    }
                }
                self.checkpoint();
                let view = self.view.as_mut().expect("just borrowed");
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
                let view = self.view.as_mut().expect("a view is open to be dropped on");
                match view.root.move_node(&from, &destination) {
                    Some(landed) => view.selected = landed,
                    None => {
                        // Nothing moved: drop the checkpoint we just took.
                        self.past.pop();
                    }
                }
            }
        }
        cx.notify();
    }

    /// Gives the selected node a handler, named after it, if its component has
    /// an action property and none is set yet. Bound to double-click.
    pub fn add_handler_to_selection(&mut self, cx: &mut Context<Self>) {
        let Some(view) = self.view.as_ref() else {
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
        let view = self.view.as_mut().expect("just borrowed");
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
        let Some(view) = self.view.as_ref() else {
            return;
        };
        if view.selected.is_empty() {
            return;
        }
        let selected = view.selected.clone();
        self.checkpoint();
        let view = self.view.as_mut().expect("just borrowed");
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

    /// Writes the view back to its file.
    pub fn save_view(&mut self, cx: &mut Context<Self>) {
        let Some(view) = self.view.as_mut() else {
            return;
        };
        self.message = match view.save() {
            Ok(()) => Some(SharedString::from(format!("{} enregistré", view.name()))),
            Err(error) => Some(SharedString::from(error)),
        };
        cx.notify();
    }

    /// Steps back one edit.
    pub fn undo(&mut self, cx: &mut Context<Self>) {
        let Some(view) = self.view.as_mut() else {
            return;
        };
        if let Some(previous) = self.past.pop() {
            self.revision += 1;
            self.future.push(std::mem::replace(&mut view.root, previous));
            view.selected.clear();
            cx.notify();
        }
    }

    /// Steps forward one edit.
    pub fn redo(&mut self, cx: &mut Context<Self>) {
        let Some(view) = self.view.as_mut() else {
            return;
        };
        if let Some(next) = self.future.pop() {
            self.revision += 1;
            self.past.push(std::mem::replace(&mut view.root, next));
            view.selected.clear();
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

    fn render_status_bar(&self) -> impl IntoElement {
        let label = match (&self.message, &self.view, &self.project) {
            (Some(message), _, _) => message.clone(),
            (None, Some(view), _) => SharedString::from(format!(
                "{}{} · {} nœuds",
                view.name(),
                if view.dirty() { " •" } else { "" },
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
        self.sync_prop_inputs(window, cx);
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
