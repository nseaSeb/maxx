//! The native menu bar, laid out the way Zed lays out its own.

use gpui::{App, Menu, MenuItem, OsAction, SystemMenuType};

use rust_i18n::t;

use crate::actions::*;
use crate::tr;

/// The recent projects, most recent first.
///
/// An entry carries the index rather than the path: a gpui action is a value
/// the menu bar keeps, and the settings are the one place the paths live.
fn recent_projects_menu(cx: &App) -> MenuItem {
    let recent = &crate::settings::state(cx).recent_projects;
    if recent.is_empty() {
        // A submenu with nothing in it looks broken; a disabled-looking entry
        // that does nothing says what is going on.
        return MenuItem::action(tr("menu.no_recent"), NoRecentProject);
    }

    let items = recent
        .iter()
        .enumerate()
        .map(|(index, path)| {
            let label = path
                .file_name()
                .map(|name| name.to_string_lossy().into_owned())
                .unwrap_or_else(|| path.to_string_lossy().into_owned());
            MenuItem::action(label, OpenRecent { index })
        })
        .chain([
            MenuItem::separator(),
            MenuItem::action(tr("menu.clear_recent"), ClearRecentProjects),
        ])
        .collect();

    MenuItem::submenu(Menu { name: tr("menu.open_recent"), items })
}

/// Builds the whole menu bar.
///
/// macOS ignores the name of the first menu and shows the bundle name instead
/// (the binary name when run outside a bundle), so `"maxx"` here is only a
/// label for the other platforms.
///
/// Takes the application because the recent projects live in the settings, and
/// a gpui menu bar is a value handed over once: the whole bar is rebuilt and
/// handed over again whenever that list changes.
pub fn app_menus(cx: &App) -> Vec<Menu> {
    vec![
        Menu {
            name: "maxx".into(),
            items: vec![
                MenuItem::action(tr("menu.about"), About),
                MenuItem::separator(),
                MenuItem::action(tr("menu.preferences"), OpenPreferences),
                MenuItem::separator(),
                MenuItem::os_submenu("Services", SystemMenuType::Services),
                MenuItem::separator(),
                MenuItem::action(tr("menu.hide"), HideApp),
                MenuItem::action(tr("menu.hide_others"), HideOthers),
                MenuItem::action(tr("menu.show_all"), ShowAll),
                MenuItem::separator(),
                MenuItem::action(tr("menu.quit"), Quit),
            ],
        },
        Menu {
            name: tr("menu.file"),
            items: vec![
                MenuItem::action(tr("menu.new_project"), NewProject),
                MenuItem::action(tr("menu.new_view"), NewView),
                MenuItem::action(tr("menu.new_window"), NewWindow),
                MenuItem::separator(),
                MenuItem::submenu(Menu {
                    name: tr("menu.add_to_project"),
                    items: vec![
                        MenuItem::action(tr("menu.add_menu_bar"), OpenMenuBar),
                        MenuItem::action(tr("menu.add_system"), AddSystemModule),
                        MenuItem::action(tr("menu.add_settings"), AddSettingsModule),
                        MenuItem::separator(),
                        MenuItem::action(tr("menu.update_modules"), UpdateModules),
                    ],
                }),
                MenuItem::separator(),
                MenuItem::action(tr("menu.open_folder"), OpenFolder),
                recent_projects_menu(cx),
                MenuItem::separator(),
                MenuItem::action(tr("menu.save"), Save),
                MenuItem::action(tr("menu.reload_view"), ReloadView),
                MenuItem::action(tr("menu.overwrite"), OverwriteFile),
                MenuItem::separator(),
                MenuItem::action(tr("menu.adopt_view"), AdoptView),
                MenuItem::separator(),
                MenuItem::action(tr("menu.close_project"), CloseFolder),
                MenuItem::action(tr("menu.close_view"), CloseWindow),
            ],
        },
        Menu {
            name: tr("menu.edit"),
            items: vec![
                MenuItem::os_action(tr("menu.undo"), Undo, OsAction::Undo),
                MenuItem::os_action(tr("menu.redo"), Redo, OsAction::Redo),
                MenuItem::separator(),
                MenuItem::os_action(tr("menu.cut"), Cut, OsAction::Cut),
                MenuItem::os_action(tr("menu.copy"), Copy, OsAction::Copy),
                MenuItem::os_action(tr("menu.paste"), Paste, OsAction::Paste),
                MenuItem::os_action(tr("menu.select_all"), SelectAll, OsAction::SelectAll),
                MenuItem::separator(),
                MenuItem::action(tr("menu.duplicate_node"), DuplicateNode),
                MenuItem::action(tr("menu.copy_node"), CopyNode),
                MenuItem::action(tr("menu.paste_node"), PasteNode),
                MenuItem::separator(),
                MenuItem::action(tr("menu.add_menu"), AddMenu),
                MenuItem::action(tr("menu.add_entry"), AddMenuEntry),
                MenuItem::action(tr("menu.add_separator"), AddMenuSeparator),
                MenuItem::separator(),
                MenuItem::action(tr("menu.move_up"), MoveMenuUp),
                MenuItem::action(tr("menu.move_down"), MoveMenuDown),
                MenuItem::separator(),
                MenuItem::action(tr("menu.delete_node"), DeleteNode),
                MenuItem::action(tr("menu.delete_file"), DeleteFile),
            ],
        },
        Menu {
            name: tr("menu.view"),
            items: vec![
                MenuItem::action(tr("menu.project_panel"), ToggleProjectPanel),
                MenuItem::action(tr("menu.output"), ToggleOutput),
                MenuItem::action(tr("menu.status_bar"), ToggleStatusBar),
                MenuItem::separator(),
                MenuItem::action(tr("menu.project_menu_bar"), OpenMenuBar),
                MenuItem::action(tr("menu.remove_menu_bar"), RemoveMenuBar),
            ],
        },
        Menu {
            name: tr("menu.run"),
            items: vec![
                MenuItem::action(tr("menu.run_project"), RunProject),
                MenuItem::action(tr("menu.stop"), StopProject),
                MenuItem::separator(),
                MenuItem::action(tr("menu.prewarm"), PrewarmProject),
            ],
        },
        Menu {
            name: tr("menu.go"),
            items: vec![
                MenuItem::action(tr("menu.reveal"), RevealInFinder),
                MenuItem::action(tr("menu.open_terminal"), OpenTerminal),
                MenuItem::action(
                    t!("menu.open_file_in", editor = crate::tools::editor_label(cx)).into_owned(),
                    OpenInZed,
                ),
                MenuItem::action(
                    t!("menu.open_project_in", editor = crate::tools::editor_label(cx))
                        .into_owned(),
                    OpenProjectInZed,
                ),
            ],
        },
        Menu {
            name: tr("menu.window"),
            items: vec![
                MenuItem::action(tr("menu.minimize"), Minimize),
                MenuItem::action(tr("menu.zoom"), Zoom),
            ],
        },
        Menu {
            name: tr("menu.help"),
            items: vec![MenuItem::action(tr("menu.gpui_docs"), OpenDocs)],
        },
    ]
}
