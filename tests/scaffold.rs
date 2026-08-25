//! The generated project must be ordinary Rust: it compiles and runs without
//! `maxx`, and `maxx` can read its view back.

use std::path::PathBuf;

use maxx::{parser, scaffold, view::View, workspace};

fn scratch(name: &str) -> PathBuf {
    let dir =
        std::env::var("MAXX_SCRATCH").map(PathBuf::from).unwrap_or_else(|_| std::env::temp_dir());
    let path = dir.join(name);
    let _ = std::fs::remove_dir_all(&path);
    path
}

#[test]
fn a_generated_project_is_readable_by_maxx() {
    let root = scratch("maxx_scaffold_test");
    scaffold::create_project(&root, "essai").expect("le projet doit être créé");

    assert!(root.join("Cargo.toml").exists());
    assert!(root.join("src/main.rs").exists());
    assert!(root.join("src/ui/home.rs").exists());

    let cargo = std::fs::read_to_string(root.join("Cargo.toml")).unwrap();
    assert!(
        cargo.contains("runtime_shaders"),
        "sans cette feature le projet ne compile pas sur cette machine"
    );

    let view = View::load(&root.join("src/ui/home.rs")).expect("la vue doit se relire");
    assert_eq!(view.root.base.path(), Some("v_flex"));
    assert_eq!(view.root.children.len(), 1);
    assert_eq!(view.root.children[0].base.path(), Some("Label::new"));
}

#[test]
fn adding_a_view_registers_it() {
    let root = scratch("maxx_scaffold_view_test");
    scaffold::create_project(&root, "essai").unwrap();
    scaffold::create_view(&root, "mon_ecran").expect("la vue doit être créée");

    let module = std::fs::read_to_string(root.join("src/ui/mod.rs")).unwrap();
    assert!(module.contains("pub mod home;"));
    assert!(module.contains("pub mod mon_ecran;"));

    let source = std::fs::read_to_string(root.join("src/ui/mon_ecran.rs")).unwrap();
    assert!(source.contains("pub struct MonEcran"));
    assert!(parser::locate(&source).is_ok());
}

#[test]
fn saving_a_text_input_adds_the_field_and_the_import() {
    let root = scratch("maxx_scaffold_input_test");
    scaffold::create_project(&root, "essai").unwrap();
    let path = root.join("src/ui/home.rs");

    let mut view = View::load(&path).unwrap();
    let input = maxx::registry::instantiate("input").expect("le champ texte est au catalogue");
    view.root.push_child(input);
    view.save().expect("l'enregistrement doit réussir");

    let source = std::fs::read_to_string(&path).unwrap();
    assert!(source.contains("use gpui_component::input::Input;"));
    assert!(source.contains("use gpui_component::input::InputState;"));
    assert!(source.contains("pub field: Entity<InputState>,"));
    assert!(source.contains("field: cx.new(|cx| InputState::new(window, cx)),"));
    assert!(source.contains("Input::new(&self.field)"));

    // And it still reads back.
    let reloaded = View::load(&path).unwrap();
    assert_eq!(reloaded.root.children.len(), 2);
}

#[test]
fn every_component_of_the_catalogue_is_written_out() {
    let root = scratch("maxx_kitchen_sink");
    scaffold::create_project(&root, "essai").unwrap();
    let path = root.join("src/ui/home.rs");

    let mut view = View::load(&path).unwrap();
    for spec in maxx::registry::CATALOGUE {
        let node = maxx::registry::instantiate(spec.id).expect("le catalogue s'instancie");
        view.root.push_child(node);
    }
    view.save().expect("l'enregistrement doit réussir");

    let source = std::fs::read_to_string(&path).unwrap();
    for spec in maxx::registry::CATALOGUE {
        assert!(source.contains(spec.import), "l'import de {} manque", spec.label);
    }

    // And the file still parses back to the same number of nodes.
    let reloaded = View::load(&path).unwrap();
    assert_eq!(reloaded.root.children.len(), maxx::registry::CATALOGUE.len() + 1);
}

#[test]
fn saving_twice_produces_the_same_file() {
    let root = scratch("maxx_stable_save");
    scaffold::create_project(&root, "essai").unwrap();
    let path = root.join("src/ui/home.rs");

    let mut view = View::load(&path).unwrap();
    view.root.push_child(maxx::registry::instantiate("button").unwrap());
    view.save().unwrap();
    let once = std::fs::read_to_string(&path).unwrap();

    // The block sits at the markers' own indentation, not twice it.
    assert!(once.contains("\n            .gap_2()"), "indentation inattendue :\n{once}");

    let mut reloaded = View::load(&path).unwrap();
    reloaded.save().unwrap();
    let twice = std::fs::read_to_string(&path).unwrap();
    assert_eq!(once, twice, "un aller-retour ne doit rien changer sur disque");
}

#[test]
fn two_text_fields_do_not_share_one_state() {
    let mut root = maxx::model::Node::known("v_flex");
    root.children.push(maxx::registry::instantiate("input").unwrap());
    let second = maxx::registry::unique_input_field(&root);
    assert_eq!(second, "field_2");
}

#[test]
fn an_existing_crate_is_not_overwritten() {
    let root = scratch("maxx_no_clobber");
    scaffold::create_project(&root, "essai").unwrap();
    std::fs::write(root.join("src/ui/mod.rs"), "pub mod a_moi;\n").unwrap();

    assert!(scaffold::create_project(&root, "essai").is_err());
    assert_eq!(std::fs::read_to_string(root.join("src/ui/mod.rs")).unwrap(), "pub mod a_moi;\n");
}

#[test]
fn a_folder_name_becomes_a_valid_crate_name() {
    assert_eq!(scaffold::crate_name("Mon App"), "mon_app");
    assert_eq!(scaffold::crate_name("my.app"), "my_app");
    assert_eq!(scaffold::crate_name("2048"), "_2048");
}

