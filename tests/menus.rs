//! L'éditeur de menus : réordonner ne doit ni perdre une entrée, ni faire
//! passer une entrée d'un menu à l'autre, ni sortir des bornes.

use maxx::menu_model::{ItemDef, MenuDef};
use maxx::menufile::{Drop, MenuFile, Selection};

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

/// Une entrée traverse d'un menu à l'autre — ce que les deux touches refusent.
#[test]
fn an_entry_can_be_dragged_into_another_menu() {
    let mut menus = barre();
    assert!(menus.move_to(Selection::Item(0, 2), Drop::Item(1, 0)));

    assert_eq!(menus.menus[0].items.len(), 2, "elle a quitté son menu");
    assert_eq!(menus.menus[1].items.len(), 1);
    assert_eq!(menus.menus[1].items[0].label(), "Quitter");
    assert_eq!(menus.selected, Some(Selection::Item(1, 0)), "et la sélection la suit");
}

/// Un menu se réordonne parmi les menus, et pas ailleurs.
#[test]
fn a_menu_is_dragged_among_the_menus() {
    let mut menus = barre();
    assert!(menus.move_to(Selection::Menu(0), Drop::Menu(2)));
    assert_eq!(menus.menus[0].name, "Édition");
    assert_eq!(menus.menus[1].name, "Fichier");
    assert_eq!(menus.selected, Some(Selection::Menu(1)));
}

/// Déposée plus bas dans sa propre liste, l'entrée tient compte de son retrait.
#[test]
fn a_drop_after_the_source_accounts_for_the_removal() {
    let mut menus = barre();
    let labels = |menus: &MenuFile| -> Vec<String> {
        menus.menus[0].items.iter().map(|item| item.label()).collect()
    };
    let avant = labels(&menus);

    // La première entrée passe à la fin : l'index de dépôt vaut 3, mais le
    // retrait a décalé la liste d'un cran.
    assert!(menus.move_to(Selection::Item(0, 0), Drop::Item(0, 3)));
    assert_eq!(labels(&menus), vec![avant[1].clone(), avant[2].clone(), avant[0].clone()]);
}

/// Reposée où elle était, l'entrée ne bouge pas et ne se perd pas.
#[test]
fn a_drop_where_it_already_is_changes_nothing() {
    let mut menus = barre();
    let avant = menus.menus[0].items.clone();

    assert!(!menus.move_to(Selection::Item(0, 1), Drop::Item(0, 1)));
    assert!(!menus.move_to(Selection::Item(0, 1), Drop::Item(0, 2)));
    assert_eq!(menus.menus[0].items, avant, "l'entrée est toujours là, à sa place");
}

/// Trois refus qui sont des règles du modèle, pas des précautions.
#[test]
fn the_three_refusals_of_a_drop() {
    let mut menus = barre();

    // Un menu n'est pas une entrée.
    assert!(!menus.move_to(Selection::Menu(0), Drop::Item(1, 0)));

    // Un sous-menu ne va pas dans un sous-menu : il n'y a qu'un niveau.
    menus.menus[0].items.push(ItemDef::Submenu(MenuDef::named("Récents")));
    let sous_menu = menus.menus[0].items.len() - 1;
    assert!(!menus.move_to(Selection::Item(0, sous_menu), Drop::SubItem(0, sous_menu, 0)));

    // Et rien ne va dans un menu que maxx n'a pas su lire.
    menus.menus.push(MenuDef {
        name: "Dynamique".into(),
        items: Vec::new(),
        opaque: Some("items: construire(),".into()),
    });
    let illisible = menus.menus.len() - 1;
    assert!(!menus.move_to(Selection::Item(0, 0), Drop::Item(illisible, 0)));

    assert_eq!(menus.menus[0].items.len(), 4, "rien n'a été perdu");
    assert!(menus.menus[illisible].items.is_empty());
}

