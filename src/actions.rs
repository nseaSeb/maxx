//! Application actions, their global handlers and the default keymap.
//!
//! Every handler is registered on the `App` rather than on the workspace view:
//! menu items dispatch actions through the focused element path, and keeping
//! the handlers global means a menu item stays enabled regardless of which
//! element currently holds focus. Handlers that need a window resolve it from
//! [`App::active_window`].

use gpui::{App, AsyncApp, KeyBinding, PathPromptOptions, actions};

use crate::workspace::{self, Workspace};

actions!(
    maxx,
    [
        // Application menu
        About,
        Quit,
        HideApp,
        HideOthers,
        ShowAll,
        // File menu
        NewWindow,
        NewProject,
        NewView,
        OpenFolder,
        Save,
        DeleteNode,
        CloseFolder,
        CloseWindow,
        // Edit menu
        Undo,
        Redo,
        Cut,
        Copy,
        Paste,
        SelectAll,
        // View menu
        ToggleProjectPanel,
        ToggleStatusBar,
        ToggleOutput,
        // Run menu
        RunProject,
        StopProject,
        // Go menu
        RevealInFinder,
        // Window menu
        Minimize,
        Zoom,
        // Help menu
        OpenDocs,
    ]
);

/// URL opened by Help > GPUI Documentation.
const DOCS_URL: &str = "https://gpui.rs";

/// Registers the handler for every action reachable from the menu bar.
pub fn register_handlers(cx: &mut App) {
    cx.on_action(|_: &Quit, cx: &mut App| cx.quit());
    cx.on_action(|_: &HideApp, cx: &mut App| cx.hide());
    cx.on_action(|_: &HideOthers, cx: &mut App| cx.hide_other_apps());
    cx.on_action(|_: &ShowAll, cx: &mut App| cx.unhide_other_apps());
    cx.on_action(|_: &About, cx: &mut App| {
        log_about(cx);
    });
    cx.on_action(|_: &OpenDocs, cx: &mut App| cx.open_url(DOCS_URL));

    cx.on_action(|_: &NewWindow, cx: &mut App| {
        workspace::open_workspace_window(None, cx);
    });
    cx.on_action(open_folder);
    cx.on_action(new_project);

    cx.on_action(|_: &NewView, cx: &mut App| {
        with_active_workspace(cx, |workspace, _, cx| workspace.new_view(cx));
    });
    cx.on_action(|_: &Save, cx: &mut App| {
        with_active_workspace(cx, |workspace, _, cx| workspace.save_view(cx));
    });
    cx.on_action(|_: &DeleteNode, cx: &mut App| {
        with_active_workspace(cx, |workspace, _, cx| workspace.delete_selected(cx));
    });
    cx.on_action(|_: &Undo, cx: &mut App| {
        with_active_workspace(cx, |workspace, _, cx| workspace.undo(cx));
    });
    cx.on_action(|_: &Redo, cx: &mut App| {
        with_active_workspace(cx, |workspace, _, cx| workspace.redo(cx));
    });

    cx.on_action(|_: &CloseFolder, cx: &mut App| {
        with_active_workspace(cx, |workspace, window, cx| {
            workspace.close_project(window, cx);
        });
    });
    cx.on_action(|_: &CloseWindow, cx: &mut App| {
        with_active_workspace(cx, |_, window, _| window.remove_window());
    });

    cx.on_action(|_: &ToggleProjectPanel, cx: &mut App| {
        with_active_workspace(cx, |workspace, _, cx| workspace.toggle_project_panel(cx));
    });
    cx.on_action(|_: &RunProject, cx: &mut App| {
        with_active_workspace(cx, |workspace, _, cx| workspace.run_project(cx));
    });
    cx.on_action(|_: &StopProject, cx: &mut App| {
        with_active_workspace(cx, |workspace, _, cx| workspace.stop_project(cx));
    });
    cx.on_action(|_: &ToggleOutput, cx: &mut App| {
        with_active_workspace(cx, |workspace, _, cx| workspace.toggle_output(cx));
    });
    cx.on_action(|_: &ToggleStatusBar, cx: &mut App| {
        with_active_workspace(cx, |workspace, _, cx| workspace.toggle_status_bar(cx));
    });

    cx.on_action(|_: &RevealInFinder, cx: &mut App| {
        let path = active_workspace_path(cx);
        if let Some(path) = path {
            cx.reveal_path(&path);
        }
    });

    cx.on_action(|_: &Minimize, cx: &mut App| {
        with_active_workspace(cx, |_, window, _| window.minimize_window());
    });
    cx.on_action(|_: &Zoom, cx: &mut App| {
        with_active_workspace(cx, |_, window, _| window.zoom_window());
    });
}