#[test]
fn a_button_action_writes_a_method_stub() {
    let root = scratch("maxx_handler");
    scaffold::create_project(&root, "essai").unwrap();
    let path = root.join("src/ui/home.rs");

    let mut view = View::load(&path).unwrap();
    let mut button = maxx::registry::instantiate("button").unwrap();
    let spec = maxx::registry::of(&button).unwrap();
    let action = spec
        .props
        .iter()
        .find(|prop| prop.label == "Action")
        .expect("le bouton a une propriété Action");

    let name = maxx::registry::suggested_handler(&button);
    assert_eq!(name, "on_button");
    maxx::registry::write(&mut button, action, &name);
    view.root.push_child(button);
    view.save().expect("l'enregistrement doit réussir");

    let source = std::fs::read_to_string(&path).unwrap();
    assert!(source.contains(".on_click(cx.listener(Self::on_button))"));
    assert!(source.contains("pub fn on_button("));
    assert!(source.contains("use gpui::ClickEvent;"));
    // `cx` must be named, not `_cx`, for the listener call to compile.
    assert!(source.contains("_window: &mut Window, cx: &mut Context<Self>"));

    // Saving again neither duplicates the stub nor loses what is in it.
    let mut reloaded = View::load(&path).unwrap();
    assert_eq!(
        maxx::registry::read(&reloaded.root.children[1], action).as_deref(),
        Some("on_button")
    );
    reloaded.save().unwrap();
    let twice = std::fs::read_to_string(&path).unwrap();
    assert_eq!(twice.matches("pub fn on_button(").count(), 1);
}

#[test]
fn the_runner_reports_a_failure_instead_of_hanging() {
    // A directory that is not a cargo project: `cargo run` exits at once, which
    // exercises the whole thread-and-channel path without opening a window.
    let root = scratch("maxx_runner");
    std::fs::create_dir_all(&root).unwrap();

    let receiver = maxx::run::start(root);
    let mut lines = Vec::new();
    let mut finished = None;
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(30);

    while finished.is_none() && std::time::Instant::now() < deadline {
        match receiver.recv_timeout(std::time::Duration::from_millis(200)) {
            Ok(maxx::run::Message::Line(line)) => lines.push(line),
            Ok(maxx::run::Message::Started(pid)) => assert!(pid > 0),
            Ok(maxx::run::Message::Finished(ok)) => finished = Some(ok),
            Err(std::sync::mpsc::RecvTimeoutError::Timeout) => {}
            Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => break,
        }
    }

    assert_eq!(finished, Some(false), "cargo doit signaler l'échec");
    assert!(
        lines.iter().any(|line| line.contains("could not find")
            || line.contains("Cargo.toml")
            || line.contains("error")),
        "la sortie de cargo doit remonter dans le panneau : {lines:?}"
    );
}

#[test]
fn style_properties_reach_the_generated_file() {
    let root = scratch("maxx_styles");
    scaffold::create_project(&root, "essai").unwrap();
    let path = root.join("src/ui/home.rs");

    let mut view = View::load(&path).unwrap();
    let mut button = maxx::registry::instantiate("button").unwrap();
    let spec = maxx::registry::of(&button).unwrap();

    for (label, value) in [
        ("Largeur", "120"),
        ("Fond", "#1e2127"),
        ("Couleur du texte", "c8ccd4"),
        ("Infobulle", "Enregistrer"),
        ("Taille du texte", "text_sm"),
    ] {
        let prop = maxx::registry::props(spec)
            .into_iter()
            .find(|prop| prop.label == label)
            .unwrap_or_else(|| panic!("propriété « {label} » absente"));
        maxx::registry::write(&mut button, prop, value);
    }
    view.root.push_child(button);
    view.save().unwrap();

    let source = std::fs::read_to_string(&path).unwrap();
    assert!(source.contains(".w(px(120.))"));
    assert!(source.contains(".bg(rgb(0x1e2127))"));
    assert!(source.contains(".text_color(rgb(0xc8ccd4))"));
    assert!(source.contains(".tooltip(\"Enregistrer\")"));
    assert!(source.contains(".text_sm()"));
    // `px` and `rgb` are functions of gpui, not methods of the component.
    assert!(source.contains("use gpui::px;"));
    assert!(source.contains("use gpui::rgb;"));

    // And they read back.
    let reloaded = View::load(&path).unwrap();
    let button = &reloaded.root.children[1];
    let width =
        maxx::registry::props(spec).into_iter().find(|prop| prop.label == "Largeur").unwrap();
    assert_eq!(maxx::registry::read(button, width).as_deref(), Some("120"));
}

#[test]
fn an_uncatalogued_call_is_reported_as_such() {
    let mut node = maxx::registry::instantiate("button").unwrap();
    let spec = maxx::registry::of(&node).unwrap();
    node.calls.push(maxx::model::Call::bare("shadow_lg"));

    assert!(maxx::registry::covers(spec, "label"));
    assert!(maxx::registry::covers(spec, "w"), "les styles communs comptent");
    assert!(!maxx::registry::covers(spec, "shadow_lg"));
}

#[test]
fn a_generated_project_shares_the_build_cache() {
    let root = scratch("maxx_cache_a");
    scaffold::create_project(&root, "cache_a").unwrap();

    // Le chemin est comparé après lecture du TOML, jamais par sous-chaîne : un
    // chemin Windows y est écrit échappé — `C:\\Users\\…` — et le chercher
    // brut échouait sur Windows alors que le fichier était juste. C'est ce test
    // qui a fait rougir la matrice après la correction de l'échappement.
    let config = std::fs::read_to_string(root.join(".cargo/config.toml")).unwrap();
    assert!(config.contains("[build]"));
    let parsed: toml::Value = toml::from_str(&config).expect("le fichier doit rester du TOML");
    assert_eq!(
        std::path::PathBuf::from(parsed["build"]["target-dir"].as_str().unwrap()),
        maxx::run::shared_target_dir()
    );

    // The cache is machine-local, so it must not follow the project into git.
    let ignore = std::fs::read_to_string(root.join(".gitignore")).unwrap();
    assert!(ignore.contains("/.cargo"));
    assert!(ignore.contains("/target"));
}

