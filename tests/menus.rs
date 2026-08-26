//! The menu editor: reordering must neither lose an entry, nor carry an entry
//! from one menu to another, nor go out of bounds.

use maxx::menu_model::{ItemDef, MenuDef};
use maxx::menufile::{Drop, MenuFile, Selection};

/// Wraps menus in the smallest file that carries a managed region.
fn fichier_de(menus: &[MenuDef]) -> String {
    let rendu = maxx::menu_model::render(menus);
    format!(
        "use gpui::{{Menu, MenuItem}};\n\npub fn app_menus() -> Vec<Menu> {{\n    // maxx:begin\n{}\n    // maxx:end\n}}\n",
        rendu.lines().map(|ligne| format!("    {ligne}")).collect::<Vec<_>>().join("\n")
    )
}

fn barre() -> MenuFile {
    let mut file = MenuDef::named("Fichier");
    file.items.push(ItemDef::Action {
        label: "Nouveau".into(),
        action: "Nouveau".into(),
        os_action: None,
        shortcut: None,
    });
    file.items.push(ItemDef::Separator);
    file.items.push(ItemDef::Action {
        label: "Quitter".into(),
        action: "Quitter".into(),
        os_action: None,
        shortcut: None,
    });
    let menus = vec![file, MenuDef::named("Édition")];

    MenuFile::from_source(std::path::PathBuf::from("/tmp/menus.rs"), fichier_de(&menus))
        .expect("the trial menu bar must read back")
}

#[test]
fn an_entry_moves_within_its_own_menu() {
    let mut menus = barre();
    menus.selected = Some(Selection::Item(0, 2));

    assert!(menus.move_selected(true));
    assert_eq!(menus.selected, Some(Selection::Item(0, 1)));
    let labels: Vec<String> = menus.menus[0].items.iter().map(|item| item.label()).collect();
    assert_eq!(labels[1], "Quitter");
    // The selection follows the entry that moved, otherwise the next one moves
    // on the following press.
    assert_eq!(menus.selected_item().map(|item| item.label()).as_deref(), Some("Quitter"));
    // Nothing was lost.
    assert_eq!(menus.menus[0].items.len(), 3);
    assert_eq!(menus.menus[1].items.len(), 0);
}

#[test]
fn an_entry_never_leaves_its_menu() {
    let mut menus = barre();

    // At the head, moving up does nothing — and above all does not spill into
    // the previous menu.
    menus.selected = Some(Selection::Item(0, 0));
    assert!(!menus.move_selected(true));
    assert_eq!(menus.selected, Some(Selection::Item(0, 0)));

    // At the tail, moving down does not either.
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
    // The menu takes its entries along with it.
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

    // Rendering and then reading back have to give the same order: moving on
    // screen without the file following would serve no purpose.
    let reread =
        MenuFile::from_source(std::path::PathBuf::from("/tmp/menus.rs"), fichier_de(&menus.menus))
            .expect("the menu bar must read back");
    assert_eq!(reread.menus[0].name, "Édition");
    assert_eq!(reread.menus[1].name, "Fichier");
    assert_eq!(reread.menus[1].items.len(), 3);
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
        .expect("the menu bar must read back");

    // The submenu is no longer an opaque block: it has a name and entries.
    let ItemDef::Submenu(inner) = &menus.menus[0].items[1] else {
        panic!("the submenu must be recognised: {:?}", menus.menus[0].items[1]);
    };
    assert_eq!(inner.name, "Récents");
    assert_eq!(inner.items.len(), 2);

    // And it goes through the rendering again without losing its content.
    let reread =
        MenuFile::from_source(std::path::PathBuf::from("/tmp/menus.rs"), fichier_de(&menus.menus))
            .expect("the rendered menu bar must read back");
    let ItemDef::Submenu(inner) = &reread.menus[0].items[1] else {
        panic!("the submenu must survive the rendering");
    };
    assert_eq!(inner.name, "Récents");
    assert_eq!(inner.items[0].label(), "Premier");
    assert_eq!(inner.items.len(), 2);
}