/// The default keymap. Every accelerator shown in the menu bar comes from
/// here: an action without a binding renders as a menu item with no shortcut.
pub fn key_bindings() -> Vec<KeyBinding> {
    vec![
        KeyBinding::new("cmd-q", Quit, None),
        KeyBinding::new("cmd-h", HideApp, None),
        KeyBinding::new("cmd-alt-h", HideOthers, None),
        KeyBinding::new("cmd-shift-n", NewWindow, None),
        KeyBinding::new("cmd-o", OpenFolder, None),
        KeyBinding::new("cmd-shift-p", NewProject, None),
        KeyBinding::new("cmd-n", NewView, None),
        KeyBinding::new("cmd-s", Save, None),
        KeyBinding::new("cmd-shift-backspace", DeleteNode, None),
        KeyBinding::new("cmd-shift-w", CloseFolder, None),
        KeyBinding::new("cmd-w", CloseWindow, None),
        KeyBinding::new("cmd-z", Undo, None),
        KeyBinding::new("cmd-shift-z", Redo, None),
        KeyBinding::new("cmd-x", Cut, None),
        KeyBinding::new("cmd-c", Copy, None),
        KeyBinding::new("cmd-v", Paste, None),
        KeyBinding::new("cmd-a", SelectAll, None),
        KeyBinding::new("cmd-b", ToggleProjectPanel, None),
        KeyBinding::new("cmd-r", RunProject, None),
        KeyBinding::new("cmd-.", StopProject, None),
        KeyBinding::new("cmd-j", ToggleOutput, None),
        KeyBinding::new("cmd-alt-r", RevealInFinder, None),
        KeyBinding::new("cmd-m", Minimize, None),
    ]
}

/// Shows the native directory picker and opens the chosen folder.
///
/// `prompt_for_paths` answers on a channel, so the result has three layers to
/// unwrap: a closed channel, a platform error, and `None` for a cancelled
/// dialog. All three mean "do nothing".
fn open_folder(_: &OpenFolder, cx: &mut App) {
    let paths = cx.prompt_for_paths(PathPromptOptions {
        files: false,
        directories: true,
        multiple: false,
        prompt: Some("Ouvrir".into()),
    });

    cx.spawn(async move |cx: &mut AsyncApp| {
        if let Ok(Ok(Some(paths))) = paths.await
            && let Some(path) = paths.into_iter().next()
        {
            cx.update(|cx| workspace::open_folder(path, cx)).ok();
        }
    })
    .detach();
}

/// Asks for a location, scaffolds a project there and opens it.
fn new_project(_: &NewProject, cx: &mut App) {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/".into());
    let path = cx.prompt_for_new_path(std::path::Path::new(&home), Some("mon_app"));

    cx.spawn(async move |cx: &mut AsyncApp| {
        let Ok(Ok(Some(path))) = path.await else {
            return;
        };
        cx.update(|cx| {
            let name = path
                .file_name()
                .map(|name| name.to_string_lossy().into_owned())
                .unwrap_or_else(|| "mon_app".into());
            match crate::scaffold::create_project(&path, &name) {
                Ok(()) => workspace::open_folder(path, cx),
                Err(error) => eprintln!("création du projet impossible : {error}"),
            }
        })
        .ok();
    })
    .detach();
}

/// Runs `f` against the workspace of the frontmost window, if there is one.
fn with_active_workspace<R>(
    cx: &mut App,
    f: impl FnOnce(&mut Workspace, &mut gpui::Window, &mut gpui::Context<Workspace>) -> R,
) -> Option<R> {
    workspace::with_active(cx, f)
}

/// Absolute path of the frontmost window's project, if it has one.
fn active_workspace_path(cx: &mut App) -> Option<std::path::PathBuf> {
    workspace::with_active(cx, |workspace, _, _| {
        workspace.project().map(|project| project.root.clone())
    })
    .flatten()
}

fn log_about(cx: &mut App) {
    let _ = cx;
    println!(
        "maxx {} — built on GPUI {}",
        env!("CARGO_PKG_VERSION"),
        "0.2.2"
    );
}