#[test]
fn a_state_field_is_declared_and_initialised() {
    let root = scratch("maxx_state");
    scaffold::create_project(&root, "essai").unwrap();
    let path = root.join("src/ui/home.rs");

    let mut view = View::load(&path).unwrap();
    assert!(view.state_fields().is_empty());

    view.add_state_field("message", "SharedString", "\"\".into()")
        .expect("le champ doit être ajouté");

    let fields = view.state_fields();
    assert_eq!(fields.len(), 1);
    assert_eq!(fields[0].name, "message");
    assert_eq!(fields[0].ty, "SharedString");
    assert_eq!(fields[0].read_expression(), "self.message.clone()");

    let source = std::fs::read_to_string(&path).unwrap();
    assert!(source.contains("message: SharedString,"));
    assert!(source.contains("message: \"\".into(),"));
    assert!(source.contains("use gpui::SharedString;"));

    // A second field with the same name is refused rather than duplicated.
    assert!(view.add_state_field("message", "usize", "0").is_err());
    assert!(view.add_state_field("mon champ", "usize", "0").is_err());
}

#[test]
fn a_property_can_read_a_state_field() {
    let mut label = maxx::registry::instantiate("label").unwrap();
    let spec = maxx::registry::of(&label).unwrap();
    let text = spec.props.iter().find(|prop| prop.label == "Texte").unwrap();

    assert_eq!(maxx::registry::read_binding(&label, text), None);

    maxx::registry::write_binding(&mut label, text, Some("self.message.clone()"));
    assert_eq!(maxx::codegen::render(&label, 0), "Label::new(self.message.clone())");
    assert_eq!(maxx::registry::read_binding(&label, text).as_deref(), Some("message"));
    // A bound value is not editable as free text: overwriting it with a string
    // literal would silently change what the code means.
    assert!(!maxx::registry::editable(&label, text));

    maxx::registry::write_binding(&mut label, text, None);
    assert_eq!(maxx::registry::read_binding(&label, text), None);
    assert!(maxx::registry::editable(&label, text));
}

#[test]
fn a_hand_written_view_can_be_adopted() {
    let root = scratch("maxx_adopt");
    std::fs::create_dir_all(root.join("src/ui")).unwrap();
    let path = root.join("src/ui/ecrit_a_la_main.rs");
    let source = "\
use gpui::{Context, Window, prelude::*};
use gpui_component::v_flex;
use gpui_component::label::Label;

pub struct Ecrit {}

impl Render for Ecrit {
    /// Un commentaire qui doit survivre.
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        v_flex().gap_2().child(Label::new(\"Écrit à la main\"))
    }
}
";
    std::fs::write(&path, source).unwrap();
    assert!(View::load(&path).is_err(), "sans marqueurs, maxx refuse");

    let adopted = maxx::parser::adopt(source).expect("l'adoption doit réussir");
    std::fs::write(&path, &adopted).unwrap();

    assert!(adopted.contains("Un commentaire qui doit survivre"));
    let view = View::load(&path).expect("la vue adoptée doit s'ouvrir");
    assert_eq!(view.root.base.path(), Some("v_flex"));
    assert_eq!(view.root.children.len(), 1);

    // Adopting twice is refused rather than nesting a second pair of markers.
    assert!(maxx::parser::adopt(&adopted).is_err());
}

#[test]
fn a_render_without_a_trailing_expression_is_refused() {
    let source = "\
impl Render for Ecrit {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        let element = v_flex();
        return element;
    }
}
";
    assert!(matches!(maxx::parser::adopt(source), Err(maxx::parser::Error::NoTrailingExpression)));
}

#[test]
fn an_outside_change_is_noticed() {
    let root = scratch("maxx_conflict");
    scaffold::create_project(&root, "essai").unwrap();
    let path = root.join("src/ui/home.rs");

    let view = View::load(&path).unwrap();
    assert!(!view.disk_changed());

    // Someone edits the file in Zed.
    let outside = std::fs::read_to_string(&path).unwrap().replace("Welcome", "Modifié dans Zed");
    std::fs::write(&path, &outside).unwrap();
    assert!(view.disk_changed());

    // With nothing to lose on this side, reloading takes what is on disk.
    let mut view = view;
    view.reload().unwrap();
    assert!(!view.disk_changed());
    assert!(!view.dirty());
    let label = &view.root.children[0];
    assert_eq!(
        maxx::registry::read(
            label,
            maxx::registry::of(label).unwrap().props.iter().next().unwrap()
        )
        .as_deref(),
        Some("Modifié dans Zed")
    );
}

#[test]
fn insertions_land_in_the_view_not_in_a_helper_type() {
    let root = scratch("maxx_anchor");
    scaffold::create_project(&root, "essai").unwrap();
    let path = root.join("src/ui/home.rs");

    // A helper type declared above the view, as a developer would.
    let source = std::fs::read_to_string(&path).unwrap();
    let source = source.replace(
        "pub struct Home {}",
        "pub struct Ligne {\n    pub titre: String,\n}\n\nimpl Ligne {\n    pub fn nouvelle() -> Self {\n        Self {\n            titre: String::new(),\n        }\n    }\n}\n\npub struct Home {}",
    );
    std::fs::write(&path, &source).unwrap();

    let mut view = View::load(&path).unwrap();
    let mut button = maxx::registry::instantiate("button").unwrap();
    let spec = maxx::registry::of(&button).unwrap();
    let action = spec.props.iter().find(|p| p.label == "Action").unwrap();
    maxx::registry::write(&mut button, action, "on_go");
    view.root.push_child(button);
    view.root.push_child(maxx::registry::instantiate("input").unwrap());
    view.save().expect("l'enregistrement doit réussir");

    let written = std::fs::read_to_string(&path).unwrap();
    let ligne = &written
        [written.find("pub struct Ligne").unwrap()..written.find("pub struct Home").unwrap()];
    assert!(!ligne.contains("champ"), "le type auxiliaire est intact :\n{ligne}");
    assert!(!ligne.contains("on_go"), "le stub ne va pas dans le type auxiliaire");

    let home = &written[written.find("pub struct Home").unwrap()..];
    assert!(home.contains("pub field: Entity<InputState>,"));
    assert!(home.contains("pub fn on_go("));
    // And the initializer goes in the struct literal, not in the signature.
    assert!(home.contains("Self {\n            field: cx.new("));
}

