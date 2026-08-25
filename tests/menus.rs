//! L'éditeur de menus : réordonner ne doit ni perdre une entrée, ni faire
//! passer une entrée d'un menu à l'autre, ni sortir des bornes.

use maxx::menu_model::{ItemDef, MenuDef};
use maxx::menufile::{MenuFile, Selection};

/// Enveloppe des menus dans le plus petit fichier qui porte une zone gérée.
fn fichier_de(menus: &[MenuDef]) -> String {
    let rendu = maxx::menu_model::render(menus);
    format!(
        "use gpui::{{Menu, MenuItem}};\n\npub fn app_menus() -> Vec<Menu> {{\n    // maxx:begin\n{}\n    // maxx:end\n}}\n",
        rendu.lines().map(|ligne| format!("    {ligne}")).collect::<Vec<_>>().join("\n")
    )
}

fn barre() -> MenuFile {
    let mut fichier = MenuDef::named("Fichier");
    fichier.items.push(ItemDef::Action {
        label: "Nouveau".into(),
        action: "Nouveau".into(),
        os_action: None,
    });
    fichier.items.push(ItemDef::Separator);
    fichier.items.push(ItemDef::Action {
        label: "Quitter".into(),
        action: "Quitter".into(),
        os_action: None,
    });
    let menus = vec![fichier, MenuDef::named("Édition")];

    MenuFile::from_source(std::path::PathBuf::from("/tmp/menus.rs"), fichier_de(&menus))
        .expect("la barre d'essai doit se relire")
}

#[test]
fn an_entry_moves_within_its_own_menu() {
    let mut menus = barre();
    menus.selected = Some(Selection::Item(0, 2));

    assert!(menus.move_selected(true));
    assert_eq!(menus.selected, Some(Selection::Item(0, 1)));
    let labels: Vec<String> = menus.menus[0].items.iter().map(|item| item.label()).collect();
    assert_eq!(labels[1], "Quitter");
    // La sélection suit l'entrée déplacée, sinon on déplace la suivante au coup
    // d'après.
    assert_eq!(menus.selected_item().map(|item| item.label()).as_deref(), Some("Quitter"));
    // Rien n'a été perdu.
    assert_eq!(menus.menus[0].items.len(), 3);
    assert_eq!(menus.menus[1].items.len(), 0);
}

#[test]
fn an_entry_never_leaves_its_menu() {
    let mut menus = barre();

    // En tête, monter ne fait rien — et surtout ne verse pas dans le menu
    // précédent.
    menus.selected = Some(Selection::Item(0, 0));
    assert!(!menus.move_selected(true));
    assert_eq!(menus.selected, Some(Selection::Item(0, 0)));

    // En queue, descendre non plus.
    menus.selected = Some(Selection::Item(0, 2));
    assert!(!menus.move_selected(false));
    assert_eq!(menus.menus[0].items.len(), 3);
    assert_eq!(menus.menus[1].items.len(), 0);
}

#[test]
fn a_menu_moves_among_the_menus() {
    let mut menus = barre();
    menus.selected = Some(Selection::Menu(1));

    assert!(menus.move_selected(true));
    assert_eq!(menus.selected, Some(Selection::Menu(0)));
    assert_eq!(menus.menus[0].name, "Édition");
    assert_eq!(menus.menus[1].name, "Fichier");
    // Le menu emporte ses entrées avec lui.
    assert_eq!(menus.menus[1].items.len(), 3);

    assert!(!menus.move_selected(true));
    assert_eq!(menus.selected, Some(Selection::Menu(0)));
}

#[test]
fn moving_without_a_selection_does_nothing() {
    let mut menus = barre();
    menus.selected = None;
    assert!(!menus.move_selected(true));
    assert!(!menus.move_selected(false));
}

#[test]
fn reordering_survives_being_written_and_read_back() {
    let mut menus = barre();
    menus.selected = Some(Selection::Menu(1));
    menus.move_selected(true);

    // Le rendu puis la relecture doivent rendre le même ordre : déplacer dans
    // l'écran sans que le fichier suive ne servirait à rien.
    let relu =
        MenuFile::from_source(std::path::PathBuf::from("/tmp/menus.rs"), fichier_de(&menus.menus))
            .expect("la barre doit se relire");
    assert_eq!(relu.menus[0].name, "Édition");
    assert_eq!(relu.menus[1].name, "Fichier");
    assert_eq!(relu.menus[1].items.len(), 3);
}

