//! Application actions, their global handlers and the default keymap.
//!
//! Every handler is registered on the `App` rather than on the workspace view:
//! menu items dispatch actions through the focused element path, and keeping
//! the handlers global means a menu item stays enabled regardless of which
//! element currently holds focus. Handlers that need a window resolve it from
//! [`App::active_window`].

use gpui::{Action, App, AsyncApp, KeyBinding, PathPromptOptions, actions};

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
        NewSidebarProject,
        NewSettingsProject,
        NewListDetailProject,
        NewFormProject,
        NewDashboardProject,
        NewWizardProject,
        NewUtilityProject,
        NewEditorProject,
        NewView,
        AddSystemModule,
        AddSettingsModule,
        AddComponentsModule,
        AddThemeModule,
        AddAssetsModule,
        AddWindowModule,
        UpdateModules,
        NoRecentProject,
        ClearRecentProjects,
        OpenFolder,
        OpenDemo,
        AdoptView,
        SetEntryView,
        ReloadView,
        OverwriteFile,
        Save,
        DeleteNode,
        DeleteFile,
        CloseFolder,
        CloseWindow,
        // The tab strip's own, reached by a right click on a tab and from the
        // File menu — which is what puts them in `⌘K` as well.
        CloseOtherTabs,
        CloseTabsToTheRight,
        RevealTabInProject,
        CopyTabPath,
        // Edit menu
        Undo,
        Redo,
        Cut,
        Copy,
        Paste,
        SelectAll,
        ToggleTheme,
        TogglePalette,
        QuickOpen,
        NextTab,
        PreviousTab,
        LastView,
        PaletteUp,
        PaletteDown,
        PaletteClose,
        DuplicateNode,
        WrapInColumn,
        WrapInRow,
        Unwrap,
        CopyNode,
        PasteNode,
        AddMenu,
        AddMenuEntry,
        AddMenuSeparator,
        AddSubmenu,
        DeleteMenuEntry,
        MoveMenuUp,
        MoveMenuDown,
        MoveNodeUp,
        MoveNodeDown,
        // The palette's own three. They carry no menu entry on purpose: each
        // one acts on the row a right click lit, and from `⌘K` — where no row
        // was ever right-clicked — all three could only answer that they have
        // nothing to insert.
        InsertBefore,
        InsertAfter,
        InsertInto,
        // The structure tree's own keys. They carry no menu entry: `↑` on a row
        // is not a command one goes looking for in a bar, and the palette
        // flattens the bar.
        SelectPreviousNode,
        SelectNextNode,
        SelectParentNode,
        SelectChildNode,
        EditNodeText,
        // View menu
        ToggleProjectPanel,
        ToggleStatusBar,
        ToggleOutput,
        ToggleCode,
        ViewCode,
        OpenPreferences,
        OpenMenuBar,
        RemoveMenuBar,
        // Run menu
        RunProject,
        StopProject,
        PrewarmProject,
        // Go menu
        RevealInFinder,
        OpenTerminal,
        OpenInZed,
        OpenProjectInZed,
        OpenHandler,
        // Window menu
        Minimize,
        Zoom,
        // Help menu
        OpenDocs,
    ]
);

/// Opens the recent project at `index` in the list.
///
/// Carries its index rather than its path: an action is a value the menu bar
/// holds on to, and the settings are the one place the paths live. The bar is
/// rebuilt whenever the list changes, so an index never points at the wrong
/// project — and the handler checks it anyway.
#[derive(Clone, PartialEq, Debug, serde::Deserialize, schemars::JsonSchema, Action)]
#[action(namespace = maxx)]
pub struct OpenRecent {
    /// Rank in the recent list, most recent first.
    pub index: usize,
}

/// URL opened by Help > GPUI Documentation.
const DOCS_URL: &str = "https://gpui.rs";