/// Une entrée déposée dans un sous-menu situé plus bas vise le bon.
///
/// Le retrait décale la liste de premier niveau, donc le sous-menu désigné par
/// son rang n'est plus au même rang au moment du dépôt. Sans correction,
/// l'entrée atterrissait dans le sous-menu suivant.
#[test]
fn a_drop_into_a_submenu_below_the_source_aims_true() {
    let mut menus = barre();
    menus.menus[0].items = vec![
        ItemDef::Action {
            label: "Entree".into(),
            action: "Entree".into(),
            os_action: None,
            shortcut: None,
        },
        ItemDef::Submenu(MenuDef::named("SousA")),
        ItemDef::Submenu(MenuDef::named("SousB")),
    ];

    assert!(menus.move_to(Selection::Item(0, 0), Drop::SubItem(0, 1, 0)));

    let sous_a = &menus.menus[0].items[0];
    let sous_b = &menus.menus[0].items[1];
    let ItemDef::Submenu(sous_a) = sous_a else { panic!("SousA a disparu") };
    let ItemDef::Submenu(sous_b) = sous_b else { panic!("SousB a disparu") };
    assert_eq!(sous_a.name, "SousA");
    assert_eq!(
        sous_a.items.iter().map(|item| item.label()).collect::<Vec<_>>(),
        vec!["Entree"],
        "l'entrée doit être dans le sous-menu visé"
    );
    assert!(sous_b.items.is_empty(), "et pas dans le suivant");
    assert_eq!(menus.selected, Some(Selection::SubItem(0, 0, 0)));
}

/// La palette aplatit la barre de menus, et laisse de côté ce qui n'est pas une
/// commande.
///
/// Elle n'a pas de liste à elle : c'est la barre de menus, aplatie. Ce test est
/// ce qui tient la promesse — une commande ajoutée au menu y apparaît sans
/// qu'on y touche, et une entrée qui n'est pas une commande n'y apparaît pas.
#[test]
fn the_palette_is_the_menu_bar_flattened() {
    use gpui::{Menu, MenuItem};

    let barre = vec![Menu {
        name: "Fichier".into(),
        items: vec![
            MenuItem::action("Enregistrer", maxx::actions::Save),
            MenuItem::separator(),
            MenuItem::os_submenu("Services", gpui::SystemMenuType::Services),
            MenuItem::action("Aucun projet récent", maxx::actions::NoRecentProject),
            MenuItem::submenu(Menu {
                name: "Ajouter au projet".into(),
                items: vec![MenuItem::action("Les réglages", maxx::actions::AddSettingsModule)],
            }),
        ],
    }];

    let commandes = maxx::palette::flatten(barre);
    let libellés: Vec<String> = commandes.iter().map(|c| c.label.to_string()).collect();

    // Le séparateur et le menu du système ne sont pas des commandes ; le
    // place-tenant des projets récents ne se lance pas.
    assert_eq!(commandes.len(), 2, "{libellés:?}");

    // Chaque ligne porte le chemin qui y mène, sans quoi deux « Ajouter » de
    // menus différents seraient la même ligne.
    assert_eq!(libellés[0], "Fichier ▸ Enregistrer");
    assert_eq!(libellés[1], "Fichier ▸ Ajouter au projet ▸ Les réglages");

    // Et le raccourci vient du clavier de maxx, pas d'une seconde table.
    let raccourci = commandes[0].shortcut.as_ref().map(|keys| keys.to_string());
    assert_eq!(raccourci.as_deref(), Some("⌘S"), "{raccourci:?}");
    assert!(commandes[1].shortcut.is_none(), "cette entrée n'a pas de raccourci");
}

/// La recherche de la palette prend les mots dans n'importe quel ordre.
#[test]
fn the_palette_search_takes_words_in_any_order() {
    use gpui::{Menu, MenuItem};

    let barre = || {
        vec![Menu {
            name: "Fichier".into(),
            items: vec![
                MenuItem::action("Enregistrer", maxx::actions::Save),
                MenuItem::submenu(Menu {
                    name: "Ajouter au projet".into(),
                    items: vec![MenuItem::action("Les réglages", maxx::actions::AddSettingsModule)],
                }),
            ],
        }]
    };
    let cherche = |query: &str| -> Vec<String> {
        maxx::palette::filter(maxx::palette::flatten(barre()), query)
            .into_iter()
            .map(|command| command.label.to_string())
            .collect()
    };

    assert_eq!(cherche("").len(), 2, "une requête vide ne cache rien");

    // Deux mots séparés par un menu entier, dans le désordre, et sans accent.
    let trouvé = cherche("reglages ajouter");
    assert_eq!(trouvé, vec!["Fichier ▸ Ajouter au projet ▸ Les réglages"], "{trouvé:?}");

    assert!(cherche("zzzz").is_empty());
}