#[test]
fn a_submenu_is_read_written_and_read_back() {
    let source = r#"use gpui::{Menu, MenuItem};

pub fn app_menus() -> Vec<Menu> {
    // maxx:begin
    vec![Menu {
        name: "Fichier".into(),
        items: vec![
            MenuItem::action("Nouveau", Nouveau),
            MenuItem::submenu(Menu {
                name: "Récents".into(),
                items: vec![
                    MenuItem::action("Premier", PremierRecent),
                    MenuItem::separator(),
                ],
            }),
        ],
    }]
    // maxx:end
}
"#;

    let menus = MenuFile::from_source(std::path::PathBuf::from("/tmp/menus.rs"), source.into())
        .expect("la barre doit se relire");

    // Le sous-menu n'est plus un bloc opaque : il a un nom et des entrées.
    let ItemDef::Submenu(interne) = &menus.menus[0].items[1] else {
        panic!("le sous-menu doit être reconnu : {:?}", menus.menus[0].items[1]);
    };
    assert_eq!(interne.name, "Récents");
    assert_eq!(interne.items.len(), 2);

    // Et il repasse par le rendu sans y perdre son contenu.
    let relu =
        MenuFile::from_source(std::path::PathBuf::from("/tmp/menus.rs"), fichier_de(&menus.menus))
            .expect("la barre rendue doit se relire");
    let ItemDef::Submenu(interne) = &relu.menus[0].items[1] else {
        panic!("le sous-menu doit survivre au rendu");
    };
    assert_eq!(interne.name, "Récents");
    assert_eq!(interne.items[0].label(), "Premier");
    assert_eq!(interne.items.len(), 2);
}

#[test]
fn an_entry_of_a_submenu_moves_inside_it() {
    let mut menus = barre();
    // Un sous-menu à la place de la deuxième entrée de Fichier.
    let mut interne = MenuDef::named("Récents");
    interne.items.push(ItemDef::Action {
        label: "Un".into(),
        action: "Un".into(),
        os_action: None,
    });
    interne.items.push(ItemDef::Action {
        label: "Deux".into(),
        action: "Deux".into(),
        os_action: None,
    });
    menus.menus[0].items[1] = ItemDef::Submenu(interne);

    menus.selected = Some(Selection::SubItem(0, 1, 1));
    assert!(menus.move_selected(true));
    assert_eq!(menus.selected, Some(Selection::SubItem(0, 1, 0)));

    let ItemDef::Submenu(interne) = &menus.menus[0].items[1] else {
        panic!("toujours un sous-menu");
    };
    assert_eq!(interne.items[0].label(), "Deux");
    // Rien n'a fui vers le menu qui le contient.
    assert_eq!(menus.menus[0].items.len(), 3);

    // En tête, monter ne fait pas remonter l'entrée hors du sous-menu.
    assert!(!menus.move_selected(true));
    assert_eq!(menus.menus[0].items.len(), 3);
}

#[test]
fn adding_to_a_selected_submenu_goes_inside_it() {
    let mut menus = barre();
    menus.menus[0].items[1] = ItemDef::Submenu(MenuDef::named("Récents"));

    // Sous-menu sélectionné : l'entrée va dedans, pas à côté.
    menus.selected = Some(Selection::Item(0, 1));
    menus.add_item(ItemDef::Separator);
    assert_eq!(menus.selected, Some(Selection::SubItem(0, 1, 0)));
    let ItemDef::Submenu(interne) = &menus.menus[0].items[1] else {
        panic!("toujours un sous-menu");
    };
    assert_eq!(interne.items.len(), 1);
    assert_eq!(menus.menus[0].items.len(), 3);

    // Et supprimer depuis l'intérieur ne retire que l'entrée.
    menus.remove_selected();
    let ItemDef::Submenu(interne) = &menus.menus[0].items[1] else {
        panic!("le sous-menu doit rester");
    };
    assert_eq!(interne.items.len(), 0);
    assert_eq!(menus.menus[0].items.len(), 3);
}