#[test]
fn a_state_field_is_refused_when_the_view_has_no_usable_shape() {
    let root = scratch("maxx_shape");
    scaffold::create_project(&root, "essai").unwrap();
    let path = root.join("src/ui/home.rs");

    // A view with no `Self { .. }` to initialize into.
    let source = std::fs::read_to_string(&path)
        .unwrap()
        .replace("        Self {}\n", "        Self::default()\n");
    std::fs::write(&path, &source).unwrap();

    let mut view = View::load(&path).unwrap();
    assert!(
        view.add_state_field("compteur", "usize", "0").is_err(),
        "mieux vaut refuser que déclarer la moitié du champ"
    );
    assert_eq!(
        std::fs::read_to_string(&path).unwrap(),
        source,
        "et le fichier ne doit pas avoir bougé"
    );
}

#[test]
fn a_wrapped_import_is_not_duplicated() {
    let root = scratch("maxx_wrapped");
    scaffold::create_project(&root, "essai").unwrap();
    let path = root.join("src/ui/home.rs");

    let source = std::fs::read_to_string(&path).unwrap().replace(
        "use gpui::{Context, Window, prelude::*};",
        "use gpui::{\n    Context,\n    Window,\n    px,\n    prelude::*,\n};",
    );
    std::fs::write(&path, &source).unwrap();

    let mut view = View::load(&path).unwrap();
    let mut button = maxx::registry::instantiate("button").unwrap();
    let spec = maxx::registry::of(&button).unwrap();
    let width =
        maxx::registry::props(spec).into_iter().find(|prop| prop.label == "Largeur").unwrap();
    maxx::registry::write(&mut button, width, "120");
    view.root.push_child(button);
    view.save().unwrap();

    let written = std::fs::read_to_string(&path).unwrap();
    assert!(
        !written.contains("use gpui::px;"),
        "px est déjà importé par le use groupé :\n{written}"
    );
}

#[test]
fn a_helper_type_whose_name_starts_like_the_view_is_left_alone() {
    let root = scratch("maxx_prefix");
    scaffold::create_project(&root, "essai").unwrap();
    let path = root.join("src/ui/home.rs");

    let source = std::fs::read_to_string(&path).unwrap().replace(
        "pub struct Home {}",
        "pub struct HomeConfig {\n    pub titre: String,\n}\n\npub struct Home {}",
    );
    std::fs::write(&path, &source).unwrap();

    let mut view = View::load(&path).unwrap();
    view.root.push_child(maxx::registry::instantiate("input").unwrap());
    view.save().unwrap();

    let written = std::fs::read_to_string(&path).unwrap();
    let config = &written[written.find("pub struct HomeConfig").unwrap()
        ..written.find("pub struct Home {").unwrap()];
    assert!(!config.contains("champ"), "le type voisin est intact :\n{config}");
}

#[test]
fn a_generated_project_has_a_menu_bar() {
    let root = scratch("maxx_menus");
    scaffold::create_project(&root, "essai").unwrap();
    let path = root.join("src/menus.rs");

    let main = std::fs::read_to_string(root.join("src/main.rs")).unwrap();
    assert!(main.contains("cx.set_menus(menus::app_menus());"));
    assert!(main.contains("menus::register(cx);"));

    let mut menus = maxx::menufile::MenuFile::load(&path).expect("les menus doivent se relire");
    assert_eq!(menus.menus.len(), 3);
    assert_eq!(menus.menus[1].name, "Edit");
    assert!(menus.menus[0].items.iter().any(|item| item.label() == "Quit"));
    assert!(!menus.dirty());

    // An entry with a brand new action declares and wires it on save.
    menus.selected = Some(maxx::menufile::Selection::Menu(0));
    menus.add_item(maxx::menu_model::ItemDef::Action {
        label: "Préférences…".into(),
        action: "OpenSettings".into(),
        os_action: None,
        shortcut: None,
    });
    assert!(menus.dirty());
    menus.save(false).expect("l'enregistrement doit réussir");

    let source = std::fs::read_to_string(&path).unwrap();
    assert!(source.contains("MenuItem::action(\"Préférences…\", OpenSettings)"));
    assert!(source.contains("OpenSettings]"), "déclaré dans actions! : {source}");
    assert!(source.contains("cx.on_action(|_: &OpenSettings,"));

    // And it reads back, twice over, without drifting.
    let mut again = maxx::menufile::MenuFile::load(&path).unwrap();
    assert_eq!(again.menus, menus.menus);
    again.save(false).unwrap();
    assert_eq!(std::fs::read_to_string(&path).unwrap(), source);
}

#[test]
fn an_unknown_menu_entry_is_carried_through() {
    let root = scratch("maxx_menus_opaque");
    scaffold::create_project(&root, "essai").unwrap();
    let path = root.join("src/menus.rs");

    let source = std::fs::read_to_string(&path).unwrap().replace(
        "MenuItem::action(\"Quit\", Quit),",
        "MenuItem::submenu(sous_menu()),\n                MenuItem::action(\"Quit\", Quit),",
    );
    std::fs::write(&path, &source).unwrap();

    let mut menus = maxx::menufile::MenuFile::load(&path).expect("doit se relire quand même");
    assert!(
        menus
            .menus
            .iter()
            .flat_map(|menu| &menu.items)
            .any(|item| matches!(item, maxx::menu_model::ItemDef::Opaque(_)))
    );

    menus.save(false).unwrap();
    assert!(
        std::fs::read_to_string(&path).unwrap().contains("MenuItem::submenu(sous_menu())"),
        "ce que maxx ne comprend pas ressort tel quel"
    );
}