#[test]
fn an_entry_of_a_submenu_moves_inside_it() {
    let mut menus = barre();
    // A submenu in place of the second entry of the first menu.
    let mut inner = MenuDef::named("Récents");
    inner.items.push(ItemDef::Action {
        label: "One".into(),
        action: "One".into(),
        os_action: None,
        shortcut: None,
    });
    inner.items.push(ItemDef::Action {
        label: "Two".into(),
        action: "Two".into(),
        os_action: None,
        shortcut: None,
    });
    menus.menus[0].items[1] = ItemDef::Submenu(inner);

    menus.selected = Some(Selection::SubItem(0, 1, 1));
    assert!(menus.move_selected(true));
    assert_eq!(menus.selected, Some(Selection::SubItem(0, 1, 0)));

    let ItemDef::Submenu(inner) = &menus.menus[0].items[1] else {
        panic!("still a submenu");
    };
    assert_eq!(inner.items[0].label(), "Two");
    // Nothing leaked into the menu holding it.
    assert_eq!(menus.menus[0].items.len(), 3);

    // At the head, moving up does not lift the entry out of the submenu.
    assert!(!menus.move_selected(true));
    assert_eq!(menus.menus[0].items.len(), 3);
}

#[test]
fn adding_to_a_selected_submenu_goes_inside_it() {
    let mut menus = barre();
    menus.menus[0].items[1] = ItemDef::Submenu(MenuDef::named("Récents"));

    // Submenu selected: the entry goes inside it, not beside it.
    menus.selected = Some(Selection::Item(0, 1));
    menus.add_item(ItemDef::Separator);
    assert_eq!(menus.selected, Some(Selection::SubItem(0, 1, 0)));
    let ItemDef::Submenu(inner) = &menus.menus[0].items[1] else {
        panic!("still a submenu");
    };
    assert_eq!(inner.items.len(), 1);
    assert_eq!(menus.menus[0].items.len(), 3);

    // And deleting from the inside removes only the entry.
    menus.remove_selected();
    let ItemDef::Submenu(inner) = &menus.menus[0].items[1] else {
        panic!("the submenu must stay");
    };
    assert_eq!(inner.items.len(), 0);
    assert_eq!(menus.menus[0].items.len(), 3);
}

#[test]
fn an_action_written_inside_a_submenu_is_declared_too() {
    // The defect this test locks down: an action added inside a submenu was
    // written into the file but never declared in `actions!` nor wired — the
    // generated project stopped compiling on a name maxx had just written
    // itself.
    let mut menus = barre();
    let mut inner = MenuDef::named("Récents");
    inner.items.push(ItemDef::Action {
        label: "Rouvrir".into(),
        action: "RouvrirRecent".into(),
        os_action: None,
        shortcut: None,
    });
    menus.menus[0].items[1] = ItemDef::Submenu(inner);

    let actions = menus.actions();
    assert!(actions.contains(&"RouvrirRecent".to_string()), "{actions:?}");
    // And the first-level ones did not disappear on the way.
    assert!(actions.contains(&"Nouveau".to_string()), "{actions:?}");
    assert!(actions.contains(&"Quitter".to_string()), "{actions:?}");
}

#[test]
fn a_qualified_or_system_action_is_still_left_alone_inside_a_submenu() {
    let mut menus = barre();
    let mut inner = MenuDef::named("Récents");
    // An action from another module: not ours to declare.
    inner.items.push(ItemDef::Action {
        label: "Ailleurs".into(),
        action: "other::Action".into(),
        os_action: None,
        shortcut: None,
    });
    // A system action: declaring it would mask what it delegates to.
    inner.items.push(ItemDef::Action {
        label: "Copier".into(),
        action: "Copy".into(),
        os_action: Some("Copy".into()),
        shortcut: None,
    });
    menus.menus[0].items[1] = ItemDef::Submenu(inner);

    let actions = menus.actions();
    assert!(!actions.iter().any(|action| action.contains("::")), "{actions:?}");
    assert!(!actions.contains(&"Copy".to_string()), "{actions:?}");
}