/// Registers the handler for every action reachable from the menu bar.
pub fn register_handlers(cx: &mut App) {
    cx.on_action(|_: &Quit, cx: &mut App| cx.quit());
    cx.on_action(|_: &HideApp, cx: &mut App| cx.hide());
    cx.on_action(|_: &HideOthers, cx: &mut App| cx.hide_other_apps());
    cx.on_action(|_: &ShowAll, cx: &mut App| cx.unhide_other_apps());
    cx.on_action(|_: &About, cx: &mut App| crate::about::open(cx));
    cx.on_action(|_: &OpenDocs, cx: &mut App| cx.open_url(DOCS_URL));

    cx.on_action(|_: &NewWindow, cx: &mut App| {
        workspace::open_workspace_window(None, cx);
    });
    cx.on_action(open_folder);
    cx.on_action(new_project);
    cx.on_action(new_sidebar_project);
    cx.on_action(new_settings_project);
    cx.on_action(new_list_detail_project);
    cx.on_action(new_form_project);
    cx.on_action(new_dashboard_project);
    cx.on_action(new_wizard_project);
    cx.on_action(new_utility_project);
    cx.on_action(new_editor_project);

    cx.on_action(|action: &OpenRecent, cx: &mut App| {
        let path = crate::settings::state(cx).recent_projects.get(action.index).cloned();
        // The project may have been moved since the bar was built.
        let Some(path) = path.filter(|path| path.is_dir()) else {
            return;
        };
        // Deferred: `open_folder` reuses the current window through
        // `with_active`, which cannot enter a window update from inside one —
        // called directly it would silently fail to reuse and open a second
        // window every time, leaving the empty one behind.
        cx.defer(move |cx: &mut App| workspace::open_folder(path, cx));
    });
    // No menu entry and no shortcut: the demo is a way *in*, offered on the
    // welcome screen to a hand that has nothing to open yet. An action all the
    // same, because dispatching one is how a button on that screen reaches the
    // deferral `open_folder` needs — reusing the empty window instead of
    // leaving it behind, exactly as `OpenRecent` does above.
    cx.on_action(|_: &OpenDemo, cx: &mut App| {
        let Some(path) = crate::project::demo() else {
            return;
        };
        cx.defer(move |cx: &mut App| workspace::open_folder(path, cx));
    });
    cx.on_action(|_: &ClearRecentProjects, cx: &mut App| {
        crate::settings::update_state(cx, |state| state.recent_projects.clear());
        cx.set_menus(crate::menus::app_menus(cx));
    });
    cx.on_action(|_: &NoRecentProject, _cx: &mut App| {});

    cx.on_action(|_: &NewView, cx: &mut App| {
        with_active_workspace(cx, |workspace, _, cx| workspace.new_view(cx));
    });
    cx.on_action(|_: &SetEntryView, cx: &mut App| {
        with_active_workspace(cx, |workspace, _, cx| workspace.set_entry_view(cx));
    });
    cx.on_action(|_: &AdoptView, cx: &mut App| {
        with_active_workspace(cx, |workspace, _, cx| workspace.adopt_view(cx));
    });
    cx.on_action(|_: &ReloadView, cx: &mut App| {
        with_active_workspace(cx, |workspace, _, cx| workspace.reload_view(cx));
    });
    cx.on_action(|_: &OverwriteFile, cx: &mut App| {
        with_active_workspace(cx, |workspace, _, cx| workspace.overwrite_view(cx));
    });
    cx.on_action(|_: &Save, cx: &mut App| {
        with_active_workspace(cx, |workspace, _, cx| workspace.save_view(cx));
    });
    cx.on_action(|_: &AddComponentsModule, cx: &mut App| {
        with_active_workspace(cx, |workspace, _, cx| workspace.add_components_module(cx));
    });
    cx.on_action(|_: &AddThemeModule, cx: &mut App| {
        with_active_workspace(cx, |workspace, _, cx| workspace.add_theme_module(cx));
    });
    cx.on_action(|_: &ToggleTheme, cx: &mut App| {
        // From the menu, the toggle is a round trip between the two modes, and
        // it leaves `System` behind: asking explicitly for one of the two is no
        // longer wanting to follow.
        let dark = !crate::theme::is_dark();
        crate::settings::update_prefs(cx, |prefs| {
            prefs.theme = if dark { "dark".into() } else { "light".into() };
        });
        crate::apply_theme(cx);
        crate::workspace::notify_all(cx);
    });
    cx.on_action(|_: &TogglePalette, cx: &mut App| {
        with_active_workspace(cx, |workspace, window, cx| workspace.toggle_palette(window, cx));
    });
    cx.on_action(|_: &NextTab, cx: &mut App| {
        with_active_workspace(cx, |workspace, _, cx| workspace.step_view(true, cx));
    });
    cx.on_action(|_: &PreviousTab, cx: &mut App| {
        with_active_workspace(cx, |workspace, _, cx| workspace.step_view(false, cx));
    });
    cx.on_action(|_: &LastView, cx: &mut App| {
        with_active_workspace(cx, |workspace, _, cx| workspace.last_view(cx));
    });
    cx.on_action(|_: &QuickOpen, cx: &mut App| {
        with_active_workspace(cx, |workspace, window, cx| workspace.quick_open(window, cx));
    });
    cx.on_action(|_: &PaletteUp, cx: &mut App| {
        with_active_workspace(cx, |workspace, _, cx| workspace.move_palette(false, cx));
    });
    cx.on_action(|_: &PaletteDown, cx: &mut App| {
        with_active_workspace(cx, |workspace, _, cx| workspace.move_palette(true, cx));
    });
    cx.on_action(|_: &PaletteClose, cx: &mut App| {
        with_active_workspace(cx, |workspace, _, cx| workspace.close_palette(cx));
    });
    cx.on_action(|_: &DuplicateNode, cx: &mut App| {
        with_active_workspace(cx, |workspace, _, cx| workspace.duplicate_selected(cx));
    });
    cx.on_action(|_: &WrapInColumn, cx: &mut App| {
        with_active_workspace(cx, |workspace, _, cx| workspace.wrap_selected("column", cx));
    });
    cx.on_action(|_: &WrapInRow, cx: &mut App| {
        with_active_workspace(cx, |workspace, _, cx| workspace.wrap_selected("row", cx));
    });
    cx.on_action(|_: &Unwrap, cx: &mut App| {
        with_active_workspace(cx, |workspace, _, cx| workspace.unwrap_selected(cx));
    });
    cx.on_action(|_: &CopyNode, cx: &mut App| {
        with_active_workspace(cx, |workspace, _, cx| workspace.copy_selection(cx));
    });
    cx.on_action(|_: &PasteNode, cx: &mut App| {
        with_active_workspace(cx, |workspace, _, cx| workspace.paste_node(cx));
    });
    cx.on_action(|_: &DeleteNode, cx: &mut App| {
        with_active_workspace(cx, |workspace, _, cx| workspace.delete_selected(cx));
    });
    cx.on_action(|_: &DeleteFile, cx: &mut App| {
        with_active_workspace(cx, |workspace, _, cx| workspace.delete_selected_entry(cx));
    });
    cx.on_action(|_: &AddMenu, cx: &mut App| {
        with_active_workspace(cx, |workspace, _, cx| workspace.add_menu(cx));
    });
    cx.on_action(|_: &AddMenuEntry, cx: &mut App| {
        with_active_workspace(cx, |workspace, _, cx| workspace.add_menu_item(false, cx));
    });
    cx.on_action(|_: &AddMenuSeparator, cx: &mut App| {
        with_active_workspace(cx, |workspace, _, cx| workspace.add_menu_item(true, cx));
    });
    cx.on_action(|_: &AddSubmenu, cx: &mut App| {
        with_active_workspace(cx, |workspace, _, cx| workspace.add_submenu(cx));
    });
    cx.on_action(|_: &DeleteMenuEntry, cx: &mut App| {
        with_active_workspace(cx, |workspace, _, cx| workspace.remove_menu_selection(cx));
    });
    cx.on_action(|_: &InsertBefore, cx: &mut App| {
        with_active_workspace(cx, |workspace, _, cx| {
            workspace.insert_from_palette(workspace::Insert::Before, cx);
        });
    });
    cx.on_action(|_: &InsertAfter, cx: &mut App| {
        with_active_workspace(cx, |workspace, _, cx| {
            workspace.insert_from_palette(workspace::Insert::After, cx);
        });
    });
    cx.on_action(|_: &InsertInto, cx: &mut App| {
        with_active_workspace(cx, |workspace, _, cx| {
            workspace.insert_from_palette(workspace::Insert::Into, cx);
        });
    });
    cx.on_action(|_: &MoveMenuUp, cx: &mut App| {
        with_active_workspace(cx, |workspace, _, cx| workspace.move_menu_selection(true, cx));
    });
    cx.on_action(|_: &MoveMenuDown, cx: &mut App| {
        with_active_workspace(cx, |workspace, _, cx| workspace.move_menu_selection(false, cx));
    });
    cx.on_action(|_: &MoveNodeUp, cx: &mut App| {
        with_active_workspace(cx, |workspace, _, cx| workspace.move_selected_node(true, cx));
    });
    cx.on_action(|_: &MoveNodeDown, cx: &mut App| {
        with_active_workspace(cx, |workspace, _, cx| workspace.move_selected_node(false, cx));
    });
    cx.on_action(|_: &SelectPreviousNode, cx: &mut App| {
        with_active_workspace(cx, |workspace, _, cx| workspace.step_selection(false, cx));
    });
    cx.on_action(|_: &SelectNextNode, cx: &mut App| {
        with_active_workspace(cx, |workspace, _, cx| workspace.step_selection(true, cx));
    });
    cx.on_action(|_: &SelectParentNode, cx: &mut App| {
        with_active_workspace(cx, |workspace, _, cx| workspace.step_depth(false, cx));
    });
    cx.on_action(|_: &SelectChildNode, cx: &mut App| {
        with_active_workspace(cx, |workspace, _, cx| workspace.step_depth(true, cx));
    });
    cx.on_action(|_: &EditNodeText, cx: &mut App| {
        with_active_workspace(cx, |workspace, window, cx| workspace.focus_prop_text(window, cx));
    });
    cx.on_action(|_: &OpenHandler, cx: &mut App| {
        with_active_workspace(cx, |workspace, _, cx| workspace.open_selected_handler(cx));
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
        with_active_workspace(cx, |workspace, window, cx| {
            // ⌘W closes the front tab first, the window only once there are no
            // views left — the habit every editor gives you.
            if workspace.preferences() {
                workspace.close_preferences(cx);
                return;
            }
            if workspace.menu_file().is_some() {
                workspace.close_menu_file(cx);
                return;
            }
            match workspace.active_index() {
                Some(index) => workspace.close_view(index, cx),
                None => window.remove_window(),
            }
        });
    });

    cx.on_action(|_: &CloseOtherTabs, cx: &mut App| {
        with_active_workspace(cx, |workspace, _, cx| workspace.close_other_views(cx));
    });
    cx.on_action(|_: &CloseTabsToTheRight, cx: &mut App| {
        with_active_workspace(cx, |workspace, _, cx| workspace.close_views_to_the_right(cx));
    });
    // The view in front, not the explorer selection: this one hangs off the tab
    // strip, and it is about the tab the right click just lit.
    cx.on_action(|_: &RevealTabInProject, cx: &mut App| {
        with_active_workspace(cx, |workspace, _, cx| workspace.reveal_view_in_project(cx));
    });
    cx.on_action(|_: &CopyTabPath, cx: &mut App| {
        with_active_workspace(cx, |workspace, _, cx| workspace.copy_view_path(cx));
    });

    cx.on_action(|_: &ToggleProjectPanel, cx: &mut App| {
        with_active_workspace(cx, |workspace, _, cx| workspace.toggle_project_panel(cx));
    });
    cx.on_action(|_: &RunProject, cx: &mut App| {
        with_active_workspace(cx, |workspace, _, cx| workspace.run_project(cx));
    });
    cx.on_action(|_: &PrewarmProject, cx: &mut App| {
        with_active_workspace(cx, |workspace, _, cx| workspace.prewarm_project(cx));
    });
    cx.on_action(|_: &StopProject, cx: &mut App| {
        with_active_workspace(cx, |workspace, _, cx| workspace.stop_project(cx));
    });
    cx.on_action(|_: &ToggleOutput, cx: &mut App| {
        with_active_workspace(cx, |workspace, _, cx| workspace.toggle_output(cx));
    });
    cx.on_action(|_: &ToggleCode, cx: &mut App| {
        with_active_workspace(cx, |workspace, _, cx| workspace.toggle_code(cx));
    });
    // The explorer selection, not the active view: this one hangs off the right
    // click, and it is about the file the pointer is on.
    cx.on_action(|_: &ViewCode, cx: &mut App| {
        let path = selected_entry_path(cx);
        if let Some(path) = path {
            with_active_workspace(cx, |workspace, _, cx| workspace.open_code(path, cx));
        }
    });
    cx.on_action(|_: &AddSystemModule, cx: &mut App| {
        with_active_workspace(cx, |workspace, _, cx| workspace.add_system_module(cx));
    });
    cx.on_action(|_: &AddSettingsModule, cx: &mut App| {
        with_active_workspace(cx, |workspace, _, cx| workspace.add_settings_module(cx));
    });
    cx.on_action(|_: &AddAssetsModule, cx: &mut App| {
        with_active_workspace(cx, |workspace, _, cx| workspace.add_assets_module(cx));
    });
    cx.on_action(|_: &AddWindowModule, cx: &mut App| {
        with_active_workspace(cx, |workspace, _, cx| workspace.add_window_module(cx));
    });
    cx.on_action(|_: &UpdateModules, cx: &mut App| {
        with_active_workspace(cx, |workspace, _, cx| workspace.update_modules(cx));
    });
    cx.on_action(|_: &OpenPreferences, cx: &mut App| {
        with_active_workspace(cx, |workspace, _, cx| workspace.toggle_preferences(cx));
    });
    cx.on_action(|_: &OpenMenuBar, cx: &mut App| {
        with_active_workspace(cx, |workspace, _, cx| workspace.open_menu_bar(cx));
    });
    cx.on_action(|_: &RemoveMenuBar, cx: &mut App| {
        with_active_workspace(cx, |workspace, _, cx| workspace.remove_menu_bar(cx));
    });
    cx.on_action(|_: &ToggleStatusBar, cx: &mut App| {
        with_active_workspace(cx, |workspace, _, cx| workspace.toggle_status_bar(cx));
    });

    cx.on_action(|_: &RevealInFinder, cx: &mut App| {
        let path = selected_entry_path(cx);
        if let Some(path) = path {
            cx.reveal_path(&path);
        }
    });

    cx.on_action(|_: &OpenTerminal, cx: &mut App| {
        if let Some(path) = active_workspace_path(cx) {
            crate::tools::open_terminal(cx, &path);
        }
    });
    cx.on_action(|_: &OpenInZed, cx: &mut App| {
        with_active_workspace(cx, |workspace, _, cx| workspace.open_in_editor(cx));
    });
    cx.on_action(|_: &OpenProjectInZed, cx: &mut App| {
        if let Some(path) = active_workspace_path(cx) {
            crate::tools::open_in_editor(cx, &path, None);
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
        KeyBinding::new("cmd-shift-r", ReloadView, None),
        KeyBinding::new("cmd-ctrl-up", MoveMenuUp, None),
        KeyBinding::new("cmd-ctrl-down", MoveMenuDown, None),
        KeyBinding::new("cmd-k", TogglePalette, None),
        KeyBinding::new("cmd-p", QuickOpen, None),
        // Zed's own three, and for once maxx has nothing to choose: a hand
        // coming from there reaches for these without looking.
        KeyBinding::new("cmd-alt-right", NextTab, None),
        KeyBinding::new("cmd-alt-left", PreviousTab, None),
        KeyBinding::new("ctrl-tab", LastView, None),
        // These three hold in the palette only: `escape`, `up` and `down` taken
        // globally would be taken away from the rest of the interface.
        KeyBinding::new("escape", PaletteClose, Some("Palette")),
        KeyBinding::new("up", PaletteUp, Some("Palette")),
        KeyBinding::new("down", PaletteDown, Some("Palette")),
        // The structure tree's own, and they hold nowhere else: `up`, `down`,
        // `left`, `right`, `enter` and `backspace` taken globally would be taken
        // away from every text box in the window. Same shape as the palette's
        // three, and the same reason.
        KeyBinding::new("up", SelectPreviousNode, Some("Tree")),
        KeyBinding::new("down", SelectNextNode, Some("Tree")),
        KeyBinding::new("left", SelectParentNode, Some("Tree")),
        KeyBinding::new("right", SelectChildNode, Some("Tree")),
        KeyBinding::new("enter", EditNodeText, Some("Tree")),
        KeyBinding::new("backspace", DeleteNode, Some("Tree")),
        KeyBinding::new("delete", DeleteNode, Some("Tree")),
        KeyBinding::new("alt-up", MoveNodeUp, Some("Tree")),
        KeyBinding::new("alt-down", MoveNodeDown, Some("Tree")),
        // The same two, unscoped, and they are what the Edit menu shows. Not a
        // convenience: macOS reads a menu item's key equivalent out of the
        // keymap and falls back to the *first* binding when none of them holds
        // in a default context — so an entry whose only binding carries a
        // predicate would display `⌥↑` and let the system fire it from inside a
        // text field, where `⌥↑` is a caret move. A binding that holds
        // everywhere is what the menu is allowed to promise.
        KeyBinding::new("cmd-alt-up", MoveNodeUp, None),
        KeyBinding::new("cmd-alt-down", MoveNodeDown, None),
        KeyBinding::new("cmd-d", DuplicateNode, None),
        KeyBinding::new("cmd-alt-g", WrapInColumn, None),
        KeyBinding::new("cmd-alt-shift-g", WrapInRow, None),
        KeyBinding::new("cmd-alt-u", Unwrap, None),
        KeyBinding::new("cmd-alt-c", CopyNode, None),
        KeyBinding::new("cmd-alt-v", PasteNode, None),
        KeyBinding::new("cmd-shift-backspace", DeleteNode, None),
        KeyBinding::new("cmd-shift-w", CloseFolder, None),
        KeyBinding::new("cmd-w", CloseWindow, None),
        KeyBinding::new("cmd-z", Undo, None),
        KeyBinding::new("cmd-shift-z", Redo, None),
        KeyBinding::new("cmd-x", Cut, None),
        KeyBinding::new("cmd-c", Copy, None),
        KeyBinding::new("cmd-v", Paste, None),
        KeyBinding::new("cmd-a", SelectAll, None),
        KeyBinding::new("cmd-,", OpenPreferences, None),
        KeyBinding::new("cmd-b", ToggleProjectPanel, None),
        KeyBinding::new("cmd-r", RunProject, None),
        KeyBinding::new("cmd-.", StopProject, None),
        KeyBinding::new("cmd-j", ToggleOutput, None),
        KeyBinding::new("cmd-e", ToggleCode, None),
        KeyBinding::new("cmd-alt-r", RevealInFinder, None),
        KeyBinding::new("cmd-alt-t", OpenTerminal, None),
        KeyBinding::new("cmd-alt-z", OpenInZed, None),
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
        prompt: Some(crate::tr("designer.choose")),
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
    scaffold_project(crate::scaffold::Template::Empty, cx);
}

/// The same, in the shape of a sidebar and the view of the moment.
fn new_sidebar_project(_: &NewSidebarProject, cx: &mut App) {
    scaffold_project(crate::scaffold::Template::Sidebar, cx);
}

/// The same again, with the settings module and a screen that reads it.
fn new_settings_project(_: &NewSettingsProject, cx: &mut App) {
    scaffold_project(crate::scaffold::Template::Settings, cx);
}

/// A list, and a panel showing the row that is selected.
fn new_list_detail_project(_: &NewListDetailProject, cx: &mut App) {
    scaffold_project(crate::scaffold::Template::ListDetail, cx);
}

/// Fields that are really typed into, and a button that reads them.
fn new_form_project(_: &NewFormProject, cx: &mut App) {
    scaffold_project(crate::scaffold::Template::Form, cx);
}

/// A header, a grid of cards, and the numbers they carry.
fn new_dashboard_project(_: &NewDashboardProject, cx: &mut App) {
    scaffold_project(crate::scaffold::Template::Dashboard, cx);
}

/// Steps, an indicator, and the two buttons that move between them.
fn new_wizard_project(_: &NewWizardProject, cx: &mut App) {
    scaffold_project(crate::scaffold::Template::Wizard, cx);
}

/// One compact window and one job, with no sidebar around it.
fn new_utility_project(_: &NewUtilityProject, cx: &mut App) {
    scaffold_project(crate::scaffold::Template::Utility, cx);
}

/// A strip of tabs, a text area, and a status bar.
fn new_editor_project(_: &NewEditorProject, cx: &mut App) {
    scaffold_project(crate::scaffold::Template::Editor, cx);
}

/// Asks for a location, writes `template` there and opens it.
fn scaffold_project(template: crate::scaffold::Template, cx: &mut App) {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/".into());
    let path = cx.prompt_for_new_path(
        std::path::Path::new(&home),
        Some(&crate::tr("prefs.default_project_name")),
    );

    cx.spawn(async move |cx: &mut AsyncApp| {
        let Ok(Ok(Some(path))) = path.await else {
            return;
        };
        cx.update(|cx| {
            let name = crate::scaffold::project_name(&path);
            match crate::scaffold::create_project(&path, &name, template) {
                Ok(()) => {
                    workspace::open_folder(path, cx);
                    // The dependency tree costs minutes the first time; pay it
                    // now, while there is drawing to do.
                    workspace::with_active(cx, |workspace, _, cx| workspace.prewarm_project(cx));
                }
                Err(error) => eprintln!("the project could not be created: {error}"),
            }
        })
        .ok();
    })
    .detach();
}

/// Runs `f` against the workspace of the frontmost window, if there is one.
pub(crate) fn with_active_workspace(
    cx: &mut App,
    f: impl FnOnce(&mut Workspace, &mut gpui::Window, &mut gpui::Context<Workspace>) + 'static,
) {
    workspace::defer_active(cx, f);
}

/// Absolute path of the entry the project panel is on, falling back to the
/// project root.
fn selected_entry_path(cx: &mut App) -> Option<std::path::PathBuf> {
    workspace::read_active(cx, |workspace| workspace.selected_entry()).flatten()
}

/// Absolute path of the frontmost window's project, if it has one.
fn active_workspace_path(cx: &mut App) -> Option<std::path::PathBuf> {
    workspace::read_active(cx, |workspace| workspace.project().map(|project| project.root.clone()))
        .flatten()
}