#[test]
fn saving_untouched_menus_changes_nothing() {
    let root = scratch("maxx_menus_noop");
    scaffold::create_project(&root, "essai").unwrap();
    let path = root.join("src/menus.rs");
    let before = std::fs::read_to_string(&path).unwrap();

    let mut menus = maxx::menufile::MenuFile::load(&path).unwrap();
    // The Édition entries are `os_action`s: they belong to the system, and
    // registering handlers of our own would shadow them.
    assert!(!menus.actions().iter().any(|name| name == "Copy"));
    menus.save(false).unwrap();

    assert_eq!(
        std::fs::read_to_string(&path).unwrap(),
        before,
        "un ⌘S sur une barre intacte ne doit rien réécrire"
    );
}

#[test]
fn a_menu_maxx_cannot_read_is_carried_through() {
    let root = scratch("maxx_menus_odd");
    scaffold::create_project(&root, "essai").unwrap();
    let path = root.join("src/menus.rs");

    let source = std::fs::read_to_string(&path).unwrap().replace(
        "        Menu {\n            name: \"Window\".into(),",
        "        Menu {\n            name: \"Dynamique\".into(),\n            items: construire(),\n        },\n        Menu {\n            name: \"Window\".into(),",
    );
    std::fs::write(&path, &source).unwrap();

    let mut menus = maxx::menufile::MenuFile::load(&path).expect("le fichier reste ouvrable");
    assert!(
        menus.menus.iter().any(|menu| menu.is_opaque()),
        "le menu illisible est conservé, pas fatal"
    );
    menus.add_menu();
    menus.save(false).unwrap();
    assert!(
        std::fs::read_to_string(&path).unwrap().contains("items: construire()"),
        "et il ressort tel quel"
    );
}

#[test]
fn a_qualified_action_keeps_its_path() {
    let root = scratch("maxx_menus_path");
    scaffold::create_project(&root, "essai").unwrap();
    let path = root.join("src/menus.rs");

    let source = std::fs::read_to_string(&path).unwrap().replace(
        "MenuItem::action(\"About\", About),",
        "MenuItem::action(\"About\", fichier::About),",
    );
    std::fs::write(&path, &source).unwrap();

    let mut menus = maxx::menufile::MenuFile::load(&path).unwrap();
    // Not ours to declare: it lives in another module.
    assert!(!menus.actions().iter().any(|name| name.contains("About")));
    menus.add_menu();
    menus.save(false).unwrap();
    assert!(
        std::fs::read_to_string(&path)
            .unwrap()
            .contains("MenuItem::action(\"About\", fichier::About)")
    );
}

#[test]
fn a_view_named_menus_is_not_taken_for_the_menu_bar() {
    use maxx::menufile::MenuFile;
    assert!(MenuFile::is_menu_file(std::path::Path::new("/p/src/menus.rs")));
    assert!(!MenuFile::is_menu_file(std::path::Path::new("/p/src/ui/menus.rs")));
}

#[test]
fn a_menu_action_points_at_its_handler() {
    let root = scratch("maxx_menu_goto");
    scaffold::create_project(&root, "essai").unwrap();
    let path = root.join("src/menus.rs");

    let mut menus = maxx::menufile::MenuFile::load(&path).unwrap();
    // Wired by the template.
    let line = menus.handler_line("Quit").expect("Quit a un gestionnaire");
    let source = std::fs::read_to_string(&path).unwrap();
    assert!(source.lines().nth(line - 1).unwrap().contains("&Quit,"));

    // Not wired yet: the button must say so rather than open the wrong line.
    assert_eq!(menus.handler_line("Preferences"), None);

    menus.selected = Some(maxx::menufile::Selection::Menu(0));
    menus.add_item(maxx::menu_model::ItemDef::Action {
        label: "Préférences…".into(),
        action: "Preferences".into(),
        os_action: None,
        shortcut: None,
    });
    menus.save(false).unwrap();

    let reloaded = maxx::menufile::MenuFile::load(&path).unwrap();
    let line = reloaded.handler_line("Preferences").expect("l'enregistrement l'a câblée");
    let source = std::fs::read_to_string(&path).unwrap();
    assert!(source.lines().nth(line - 1).unwrap().contains("&Preferences,"));
}

#[test]
fn deleting_a_view_unregisters_it() {
    let root = scratch("maxx_delete_view_test");
    scaffold::create_project(&root, "essai").expect("le projet doit être créé");
    scaffold::create_view(&root, "view_2").expect("la vue doit être créée");

    let mod_path = root.join("src/ui/mod.rs");
    assert!(std::fs::read_to_string(&mod_path).unwrap().contains("pub mod view_2;"));

    let file = root.join("src/ui/view_2.rs");
    assert_eq!(workspace::view_module(&root, &file).as_deref(), Some("view_2"));
    workspace::unregister_view(&root, "view_2");

    let source = std::fs::read_to_string(&mod_path).unwrap();
    assert!(!source.contains("pub mod view_2;"));
    assert!(source.contains("pub mod home;"), "les autres vues restent déclarées : {source}");
}