#[test]
fn a_stale_selection_does_not_interrupt_the_process() {
    let mut menus = barre();
    // An index that no longer exists: do nothing, do not panic.
    menus.selected = Some(Selection::Item(0, 9));
    assert!(!menus.move_selected(true));
    assert!(!menus.move_selected(false));
    assert_eq!(menus.menus[0].items.len(), 3);
}

/// The template of a menu file, with its `key_bindings` function.
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

    let mut menus = MenuFile::load(&path).expect("must read back");

    // Read on opening and put on the entry, not looked up in the file on every
    // repaint.
    let ItemDef::Action { shortcut, .. } = &menus.menus[0].items[0] else {
        panic!("the first entry is an action");
    };
    assert_eq!(shortcut.as_deref(), None, "the first entry has no shortcut to begin with");

    // Put on the model: nothing has touched the disk yet.
    let ItemDef::Action { shortcut, .. } = &mut menus.menus[0].items[0] else { unreachable!() };
    *shortcut = Some("cmd-n".into());
    assert_eq!(std::fs::read_to_string(&path).unwrap(), source, "nothing before ⌘S");

    menus.save(false).expect("the save must go through");
    let ecrit = std::fs::read_to_string(&path).unwrap();
    assert!(ecrit.contains("KeyBinding::new(\"cmd-n\", Nouveau, None),"), "{ecrit}");
    // The one next to it did not move.
    assert!(ecrit.contains("KeyBinding::new(\"cmd-q\", Quitter, None),"), "{ecrit}");

    // Reading back gives the shortcut to its entry.
    let reread = MenuFile::load(&path).expect("must read back");
    let ItemDef::Action { shortcut, .. } = &reread.menus[0].items[0] else { unreachable!() };
    assert_eq!(shortcut.as_deref(), Some("cmd-n"));
}

#[test]
fn removing_a_shortcut_removes_every_line_that_bound_it() {
    // gpui accepts several keystrokes for one action: rewriting only one would
    // leave the old one alive behind the user's back.
    let path = std::env::temp_dir().join("maxx_raccourci_double.rs");
    let source = fichier_complet(
        "        KeyBinding::new(\"cmd-n\", Nouveau, None),\n        KeyBinding::new(\"ctrl-n\", Nouveau, None),\n",
    );
    std::fs::write(&path, &source).unwrap();

    let mut menus = MenuFile::load(&path).expect("must read back");
    let ItemDef::Action { shortcut, .. } = &mut menus.menus[0].items[0] else { unreachable!() };
    *shortcut = Some("cmd-shift-n".into());
    menus.save(false).expect("the save must go through");

    let ecrit = std::fs::read_to_string(&path).unwrap();
    assert_eq!(ecrit.matches(", Nouveau, None").count(), 1, "{ecrit}");
    assert!(ecrit.contains("cmd-shift-n"), "{ecrit}");
}

#[test]
fn a_file_without_key_bindings_is_left_whole() {
    // The defect this test locks down: the source was emptied before it could
    // fail, and everything became unrecoverable.
    let path = std::env::temp_dir().join("maxx_sans_bindings.rs");
    let source = r#"use gpui::{Menu, MenuItem};

pub fn app_menus() -> Vec<Menu> {
    // maxx:begin
    vec![Menu { name: "Fichier".into(), items: vec![MenuItem::action("Nouveau", Nouveau)] }]
    // maxx:end
}
"#;
    std::fs::write(&path, source).unwrap();

    let mut menus = MenuFile::load(&path).expect("must read back");
    let ItemDef::Action { shortcut, .. } = &mut menus.menus[0].items[0] else { unreachable!() };
    *shortcut = Some("cmd-n".into());
    menus.save(false).expect("the save must go through all the same");

    assert!(!menus.source.is_empty(), "the source must never be emptied");
    let ecrit = std::fs::read_to_string(&path).unwrap();
    assert!(ecrit.contains("maxx:begin"), "{ecrit}");
    assert!(ecrit.contains("Nouveau"), "{ecrit}");
}

