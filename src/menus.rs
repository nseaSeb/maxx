//! The native menu bar, laid out the way Zed lays out its own.

use gpui::{Menu, MenuItem, NoAction, OsAction, SystemMenuType};

use crate::actions::*;

/// Builds the whole menu bar.
///
/// macOS ignores the name of the first menu and shows the bundle name instead
/// (the binary name when run outside a bundle), so `"maxx"` here is only a
/// label for the other platforms.
pub fn app_menus() -> Vec<Menu> {
    vec![
        Menu {
            name: "maxx".into(),
            items: vec![
                MenuItem::action("À propos de maxx", About),
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
                MenuItem::action("Ouvrir un dossier…", OpenFolder),
                MenuItem::action("Ouvrir un élément récent", NoAction),
                MenuItem::separator(),
                MenuItem::action("Enregistrer", Save),
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
                MenuItem::action("Supprimer le nœud", DeleteNode),
            ],
        },
        Menu {
            name: "Affichage".into(),
            items: vec![
                MenuItem::action("Panneau du projet", ToggleProjectPanel),
                MenuItem::action("Sortie", ToggleOutput),
                MenuItem::action("Barre d'état", ToggleStatusBar),
            ],
        },
        Menu {
            name: "Exécution".into(),
            items: vec![
                MenuItem::action("Lancer le projet", RunProject),
                MenuItem::action("Arrêter", StopProject),
            ],
        },
        Menu {
            name: "Aller".into(),
            items: vec![MenuItem::action("Révéler dans le Finder", RevealInFinder)],
        },
        Menu {
            name: "Fenêtre".into(),
            items: vec![
                MenuItem::action("Réduire", Minimize),
                MenuItem::action("Zoom", Zoom),
            ],
        },
        Menu {
            name: "Aide".into(),
            items: vec![MenuItem::action("Documentation GPUI", OpenDocs)],
        },
    ]
}