/// Une vue d'entrée qui ne s'appelle pas `home` est protégée comme les autres.
///
/// C'est le cas d'un projet écrit par un maxx plus ancien, dont `main.rs`
/// importe encore `accueil` : supposer le nom du gabarit courant laisserait
/// supprimer la vue que `main.rs` ouvre, et le projet ne compilerait plus.
#[test]
fn the_entry_view_is_read_from_main_rs() {
    let root = scratch("maxx_entree_ancienne");
    scaffold::create_project(&root, "essai").unwrap();

    std::fs::rename(root.join("src/ui/home.rs"), root.join("src/ui/accueil.rs")).unwrap();
    std::fs::write(root.join("src/ui/mod.rs"), "pub mod accueil;\n").unwrap();
    let main_rs = std::fs::read_to_string(root.join("src/main.rs"))
        .unwrap()
        .replace("crate::ui::home::Home", "crate::ui::accueil::Home");
    std::fs::write(root.join("src/main.rs"), main_rs).unwrap();

    assert!(
        workspace::protected_entry(&root, &root.join("src/ui/accueil.rs")).is_some(),
        "la vue que main.rs ouvre est protégée quel que soit son nom"
    );
    assert!(
        workspace::protected_entry(&root, &root.join("src/ui/home.rs")).is_none(),
        "et le nom du gabarit courant ne protège rien tout seul"
    );
}

#[test]
fn the_project_skeleton_refuses_to_be_deleted() {
    let root = scratch("maxx_protege");
    scaffold::create_project(&root, "essai").unwrap();
    for kept in ["Cargo.toml", "src/main.rs", "src/ui/mod.rs", "src/ui/home.rs"] {
        assert!(
            workspace::protected_entry(&root, &root.join(kept)).is_some(),
            "{kept} doit être protégé"
        );
    }
    assert!(workspace::protected_entry(&root, &root).is_some());
    // La barre de menus, elle, se retire : sa suppression décâble main.rs.
    assert!(workspace::protected_entry(&root, &root.join("src/menus.rs")).is_none());
    assert!(workspace::protected_entry(&root, &root.join("src/ui/view_2.rs")).is_none());
    // `view_module` ne doit pas prendre un fichier hors de src/ui pour une vue.
    assert_eq!(workspace::view_module(&root, &root.join("src/menus.rs")), None);
    assert_eq!(workspace::view_module(&root, &root.join("src/ui/sous/vue.rs")), None);
}

#[test]
fn a_menu_bar_can_be_added_to_a_project_that_has_none() {
    let root = scratch("maxx_add_menu_bar_test");
    scaffold::create_project(&root, "essai").expect("le projet doit être créé");

    // Un projet antérieur à la barre de menus générée : ni le fichier, ni le
    // câblage dans main.rs.
    std::fs::remove_file(root.join("src/menus.rs")).unwrap();
    scaffold::remove_menu_bar(&root).expect("le câblage doit partir");
    let stripped = std::fs::read_to_string(root.join("src/main.rs")).unwrap();
    assert!(!stripped.contains("mod menus;"));
    assert!(!stripped.contains("menus::app_menus()"));
    assert!(stripped.contains("cx.activate(true);"));

    scaffold::add_menu_bar(&root).expect("la barre doit être ajoutée");
    assert!(root.join("src/menus.rs").exists());
    let wired = std::fs::read_to_string(root.join("src/main.rs")).unwrap();
    assert!(wired.starts_with("mod menus;\n"));
    assert!(wired.contains("        menus::register(cx);\n"));
    assert!(wired.contains("        cx.bind_keys(menus::key_bindings());\n"));
    assert!(wired.contains("        cx.set_menus(menus::app_menus());\n"));

    // Deux fois de suite ne doit rien dupliquer.
    scaffold::add_menu_bar(&root).expect("la seconde fois ne doit rien casser");
    let twice = std::fs::read_to_string(root.join("src/main.rs")).unwrap();
    assert_eq!(twice.matches("mod menus;").count(), 1);
    assert_eq!(twice.matches("menus::register(cx);").count(), 1);

    // maxx doit relire la barre qu'il vient d'écrire.
    let menus = maxx::menufile::MenuFile::load(&root.join("src/menus.rs"))
        .expect("la barre doit se relire");
    assert!(!menus.menus.is_empty());
}

#[test]
fn wiring_the_menu_bar_keeps_the_header_of_main_rs_valid() {
    let root = scratch("maxx_menu_bar_header_test");
    scaffold::create_project(&root, "essai").expect("le projet doit être créé");
    std::fs::remove_file(root.join("src/menus.rs")).unwrap();
    scaffold::remove_menu_bar(&root).unwrap();

    // Un main.rs qui commence par un commentaire de module et un attribut
    // interne : un `mod menus;` en ligne 1 ne compilerait pas.
    let main_path = root.join("src/main.rs");
    let source = std::fs::read_to_string(&main_path).unwrap();
    std::fs::write(&main_path, format!("//! Mon application.\n#![allow(dead_code)]\n\n{source}"))
        .unwrap();

    scaffold::add_menu_bar(&root).expect("la barre doit être ajoutée");

    let wired = std::fs::read_to_string(&main_path).unwrap();
    let lines: Vec<&str> = wired.lines().collect();
    assert_eq!(lines[0], "//! Mon application.");
    assert_eq!(lines[1], "#![allow(dead_code)]");
    assert!(
        lines.iter().position(|line| *line == "mod menus;").unwrap()
            > lines.iter().position(|line| *line == "#![allow(dead_code)]").unwrap(),
        "{wired}"
    );
}

#[test]
fn a_main_rs_it_cannot_wire_is_left_alone() {
    let root = scratch("maxx_menu_bar_refusal_test");
    scaffold::create_project(&root, "essai").expect("le projet doit être créé");
    std::fs::remove_file(root.join("src/menus.rs")).unwrap();
    scaffold::remove_menu_bar(&root).unwrap();

    // Ni cx.activate(…), ni Application::new().run(…) : maxx ne sait pas où
    // insérer les appels.
    let main_path = root.join("src/main.rs");
    std::fs::write(&main_path, "fn main() {\n    println!(\"bonjour\");\n}\n").unwrap();

    let error = scaffold::add_menu_bar(&root).expect_err("le câblage doit être refusé");
    assert!(error.to_string().contains("cx.activate"));
    assert!(
        !root.join("src/menus.rs").exists(),
        "un menus.rs orphelin ferait croire au projet qu'il a déjà une barre"
    );
    let untouched = std::fs::read_to_string(&main_path).unwrap();
    assert!(!untouched.contains("mod menus;"), "{untouched}");
}