#[test]
fn a_keystroke_gpui_cannot_read_is_refused() {
    // An unreadable keystroke makes `bind_keys` interrupt at startup: the
    // generated application would refuse to open.
    for bad in ["", "cmd-", "-n", "commande-n", "cmd--n"] {
        assert!(!maxx::menufile::is_keystroke(bad), "`{bad}` should be refused");
    }
    for good in ["n", "cmd-n", "cmd-shift-n", "ctrl-alt-delete", "cmd-,", "secondary-a"] {
        assert!(maxx::menufile::is_keystroke(good), "`{good}` should be accepted");
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
    // The tree only goes two levels down: showing the third by halves would
    // give a row with no children, neither selectable nor removable.
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
        .expect("must read back");
    assert!(
        matches!(menus.menus[0].items[0], ItemDef::Opaque(_)),
        "the nested submenu must stay exactly as it is: {:?}",
        menus.menus[0].items[0]
    );
    // Et il ressort intact.
    let rendu = fichier_de(&menus.menus);
    assert!(rendu.contains("Encore"), "{rendu}");
}

/// An entry crosses from one menu to another — what the two keys refuse.
#[test]
fn an_entry_can_be_dragged_into_another_menu() {
    let mut menus = barre();
    assert!(menus.move_to(Selection::Item(0, 2), Drop::Item(1, 0)));

    assert_eq!(menus.menus[0].items.len(), 2, "it has left its menu");
    assert_eq!(menus.menus[1].items.len(), 1);
    assert_eq!(menus.menus[1].items[0].label(), "Quitter");
    assert_eq!(menus.selected, Some(Selection::Item(1, 0)), "and the selection follows it");
}

/// A menu is reordered among the menus, and nowhere else.
#[test]
fn a_menu_is_dragged_among_the_menus() {
    let mut menus = barre();
    assert!(menus.move_to(Selection::Menu(0), Drop::Menu(2)));
    assert_eq!(menus.menus[0].name, "Édition");
    assert_eq!(menus.menus[1].name, "Fichier");
    assert_eq!(menus.selected, Some(Selection::Menu(1)));
}

/// Dropped lower in its own list, the entry accounts for its own removal.
#[test]
fn a_drop_after_the_source_accounts_for_the_removal() {
    let mut menus = barre();
    let labels = |menus: &MenuFile| -> Vec<String> {
        menus.menus[0].items.iter().map(|item| item.label()).collect()
    };
    let before = labels(&menus);

    // The first entry goes to the end: the drop index is 3, but the removal has
    // shifted the list by one.
    assert!(menus.move_to(Selection::Item(0, 0), Drop::Item(0, 3)));
    assert_eq!(labels(&menus), vec![before[1].clone(), before[2].clone(), before[0].clone()]);
}

/// Put back where it was, the entry does not move and is not lost.
#[test]
fn a_drop_where_it_already_is_changes_nothing() {
    let mut menus = barre();
    let before = menus.menus[0].items.clone();

    assert!(!menus.move_to(Selection::Item(0, 1), Drop::Item(0, 1)));
    assert!(!menus.move_to(Selection::Item(0, 1), Drop::Item(0, 2)));
    assert_eq!(menus.menus[0].items, before, "the entry is still there, in its place");
}

/// Three refusals that are rules of the model, not precautions.
#[test]
fn the_three_refusals_of_a_drop() {
    let mut menus = barre();

    // A menu is not an entry.
    assert!(!menus.move_to(Selection::Menu(0), Drop::Item(1, 0)));

    // A submenu does not go inside a submenu: there is only one level.
    menus.menus[0].items.push(ItemDef::Submenu(MenuDef::named("Récents")));
    let sous_menu = menus.menus[0].items.len() - 1;
    assert!(!menus.move_to(Selection::Item(0, sous_menu), Drop::SubItem(0, sous_menu, 0)));

    // And nothing goes into a menu maxx could not read.
    menus.menus.push(MenuDef {
        name: "Dynamique".into(),
        items: Vec::new(),
        opaque: Some("items: construire(),".into()),
    });
    let illisible = menus.menus.len() - 1;
    assert!(!menus.move_to(Selection::Item(0, 0), Drop::Item(illisible, 0)));

    assert_eq!(menus.menus[0].items.len(), 4, "nothing was lost");
    assert!(menus.menus[illisible].items.is_empty());
}

/// An entry dropped into a submenu sitting lower aims at the right one.
///
/// The removal shifts the first-level list, so the submenu named by its rank is
/// no longer at that rank at the moment of the drop. Without the correction,
/// the entry landed in the next submenu.
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
    let sub_b = &menus.menus[0].items[1];
    let ItemDef::Submenu(sous_a) = sous_a else { panic!("SousA a disparu") };
    let ItemDef::Submenu(sub_b) = sub_b else { panic!("SousB a disparu") };
    assert_eq!(sous_a.name, "SousA");
    assert_eq!(
        sous_a.items.iter().map(|item| item.label()).collect::<Vec<_>>(),
        vec!["Entree"],
        "the entry must be in the submenu aimed at"
    );
    assert!(sub_b.items.is_empty(), "and not in the next one");
    assert_eq!(menus.selected, Some(Selection::SubItem(0, 0, 0)));
}

