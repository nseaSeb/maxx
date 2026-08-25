//! The native menu bar, laid out the way Zed lays out its own.

use gpui::{App, Menu, MenuItem, OsAction, SystemMenuType};

use crate::actions::*;

/// The recent projects, most recent first.
///
/// An entry carries the index rather than the path: a gpui action is a value
/// the menu bar keeps, and the settings are the one place the paths live.
fn recent_projects_menu(cx: &App) -> MenuItem {
    let recent = &crate::settings::state(cx).recent_projects;
    if recent.is_empty() {
        // A submenu with nothing in it looks broken; a disabled-looking entry
        // that does nothing says what is going on.
        return MenuItem::action("Aucun projet récent", NoRecentProject);
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
        .chain([MenuItem::separator(), MenuItem::action("Vider la liste", ClearRecentProjects)])
        .collect();

    MenuItem::submenu(Menu { name: "Ouvrir un élément récent".into(), items })
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
                MenuItem::action("À propos de maxx", About),
                MenuItem::separator(),
                MenuItem::action("Réglages…", OpenPreferences),
                MenuItem::separator(),
                MenuItem::os_submenu("Services", SystemMenuType::Services),
                MenuItem::separator(),
                MenuItem::action("Masquer maxx", HideApp),
                MenuItem::action("Masquer les autres", HideOthers),
                MenuItem::action("Tout afficher", ShowAll),
                MenuItem::separator(),
                MenuItem::action("Quitter maxx", Quit),
            ],
        },
        Menu {
            name: "Fichier".into(),
            items: vec![
                MenuItem::action("Nouveau projet…", NewProject),
                MenuItem::action("Nouvelle vue…", NewView),
                MenuItem::action("Nouvelle fenêtre", NewWindow),
                MenuItem::separator(),
                MenuItem::submenu(Menu {
                    name: "Ajouter au projet".into(),
                    items: vec![
                        MenuItem::action("La barre de menus", OpenMenuBar),
                        MenuItem::action("Le module système", AddSystemModule),
                        MenuItem::action("Les réglages", AddSettingsModule),
                        MenuItem::separator(),
                        MenuItem::action("Mettre à jour les modules", UpdateModules),
                    ],
                }),
                MenuItem::separator(),
                MenuItem::action("Ouvrir un dossier…", OpenFolder),
                recent_projects_menu(cx),
                MenuItem::separator(),
                MenuItem::action("Enregistrer", Save),
                MenuItem::action("Recharger la vue", ReloadView),
                MenuItem::action("Écraser le fichier", OverwriteFile),
                MenuItem::separator(),
                MenuItem::action("Adopter cette vue", AdoptView),
                MenuItem::separator(),
                MenuItem::action("Fermer le projet", CloseFolder),
                MenuItem::action("Fermer la vue", CloseWindow),
            ],
        },
        Menu {
            name: "Édition".into(),
            items: vec![
                MenuItem::os_action("Annuler", Undo, OsAction::Undo),
                MenuItem::os_action("Rétablir", Redo, OsAction::Redo),
                MenuItem::separator(),
                MenuItem::os_action("Couper", Cut, OsAction::Cut),
                MenuItem::os_action("Copier", Copy, OsAction::Copy),
                MenuItem::os_action("Coller", Paste, OsAction::Paste),
                MenuItem::os_action("Tout sélectionner", SelectAll, OsAction::SelectAll),
                MenuItem::separator(),
                MenuItem::action("Ajouter un menu", AddMenu),
                MenuItem::action("Ajouter une entrée", AddMenuEntry),
                MenuItem::action("Ajouter un séparateur", AddMenuSeparator),
                MenuItem::separator(),
                MenuItem::action("Supprimer le nœud", DeleteNode),
                MenuItem::action("Supprimer le fichier", DeleteFile),
            ],
        },
        Menu {
            name: "Affichage".into(),
            items: vec![
                MenuItem::action("Panneau du projet", ToggleProjectPanel),
                MenuItem::action("Sortie", ToggleOutput),
                MenuItem::action("Barre d'état", ToggleStatusBar),
                MenuItem::separator(),
                MenuItem::action("Barre de menus du projet", OpenMenuBar),
                MenuItem::action("Retirer la barre de menus", RemoveMenuBar),
            ],
        },
        Menu {
            name: "Exécution".into(),
            items: vec![
                MenuItem::action("Lancer le projet", RunProject),
                MenuItem::action("Arrêter", StopProject),
                MenuItem::separator(),
                MenuItem::action("Préparer les dépendances", PrewarmProject),
            ],
        },
        Menu {
            name: "Aller".into(),
            items: vec![
                MenuItem::action("Révéler dans le Finder", RevealInFinder),
                MenuItem::action("Ouvrir dans le Terminal", OpenTerminal),
                MenuItem::action(
                    format!("Ouvrir le fichier dans {}", crate::tools::editor_label(cx)),
                    OpenInZed,
                ),
                MenuItem::action(
                    format!("Ouvrir le projet dans {}", crate::tools::editor_label(cx)),
                    OpenProjectInZed,
                ),
            ],
        },
        Menu {
            name: "Fenêtre".into(),
            items: vec![MenuItem::action("Réduire", Minimize), MenuItem::action("Zoom", Zoom)],
        },
        Menu { name: "Aide".into(), items: vec![MenuItem::action("Documentation GPUI", OpenDocs)] },
    ]
}