#[test]
fn the_wiring_follows_the_name_the_closure_gave_the_application() {
    let root = scratch("maxx_menu_bar_binding_test");
    scaffold::create_project(&root, "essai").expect("le projet doit être créé");
    std::fs::remove_file(root.join("src/menus.rs")).unwrap();
    scaffold::remove_menu_bar(&root).unwrap();

    // Ni cx.activate(…), et une fermeture qui n'appelle pas son application
    // « cx » : trois lignes parlant de cx ne compileraient pas.
    let main_path = root.join("src/main.rs");
    std::fs::write(
        &main_path,
        "use gpui::{App, Application};\n\nfn main() {\n    Application::new().run(|app: &mut App| {\n        let _ = app;\n    });\n}\n",
    )
    .unwrap();

    scaffold::add_menu_bar(&root).expect("la barre doit être ajoutée");

    let wired = std::fs::read_to_string(&main_path).unwrap();
    assert!(wired.contains("menus::register(app);"), "{wired}");
    assert!(wired.contains("app.bind_keys(menus::key_bindings());"), "{wired}");
    assert!(wired.contains("app.set_menus(menus::app_menus());"), "{wired}");
    assert!(!wired.contains("(cx)"), "{wired}");
}

#[test]
fn the_menu_bar_is_unwired_whatever_the_application_is_called() {
    let root = scratch("maxx_menu_bar_unwire_test");
    scaffold::create_project(&root, "essai").expect("le projet doit être créé");
    std::fs::remove_file(root.join("src/menus.rs")).unwrap();
    scaffold::remove_menu_bar(&root).unwrap();

    let main_path = root.join("src/main.rs");
    std::fs::write(
        &main_path,
        "use gpui::{App, Application};\n\nfn main() {\n    Application::new().run(|app: &mut App| {\n        let _ = app;\n    });\n}\n",
    )
    .unwrap();

    scaffold::add_menu_bar(&root).expect("la barre doit être ajoutée");
    assert!(std::fs::read_to_string(&main_path).unwrap().contains("menus::register(app);"));

    // Le décâblage doit retrouver ces lignes-là, pas celles du gabarit.
    scaffold::remove_menu_bar(&root).expect("le câblage doit partir");
    let stripped = std::fs::read_to_string(&main_path).unwrap();
    assert!(!stripped.contains("menus::"), "{stripped}");
    assert!(!stripped.contains("mod menus;"), "{stripped}");
    assert!(stripped.contains("let _ = app;"), "le reste du fichier doit rester : {stripped}");
}

#[test]
fn the_system_module_is_added_declared_and_compiles_on_its_own() {
    let root = scratch("maxx_system_module_test");
    scaffold::create_project(&root, "essai").expect("le projet doit être créé");

    // Un main.rs qui commence par un commentaire de module : la déclaration ne
    // doit pas passer devant.
    let main_path = root.join("src/main.rs");
    let source = std::fs::read_to_string(&main_path).unwrap();
    std::fs::write(&main_path, format!("//! Mon application.\n{source}")).unwrap();

    scaffold::add_system_module(&root).expect("le module doit être ajouté");

    let module = root.join("src/system.rs");
    assert!(module.exists());
    let wired = std::fs::read_to_string(&main_path).unwrap();
    assert!(wired.starts_with("//! Mon application.\n"), "{wired}");
    assert!(wired.contains("mod system;"), "{wired}");

    // Deux fois de suite ne duplique rien.
    scaffold::add_system_module(&root).expect("la seconde fois ne doit rien casser");
    let twice = std::fs::read_to_string(&main_path).unwrap();
    assert_eq!(twice.matches("mod system;").count(), 1);

    // Il ne dépend ni de maxx ni de gpui : rien que du std.
    let body = std::fs::read_to_string(&module).unwrap();
    assert!(!body.contains("use gpui"), "le module doit rester du std pur");
    assert!(!body.contains("maxx::"), "le module ne doit rien devoir à maxx");
    // Et il ne double pas ce que gpui fournit déjà.
    for covered in ["clipboard", "open_url", "reveal_path"] {
        assert!(!body.contains(&format!("pub fn {covered}")), "{covered} est déjà dans gpui");
    }

    // Le retirer doit retirer sa déclaration, sinon le projet ne compile plus.
    scaffold::remove_module(&root, "system").expect("la déclaration doit partir");
    let stripped = std::fs::read_to_string(&main_path).unwrap();
    assert!(!stripped.contains("mod system;"), "{stripped}");
    assert!(stripped.contains("mod ui;"), "le reste doit rester : {stripped}");
}

#[test]
fn only_a_top_level_module_file_has_a_mod_line() {
    let root = PathBuf::from("/tmp/essai");
    assert_eq!(
        maxx::workspace::top_level_module(&root, &root.join("src/system.rs")).as_deref(),
        Some("system")
    );
    // main.rs n'est pas un module, ui/ a son propre mod.rs.
    assert_eq!(maxx::workspace::top_level_module(&root, &root.join("src/main.rs")), None);
    assert_eq!(maxx::workspace::top_level_module(&root, &root.join("src/ui/view_1.rs")), None);
    assert_eq!(maxx::workspace::top_level_module(&root, &root.join("Cargo.toml")), None);
}

#[test]
fn a_block_doc_comment_header_is_not_jumped_over() {
    let root = scratch("maxx_block_header_test");
    scaffold::create_project(&root, "essai").expect("le projet doit être créé");

    // `/*! … */` est un commentaire de module interne : aucun item ne peut le
    // précéder, et une insertion en ligne 1 casse la compilation.
    let main_path = root.join("src/main.rs");
    let source = std::fs::read_to_string(&main_path).unwrap();
    std::fs::write(
        &main_path,
        format!("/*!\n Mon application.\n*/\n#![allow(dead_code)]\n\n{source}"),
    )
    .unwrap();

    scaffold::add_system_module(&root).expect("le module doit être ajouté");

    let wired = std::fs::read_to_string(&main_path).unwrap();
    let lines: Vec<&str> = wired.lines().collect();
    let declaration = lines.iter().position(|line| *line == "mod system;").unwrap();
    let fin_du_bloc = lines.iter().position(|line| *line == "*/").unwrap();
    let attribut = lines.iter().position(|line| *line == "#![allow(dead_code)]").unwrap();
    assert!(declaration > fin_du_bloc, "{wired}");
    assert!(declaration > attribut, "{wired}");
}

