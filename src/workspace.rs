//! The workspace: the root view of every window, and the logic that decides
//! which window a freshly opened folder lands in.

mod chrome;
mod code;
mod explorer;
mod inspector;
mod menus;
mod modules;
mod palette;
mod process;
mod views;

pub use chrome::fillable;
pub use code::{CodeFile, language_for};
pub use explorer::{protected_entry, top_level_module, unregister_view, view_module};

use rust_i18n::t;

use gpui::prelude::*;
use gpui::{
    AnyElement, App, Bounds, Context, Entity, Global, SharedString, TitlebarOptions, WeakEntity,
    Window, WindowBounds, WindowId, WindowOptions, div, point, px, size, uniform_list,
};
use gpui::{ScrollHandle, ScrollStrategy, Task, UniformListScrollHandle};
use gpui_component::Root;
use gpui_component::Sizable as _;
use gpui_component::input::{Input, InputEvent, InputState};
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
    /// The keystroke bound to an entry's action.
    Shortcut,
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

/// What the palette is offering while it is open.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum PaletteMode {
    /// The menu bar, flattened: `⌘K`.
    Commands,
    /// The project's files: `⌘P`.
    Files,
}

/// Root view of a window. A workspace without a project shows the welcome
/// screen and can be reused by the next `Open Folder…`.
pub struct Workspace {
    project: Option<Project>,
    entries: Vec<Entry>,
    expanded: HashSet<PathBuf>,
    selected: Option<PathBuf>,
    /// The view the project's window opens on, as `maxx.toml` records it.
    ///
    /// Held rather than read where it is drawn: the explorer paints every row
    /// of every frame, and reading a file from there would read it a hundred
    /// times a second. Refreshed with the rows themselves.
    entry_view: Option<PathBuf>,
    /// The project's menu bar, when `src/menus.rs` is the file being edited.
    pub menu_file: Option<MenuFile>,
    /// The file the code reader is showing, when it is showing one.
    pub code: Option<CodeFile>,
    /// The reader's field, built once per file.
    code_input: Option<Entity<InputState>>,
    /// The file that field was built for.
    code_synced: Option<PathBuf>,
    /// The revision the view's code was rendered at.
    code_revision: u64,
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
    /// The view that was in front before this one.
    ///
    /// A path and not an index: closing a tab shifts every index after it, and
    /// an index kept across a close names whatever slid into its place — so
    /// "previous file" would open some other file.
    previous_view: Option<PathBuf>,
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
    /// The task draining the runner's channel. Dropping it stops the drain.
    run_task: Option<Task<()>>,
    /// The task draining the watcher's channel. Dropping it stops the drain.
    ///
    /// Cleared from the foreground only: dropping a `Task` cancels its future,
    /// and a future that cancelled itself while being polled would abort the
    /// process. Replacing the field is therefore what ends the previous drain,
    /// and the drain itself only ever returns.
    watch_task: Option<Task<()>>,
    /// The watcher itself, kept alive here: dropped, it stops watching.
    watcher: Option<notify::RecommendedWatcher>,
    /// Text boxes of the run page, one per editable field of `[run]`.
    run_inputs: Vec<(crate::projectfile::RunField, Entity<InputState>)>,
    /// The project those boxes were built for, so they are built once.
    run_synced: Option<PathBuf>,
    /// `[run]` as it stood on disk when the boxes were built.
    ///
    /// Kept beside the edited one so a write can tell what changed on which
    /// side, the way `View::saved` does for a tree.
    run_loaded: crate::projectfile::Run,
    /// `[run]` as it stands, read once with the boxes rather than per frame.
    ///
    /// Held rather than re-read where it is drawn: the preferences screen
    /// repaints on every notification, and `maxx.toml` is a file.
    run_config: crate::projectfile::Run,
    /// Name box of the view panel, where a view is renamed.
    rename_input: Option<Entity<InputState>>,
    /// Name box of the state panel.
    state_name_input: Option<Entity<InputState>>,
    /// Search box of the component palette.
    palette_filter: Option<Entity<InputState>>,
    /// The command palette's box, while it is open.
    command_input: Option<Entity<InputState>>,
    /// The commands the palette was opened on.
    ///
    /// Built once, at opening: walking the menu bar reads the settings and asks
    /// the system which editors are installed, which is not a thing to do on
    /// every keystroke.
    commands: Vec<crate::palette::Command>,
    /// The files `⌘P` is offering, relative to the project root.
    ///
    /// Beside the commands rather than instead of them: the palette is one
    /// box, one keymap and one list on screen, and what changes between `⌘K`
    /// and `⌘P` is only what fills it.
    palette_files: Vec<std::path::PathBuf>,
    /// Which of the two the open palette is showing.
    ///
    /// Said rather than guessed from an empty list: a project holding no file
    /// at all would otherwise fall back to the commands, and `⌘P` would answer
    /// a question nobody asked.
    palette_mode: PaletteMode,
    /// Which line of the command palette is highlighted.
    command_index: usize,
    /// Index into `view::STATE_TYPES` for the field about to be added.
    state_type: usize,
    /// The tree as it was when the focused inspector field was entered, and
    /// the view it belongs to.
    edit_snapshot: Option<(PathBuf, Node)>,
    /// Views changed both on disk and in the designer, awaiting a decision.
    conflicts: HashSet<PathBuf>,
    /// Projects where the assets module could not be added on its own.
    ///
    /// Kept so the refusal is said once and not on every save: it carries the
    /// lines to add by hand, and repeating it would be nagging.
    assets_refused: HashSet<PathBuf>,
    /// Whether the window held the focus on the previous frame, to notice the
    /// moment it comes back.
    was_active: bool,
    /// Scroll position of the right-hand panels.
    pub(crate) side_scroll: ScrollHandle,
    /// The natural size of the selected image, when there is one.
    ///
    /// Read once per selection rather than once per frame: `image_dimensions`
    /// opens the file, and the inspector redraws sixty times a second.
    pub(crate) image_size: Option<(u32, u32)>,
    /// The canvas scrolls on its own: a view taller than the window — an image
    /// at its natural size is enough — was cut off with no way down.
    pub(crate) canvas_scroll: ScrollHandle,
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
            entry_view: None,
            menu_file: None,
            code: None,
            code_input: None,
            code_synced: None,
            code_revision: 0,
            preferences: false,
            panel_split: cx.new(|_| ResizableState::default()),
            inspector_split: cx.new(|_| ResizableState::default()),
            menu_inputs: Vec::new(),
            menu_synced: None,
            views: Vec::new(),
            active: None,
            previous_view: None,
            message: (!outdated.is_empty()).then(|| {
                SharedString::from(
                    t!("message.modules_outdated", modules = outdated.join(", ")).into_owned(),
                )
            }),
            prop_inputs: Vec::new(),
            revision: 0,
            synced: None,
            run_output: Vec::new(),
            run_state: crate::run::State::Idle,
            run_pid: None,
            run_task: None,
            run_inputs: Vec::new(),
            run_synced: None,
            run_config: crate::projectfile::Run::default(),
            run_loaded: crate::projectfile::Run::default(),
            watch_task: None,
            watcher: None,
            rename_input: None,
            state_name_input: None,
            palette_filter: None,
            command_input: None,
            commands: Vec::new(),
            palette_files: Vec::new(),
            palette_mode: PaletteMode::Commands,
            command_index: 0,
            state_type: 0,
            edit_snapshot: None,
            conflicts: HashSet::new(),
            assets_refused: HashSet::new(),
            was_active: false,
            side_scroll: ScrollHandle::new(),
            canvas_scroll: ScrollHandle::new(),
            image_size: None,
            output_scroll: UniformListScrollHandle::new(),
        };
        workspace.refresh_entries();
        // Same reason as the notice above: a project handed straight to a fresh
        // window never passes through `set_project`, so the watch is armed here
        // too or that window never notices an outside edit.
        workspace.watch_project(cx);
        workspace
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
        // A window with no project can still hold a file in the reader; showing
        // it under the new project's name would be the previous project's file.
        self.forget_code(|_| true);
        let project = Project::open(path);
        window.set_window_title(&project.name);
        self.project = Some(project);
        self.expanded.clear();
        self.selected = None;
        self.refresh_entries();
        self.watch_project(cx);
        cx.notify();
    }

    /// Drops the project and returns the window to the welcome screen, so a
    /// later `Open Folder…` can reuse it.
    pub fn close_project(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.view().is_some_and(|view| view.dirty()) {
            self.message = Some(crate::tr("message.view_unsaved_close_project"));
            cx.notify();
            return;
        }
        self.project = None;
        self.watch_project(cx);
        self.views.clear();
        self.active = None;
        // The reader's status line comes before every other branch, welcome
        // screen included: left behind, it would name a file of a project that
        // is no longer open.
        self.forget_code(|_| true);
        self.menu_file = None;
        self.preferences = false;
        self.menu_synced = None;
        self.entries.clear();
        self.expanded.clear();
        self.selected = None;
        window.set_window_title("maxx");
        cx.notify();
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
    // A geometry saved on a screen that is no longer plugged in would make the
    // window invisible; gpui folds an off-screen window back onto the main
    // display, so there is nothing more to do here.
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
