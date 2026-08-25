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
        shortcut: None,
    });
    fichier.items.push(ItemDef::Separator);
    fichier.items.push(ItemDef::Action {
        label: "Quitter".into(),
        action: "Quitter".into(),
        os_action: None,
        shortcut: None,
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
        shortcut: None,
    });
    interne.items.push(ItemDef::Action {
        label: "Deux".into(),
        action: "Deux".into(),
        os_action: None,
        shortcut: None,
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

#[test]
fn an_action_written_inside_a_submenu_is_declared_too() {
    // Le défaut que ce test verrouille : une action ajoutée dans un sous-menu
    // était écrite dans le fichier mais jamais déclarée dans `actions!` ni
    // câblée — le projet généré ne compilait plus sur un nom que maxx venait
    // lui-même d'écrire.
    let mut menus = barre();
    let mut interne = MenuDef::named("Récents");
    interne.items.push(ItemDef::Action {
        label: "Rouvrir".into(),
        action: "RouvrirRecent".into(),
        os_action: None,
        shortcut: None,
    });
    menus.menus[0].items[1] = ItemDef::Submenu(interne);

    let actions = menus.actions();
    assert!(actions.contains(&"RouvrirRecent".to_string()), "{actions:?}");
    // Et celles du premier niveau n'ont pas disparu au passage.
    assert!(actions.contains(&"Nouveau".to_string()), "{actions:?}");
    assert!(actions.contains(&"Quitter".to_string()), "{actions:?}");
}

#[test]
fn a_qualified_or_system_action_is_still_left_alone_inside_a_submenu() {
    let mut menus = barre();
    let mut interne = MenuDef::named("Récents");
    // Une action d'un autre module : pas à nous de la déclarer.
    interne.items.push(ItemDef::Action {
        label: "Ailleurs".into(),
        action: "autre::Action".into(),
        os_action: None,
        shortcut: None,
    });
    // Une action système : la déclarer masquerait ce qu'elle délègue.
    interne.items.push(ItemDef::Action {
        label: "Copier".into(),
        action: "Copy".into(),
        os_action: Some("Copy".into()),
        shortcut: None,
    });
    menus.menus[0].items[1] = ItemDef::Submenu(interne);

    let actions = menus.actions();
    assert!(!actions.iter().any(|action| action.contains("::")), "{actions:?}");
    assert!(!actions.contains(&"Copy".to_string()), "{actions:?}");
}

#[test]
fn a_stale_selection_does_not_interrupt_the_process() {
    let mut menus = barre();
    // Un indice qui n'existe plus : ne rien faire, pas paniquer.
    menus.selected = Some(Selection::Item(0, 9));
    assert!(!menus.move_selected(true));
    assert!(!menus.move_selected(false));
    assert_eq!(menus.menus[0].items.len(), 3);
}

/// Le gabarit d'un fichier de menus, avec sa fonction `key_bindings`.
fn fichier_complet(bindings: &str) -> String {
    format!(
        r#"use gpui::{{App, Menu, MenuItem, actions}};

actions!(app, [Nouveau, Quitter]);

pub fn key_bindings() -> Vec<gpui::KeyBinding> {{
    use gpui::KeyBinding;
    vec![
{bindings}    ]
}}

pub fn app_menus() -> Vec<Menu> {{
    // maxx:begin
    vec![Menu {{
        name: "Fichier".into(),
        items: vec![MenuItem::action("Nouveau", Nouveau)],
    }}]
    // maxx:end
}}
"#
    )
}

#[test]
fn a_shortcut_travels_with_its_entry_and_is_written_at_save() {
    let path = std::env::temp_dir().join("maxx_raccourci.rs");
    let source = fichier_complet("        KeyBinding::new(\"cmd-q\", Quitter, None),\n");
    std::fs::write(&path, &source).unwrap();

    let mut menus = MenuFile::load(&path).expect("doit se relire");

    // Lu à l'ouverture et posé sur l'entrée, pas cherché dans le fichier à
    // chaque affichage.
    let ItemDef::Action { shortcut, .. } = &menus.menus[0].items[0] else {
        panic!("la première entrée est une action");
    };
    assert_eq!(shortcut.as_deref(), None, "Nouveau n'a pas de raccourci au départ");

    // Posé sur le modèle : rien n'a encore touché le disque.
    let ItemDef::Action { shortcut, .. } = &mut menus.menus[0].items[0] else { unreachable!() };
    *shortcut = Some("cmd-n".into());
    assert_eq!(std::fs::read_to_string(&path).unwrap(), source, "rien avant ⌘S");

    menus.save(false).expect("l'enregistrement doit passer");
    let ecrit = std::fs::read_to_string(&path).unwrap();
    assert!(ecrit.contains("KeyBinding::new(\"cmd-n\", Nouveau, None),"), "{ecrit}");
    // Celui d'à côté n'a pas bougé.
    assert!(ecrit.contains("KeyBinding::new(\"cmd-q\", Quitter, None),"), "{ecrit}");

    // Relire rend le raccourci à son entrée.
    let relu = MenuFile::load(&path).expect("doit se relire");
    let ItemDef::Action { shortcut, .. } = &relu.menus[0].items[0] else { unreachable!() };
    assert_eq!(shortcut.as_deref(), Some("cmd-n"));
}

#[test]
fn removing_a_shortcut_removes_every_line_that_bound_it() {
    // gpui accepte plusieurs frappes pour une action : n'en réécrire qu'une
    // laisserait l'ancienne vivante dans le dos de l'utilisateur.
    let path = std::env::temp_dir().join("maxx_raccourci_double.rs");
    let source = fichier_complet(
        "        KeyBinding::new(\"cmd-n\", Nouveau, None),\n        KeyBinding::new(\"ctrl-n\", Nouveau, None),\n",
    );
    std::fs::write(&path, &source).unwrap();

    let mut menus = MenuFile::load(&path).expect("doit se relire");
    let ItemDef::Action { shortcut, .. } = &mut menus.menus[0].items[0] else { unreachable!() };
    *shortcut = Some("cmd-shift-n".into());
    menus.save(false).expect("l'enregistrement doit passer");

    let ecrit = std::fs::read_to_string(&path).unwrap();
    assert_eq!(ecrit.matches(", Nouveau, None").count(), 1, "{ecrit}");
    assert!(ecrit.contains("cmd-shift-n"), "{ecrit}");
}

#[test]
fn a_file_without_key_bindings_is_left_whole() {
    // Le défaut que ce test verrouille : la source était vidée avant de
    // pouvoir échouer, et tout devenait irrécupérable.
    let path = std::env::temp_dir().join("maxx_sans_bindings.rs");
    let source = r#"use gpui::{Menu, MenuItem};

pub fn app_menus() -> Vec<Menu> {
    // maxx:begin
    vec![Menu { name: "Fichier".into(), items: vec![MenuItem::action("Nouveau", Nouveau)] }]
    // maxx:end
}
"#;
    std::fs::write(&path, source).unwrap();

    let mut menus = MenuFile::load(&path).expect("doit se relire");
    let ItemDef::Action { shortcut, .. } = &mut menus.menus[0].items[0] else { unreachable!() };
    *shortcut = Some("cmd-n".into());
    menus.save(false).expect("l'enregistrement doit passer malgré tout");

    assert!(!menus.source.is_empty(), "la source ne doit jamais être vidée");
    let ecrit = std::fs::read_to_string(&path).unwrap();
    assert!(ecrit.contains("maxx:begin"), "{ecrit}");
    assert!(ecrit.contains("Nouveau"), "{ecrit}");
}

#[test]
fn a_keystroke_gpui_cannot_read_is_refused() {
    // Une frappe illisible fait interrompre `bind_keys` au démarrage :
    // l'application générée refuserait de s'ouvrir.
    for mauvais in ["", "cmd-", "-n", "commande-n", "cmd--n"] {
        assert!(!maxx::menufile::is_keystroke(mauvais), "« {mauvais} » devrait être refusé");
    }
    for bon in ["n", "cmd-n", "cmd-shift-n", "ctrl-alt-delete", "cmd-,", "secondary-a"] {
        assert!(maxx::menufile::is_keystroke(bon), "« {bon} » devrait être accepté");
    }
}

#[test]
fn a_stale_menu_selection_does_not_interrupt_the_process_either() {
    let mut menus = barre();
    menus.selected = Some(Selection::Menu(9));
    assert!(!menus.move_selected(true));
    assert!(!menus.move_selected(false));
    assert_eq!(menus.menus.len(), 2);
}

#[test]
fn a_submenu_inside_a_submenu_is_kept_verbatim() {
    // L'arbre ne descend qu'à deux niveaux : afficher le troisième à moitié
    // donnerait une ligne sans enfants, ni sélectionnable ni supprimable.
    let source = r#"use gpui::{Menu, MenuItem};

pub fn app_menus() -> Vec<Menu> {
    // maxx:begin
    vec![Menu {
        name: "Fichier".into(),
        items: vec![MenuItem::submenu(Menu {
            name: "Récents".into(),
            items: vec![MenuItem::submenu(Menu { name: "Encore".into(), items: vec![] })],
        })],
    }]
    // maxx:end
}
"#;
    let menus = MenuFile::from_source(std::path::PathBuf::from("/tmp/menus.rs"), source.into())
        .expect("doit se relire");
    assert!(
        matches!(menus.menus[0].items[0], ItemDef::Opaque(_)),
        "le sous-menu imbriqué doit rester tel quel : {:?}",
        menus.menus[0].items[0]
    );
    // Et il ressort intact.
    let rendu = fichier_de(&menus.menus);
    assert!(rendu.contains("Encore"), "{rendu}");
}