#[test]
fn deleting_an_undeclared_file_leaves_main_rs_untouched() {
    let root = scratch("maxx_untouched_main_test");
    scaffold::create_project(&root, "essai").expect("le projet doit être créé");

    // Des fins de ligne Windows : une réécriture inutile les convertirait.
    let main_path = root.join("src/main.rs");
    let source = std::fs::read_to_string(&main_path).unwrap().replace('\n', "\r\n");
    std::fs::write(&main_path, &source).unwrap();

    scaffold::remove_module(&root, "jamais_declare").expect("rien à retirer");

    assert_eq!(std::fs::read_to_string(&main_path).unwrap(), source);
}

#[test]
fn the_shared_target_dir_survives_being_written_into_toml() {
    let root = scratch("maxx_toml_target_test");
    scaffold::create_project(&root, "essai").expect("le projet doit être créé");

    let config = std::fs::read_to_string(root.join(".cargo/config.toml")).unwrap();
    let parsed: toml::Value = toml::from_str(&config).unwrap_or_else(|error| {
        panic!("le fichier de configuration doit rester du TOML : {error}\n{config}")
    });
    let target = parsed["build"]["target-dir"].as_str().unwrap();
    assert!(!target.is_empty());
    // Ce qui a été échappé pour TOML doit se relire tel quel.
    assert_eq!(std::path::PathBuf::from(target), maxx::run::shared_target_dir());
}

#[test]
fn the_settings_module_brings_what_it_needs() {
    let root = scratch("maxx_settings_module_test");
    scaffold::create_project(&root, "essai").expect("le projet doit être créé");

    scaffold::add_settings_module(&root).expect("les réglages doivent être ajoutés");

    // Il tire le module système avec lui : il a besoin de savoir où ce système
    // range les fichiers d'une application.
    assert!(root.join("src/system.rs").exists());
    assert!(root.join("src/settings.rs").exists());

    let main_rs = std::fs::read_to_string(root.join("src/main.rs")).unwrap();
    assert!(main_rs.contains("mod system;"), "{main_rs}");
    assert!(main_rs.contains("mod settings;"), "{main_rs}");

    // Les deux crates sont déclarées, dans la section des dépendances et pas
    // après le bloc [profile].
    let cargo = std::fs::read_to_string(root.join("Cargo.toml")).unwrap();
    let parsed: toml::Value = toml::from_str(&cargo).expect("Cargo.toml doit rester du TOML");
    assert!(parsed["dependencies"].get("serde").is_some(), "{cargo}");
    assert!(parsed["dependencies"].get("serde_json_lenient").is_some(), "{cargo}");
    assert!(parsed.get("profile").is_some(), "le bloc profile doit survivre : {cargo}");

    // Deux fois de suite ne duplique rien.
    scaffold::add_settings_module(&root).expect("la seconde fois ne doit rien casser");
    let cargo = std::fs::read_to_string(root.join("Cargo.toml")).unwrap();
    assert_eq!(cargo.matches("serde_json_lenient").count(), 1, "{cargo}");
    let main_rs = std::fs::read_to_string(root.join("src/main.rs")).unwrap();
    assert_eq!(main_rs.matches("mod settings;").count(), 1, "{main_rs}");

    // Il ne doit rien devoir à maxx.
    let body = std::fs::read_to_string(root.join("src/settings.rs")).unwrap();
    assert!(!body.contains("maxx::"), "les réglages ne doivent rien devoir à maxx");
}

#[test]
fn a_dropdown_declares_the_field_it_needs() {
    let root = scratch("maxx_select_field");
    scaffold::create_project(&root, "essai").expect("le projet doit être créé");
    let path = root.join("src/ui/home.rs");

    let mut view = View::load(&path).expect("la vue doit se relire");
    let select =
        maxx::registry::by_id("select").expect("la liste déroulante doit être au catalogue");
    view.root.children.push(maxx::registry::instantiate("select").unwrap());

    // Lier le champ, comme le fait l'inspecteur.
    let index = view.root.children.len() - 1;
    let prop = select.props.iter().find(|prop| prop.label == "Champ lié").unwrap();
    maxx::registry::write_binding(&mut view.root.children[index], prop, Some("&self.pays"));
    view.save().expect("la vue doit s'enregistrer");

    let source = std::fs::read_to_string(&path).unwrap();
    assert!(
        source.contains("pub pays: Entity<SelectState<SearchableVec<SharedString>>>,"),
        "{source}"
    );
    assert!(source.contains("SelectState::new("), "{source}");
    assert!(
        source.contains("use gpui_component::select::{SearchableVec, SelectState};"),
        "{source}"
    );
    assert!(source.contains("use gpui_component::select::Select;"), "{source}");
    // `new` prend ses arguments : le gabarit les ignorait.
    assert!(source.contains("pub fn new(window: &mut Window, cx: &mut Context<Self>)"), "{source}");

    // Et une seconde liste ne redéclare pas le même champ.
    let mut view = View::load(&path).expect("la vue doit se relire");
    view.root.children.push(maxx::registry::instantiate("select").unwrap());
    let index = view.root.children.len() - 1;
    maxx::registry::write_binding(&mut view.root.children[index], prop, Some("&self.pays"));
    view.save().expect("la vue doit s'enregistrer");
    let source = std::fs::read_to_string(&path).unwrap();
    assert_eq!(source.matches("pub pays:").count(), 1, "{source}");
}