/// The palette flattens the menu bar, and leaves out what is not a command.
/// commande.
///
/// It has no list of its own: it is the menu bar, flattened. This test is what
/// keeps the promise — a command added to the menu appears there without anyone
/// touching it, and an entry that is not a command does not appear.
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

    let commands = maxx::palette::flatten(barre);
    let labels: Vec<String> = commands.iter().map(|c| c.label.to_string()).collect();

    // The separator and the system menu are not commands; the placeholder for
    // the recent projects does not run.
    assert_eq!(commands.len(), 2, "{labels:?}");

    // Every line carries the path leading to it, without which two "Add" from
    // different menus would be the same line.
    assert_eq!(labels[0], "Fichier ▸ Enregistrer");
    assert_eq!(labels[1], "Fichier ▸ Ajouter au projet ▸ Les réglages");

    // And the shortcut comes from maxx's own keymap, not from a second table.
    //
    // On the key and not on the modifier: gpui draws `cmd` with the system's
    // glyph — ⌘ on macOS, ⊞ on Windows, ❖ on Linux — and asserting it would
    // freeze the test on the one of the machine that wrote it.
    let shortcut = commands[0].shortcut.as_ref().map(|keys| keys.to_string());
    let shortcut = shortcut.expect("⌘S is in maxx's keymap");
    assert!(shortcut.ends_with('S'), "{shortcut}");
    assert!(shortcut.chars().count() > 1, "and it carries a modifier: {shortcut}");
    assert!(commands[1].shortcut.is_none(), "this entry has no shortcut");
}

/// The palette's search takes the words in any order.
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
    let search = |query: &str| -> Vec<String> {
        let commands = maxx::palette::flatten(barre());
        maxx::palette::matching(&commands, query)
            .into_iter()
            .map(|position| commands[position].label.to_string())
            .collect()
    };

    assert_eq!(search("").len(), 2, "an empty query hides nothing");

    // Two words separated by a whole menu, out of order, and without accents.
    let found = search("reglages ajouter");
    assert_eq!(found, vec!["Fichier ▸ Ajouter au projet ▸ Les réglages"], "{found:?}");

    assert!(search("zzzz").is_empty());
}
