//! The generated project must be ordinary Rust: it compiles and runs without
//! `maxx`, and `maxx` can read its view back.

use std::path::PathBuf;

use maxx::{parser, scaffold, view::View};

fn scratch(name: &str) -> PathBuf {
    let dir = std::env::var("MAXX_SCRATCH")
        .map(PathBuf::from)
        .unwrap_or_else(|_| std::env::temp_dir());
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
    assert!(root.join("src/ui/accueil.rs").exists());

    let cargo = std::fs::read_to_string(root.join("Cargo.toml")).unwrap();
    assert!(
        cargo.contains("runtime_shaders"),
        "sans cette feature le projet ne compile pas sur cette machine"
    );

    let view = View::load(&root.join("src/ui/accueil.rs")).expect("la vue doit se relire");
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
    assert!(module.contains("pub mod accueil;"));
    assert!(module.contains("pub mod mon_ecran;"));

    let source = std::fs::read_to_string(root.join("src/ui/mon_ecran.rs")).unwrap();
    assert!(source.contains("pub struct MonEcran"));
    assert!(parser::locate(&source).is_ok());
}

#[test]
fn saving_a_text_input_adds_the_field_and_the_import() {
    let root = scratch("maxx_scaffold_input_test");
    scaffold::create_project(&root, "essai").unwrap();
    let path = root.join("src/ui/accueil.rs");

    let mut view = View::load(&path).unwrap();
    let input = maxx::registry::instantiate("input").expect("le champ texte est au catalogue");
    view.root.push_child(input);
    view.save().expect("l'enregistrement doit réussir");

    let source = std::fs::read_to_string(&path).unwrap();
    assert!(source.contains("use gpui_component::input::Input;"));
    assert!(source.contains("use gpui_component::input::InputState;"));
    assert!(source.contains("pub champ: Entity<InputState>,"));
    assert!(source.contains("champ: cx.new(|cx| InputState::new(window, cx)),"));
    assert!(source.contains("Input::new(&self.champ)"));

    // And it still reads back.
    let reloaded = View::load(&path).unwrap();
    assert_eq!(reloaded.root.children.len(), 2);
}

#[test]
fn every_component_of_the_catalogue_is_written_out() {
    let root = scratch("maxx_kitchen_sink");
    scaffold::create_project(&root, "essai").unwrap();
    let path = root.join("src/ui/accueil.rs");

    let mut view = View::load(&path).unwrap();
    for spec in maxx::registry::CATALOGUE {
        let node = maxx::registry::instantiate(spec.id).expect("le catalogue s'instancie");
        view.root.push_child(node);
    }
    view.save().expect("l'enregistrement doit réussir");

    let source = std::fs::read_to_string(&path).unwrap();
    for spec in maxx::registry::CATALOGUE {
        assert!(
            source.contains(spec.import),
            "l'import de {} manque",
            spec.label
        );
    }

    // And the file still parses back to the same number of nodes.
    let reloaded = View::load(&path).unwrap();
    assert_eq!(
        reloaded.root.children.len(),
        maxx::registry::CATALOGUE.len() + 1
    );
}

#[test]
fn saving_twice_produces_the_same_file() {
    let root = scratch("maxx_stable_save");
    scaffold::create_project(&root, "essai").unwrap();
    let path = root.join("src/ui/accueil.rs");

    let mut view = View::load(&path).unwrap();
    view.root.push_child(maxx::registry::instantiate("button").unwrap());
    view.save().unwrap();
    let once = std::fs::read_to_string(&path).unwrap();

    // The block sits at the markers' own indentation, not twice it.
    assert!(
        once.contains("\n            .gap_2()"),
        "indentation inattendue :\n{once}"
    );

    let mut reloaded = View::load(&path).unwrap();
    reloaded.save().unwrap();
    let twice = std::fs::read_to_string(&path).unwrap();
    assert_eq!(once, twice, "un aller-retour ne doit rien changer sur disque");
}

#[test]
fn two_text_fields_do_not_share_one_state() {
    let mut root = maxx::model::Node::known("v_flex");
    root.children
        .push(maxx::registry::instantiate("input").unwrap());
    let second = maxx::registry::unique_input_field(&root);
    assert_eq!(second, "champ_2");
}

#[test]
fn an_existing_crate_is_not_overwritten() {
    let root = scratch("maxx_no_clobber");
    scaffold::create_project(&root, "essai").unwrap();
    std::fs::write(root.join("src/ui/mod.rs"), "pub mod a_moi;\n").unwrap();

    assert!(scaffold::create_project(&root, "essai").is_err());
    assert_eq!(
        std::fs::read_to_string(root.join("src/ui/mod.rs")).unwrap(),
        "pub mod a_moi;\n"
    );
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
    let path = root.join("src/ui/accueil.rs");

    let mut view = View::load(&path).unwrap();
    let mut button = maxx::registry::instantiate("button").unwrap();
    let spec = maxx::registry::of(&button).unwrap();
    let action = spec
        .props
        .iter()
        .find(|prop| prop.label == "Action")
        .expect("le bouton a une propriété Action");

    let name = maxx::registry::suggested_handler(&button);
    assert_eq!(name, "on_bouton");
    maxx::registry::write(&mut button, action, &name);
    view.root.push_child(button);
    view.save().expect("l'enregistrement doit réussir");

    let source = std::fs::read_to_string(&path).unwrap();
    assert!(source.contains(".on_click(cx.listener(Self::on_bouton))"));
    assert!(source.contains("pub fn on_bouton("));
    assert!(source.contains("use gpui::ClickEvent;"));
    // `cx` must be named, not `_cx`, for the listener call to compile.
    assert!(source.contains("_window: &mut Window, cx: &mut Context<Self>"));

    // Saving again neither duplicates the stub nor loses what is in it.
    let mut reloaded = View::load(&path).unwrap();
    assert_eq!(
        maxx::registry::read(&reloaded.root.children[1], action).as_deref(),
        Some("on_bouton")
    );
    reloaded.save().unwrap();
    let twice = std::fs::read_to_string(&path).unwrap();
    assert_eq!(twice.matches("pub fn on_bouton(").count(), 1);
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
    let path = root.join("src/ui/accueil.rs");

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
    let width = maxx::registry::props(spec)
        .into_iter()
        .find(|prop| prop.label == "Largeur")
        .unwrap();
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

    let config = std::fs::read_to_string(root.join(".cargo/config.toml")).unwrap();
    assert!(config.contains("[build]"));
    assert!(config.contains(&maxx::run::shared_target_dir().display().to_string()));

    // The cache is machine-local, so it must not follow the project into git.
    let ignore = std::fs::read_to_string(root.join(".gitignore")).unwrap();
    assert!(ignore.contains("/.cargo"));
    assert!(ignore.contains("/target"));
}

#[test]
fn a_state_field_is_declared_and_initialised() {
    let root = scratch("maxx_state");
    scaffold::create_project(&root, "essai").unwrap();
    let path = root.join("src/ui/accueil.rs");

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
    let text = spec
        .props
        .iter()
        .find(|prop| prop.label == "Texte")
        .unwrap();

    assert_eq!(maxx::registry::read_binding(&label, text), None);

    maxx::registry::write_binding(&mut label, text, Some("self.message.clone()"));
    assert_eq!(
        maxx::codegen::render(&label, 0),
        "Label::new(self.message.clone())"
    );
    assert_eq!(
        maxx::registry::read_binding(&label, text).as_deref(),
        Some("message")
    );
    // A bound value is not editable as free text: overwriting it with a string
    // literal would silently change what the code means.
    assert!(!maxx::registry::editable(&label, text));

    maxx::registry::write_binding(&mut label, text, None);
    assert_eq!(maxx::registry::read_binding(&label, text), None);
    assert!(maxx::registry::editable(&label, text));
}

#[test]
fn the_demo_view_reads_as_a_binding() {
    // The hand-written demo is the reference for what maxx must understand.
    let path = std::path::PathBuf::from("/Users/sebastienportrait/rust/maxx-demo/src/ui/accueil.rs");
    if !path.exists() {
        return;
    }
    let view = View::load(&path).expect("la vue de démo doit se relire");

    let fields = view.state_fields();
    assert!(fields.iter().any(|field| field.name == "message"));
    assert!(fields.iter().any(|field| field.name == "clics"));

    let label = &view.root.children[0];
    let spec = maxx::registry::of(label).unwrap();
    let text = spec.props.iter().find(|p| p.label == "Texte").unwrap();
    assert_eq!(
        maxx::registry::read_binding(label, text).as_deref(),
        Some("message")
    );

    let button = &view.root.children[1];
    let spec = maxx::registry::of(button).unwrap();
    let action = spec.props.iter().find(|p| p.label == "Action").unwrap();
    assert_eq!(
        maxx::registry::read(button, action).as_deref(),
        Some("on_changer")
    );
    assert!(view.method_line("on_changer").is_some());
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
    assert!(matches!(
        maxx::parser::adopt(source),
        Err(maxx::parser::Error::NoTrailingExpression)
    ));
}

#[test]
fn an_outside_change_is_noticed() {
    let root = scratch("maxx_conflict");
    scaffold::create_project(&root, "essai").unwrap();
    let path = root.join("src/ui/accueil.rs");

    let view = View::load(&path).unwrap();
    assert!(!view.disk_changed());

    // Someone edits the file in Zed.
    let outside = std::fs::read_to_string(&path)
        .unwrap()
        .replace("Bienvenue", "Modifié dans Zed");
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
    let path = root.join("src/ui/accueil.rs");

    // A helper type declared above the view, as a developer would.
    let source = std::fs::read_to_string(&path).unwrap();
    let source = source.replace(
        "pub struct Accueil {}",
        "pub struct Ligne {\n    pub titre: String,\n}\n\nimpl Ligne {\n    pub fn nouvelle() -> Self {\n        Self {\n            titre: String::new(),\n        }\n    }\n}\n\npub struct Accueil {}",
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
    let ligne = &written[written.find("pub struct Ligne").unwrap()
        ..written.find("pub struct Accueil").unwrap()];
    assert!(!ligne.contains("champ"), "le type auxiliaire est intact :\n{ligne}");
    assert!(!ligne.contains("on_go"), "le stub ne va pas dans le type auxiliaire");

    let accueil = &written[written.find("pub struct Accueil").unwrap()..];
    assert!(accueil.contains("pub champ: Entity<InputState>,"));
    assert!(accueil.contains("pub fn on_go("));
    // And the initializer goes in the struct literal, not in the signature.
    assert!(accueil.contains("Self {\n            champ: cx.new("));
}

#[test]
fn a_state_field_is_refused_when_the_view_has_no_usable_shape() {
    let root = scratch("maxx_shape");
    scaffold::create_project(&root, "essai").unwrap();
    let path = root.join("src/ui/accueil.rs");

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
    let path = root.join("src/ui/accueil.rs");

    let source = std::fs::read_to_string(&path).unwrap().replace(
        "use gpui::{Context, Window, prelude::*};",
        "use gpui::{\n    Context,\n    Window,\n    px,\n    prelude::*,\n};",
    );
    std::fs::write(&path, &source).unwrap();

    let mut view = View::load(&path).unwrap();
    let mut button = maxx::registry::instantiate("button").unwrap();
    let spec = maxx::registry::of(&button).unwrap();
    let width = maxx::registry::props(spec)
        .into_iter()
        .find(|prop| prop.label == "Largeur")
        .unwrap();
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
    let path = root.join("src/ui/accueil.rs");

    let source = std::fs::read_to_string(&path).unwrap().replace(
        "pub struct Accueil {}",
        "pub struct AccueilConfig {\n    pub titre: String,\n}\n\npub struct Accueil {}",
    );
    std::fs::write(&path, &source).unwrap();

    let mut view = View::load(&path).unwrap();
    view.root
        .push_child(maxx::registry::instantiate("input").unwrap());
    view.save().unwrap();

    let written = std::fs::read_to_string(&path).unwrap();
    let config = &written[written.find("pub struct AccueilConfig").unwrap()
        ..written.find("pub struct Accueil {").unwrap()];
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
    assert_eq!(menus.menus[1].name, "Édition");
    assert!(menus.menus[0]
        .items
        .iter()
        .any(|item| item.label() == "Quitter"));
    assert!(!menus.dirty());

    // An entry with a brand new action declares and wires it on save.
    menus.selected = Some(maxx::menufile::Selection::Menu(0));
    menus.add_item(maxx::menu_model::ItemDef::Action {
        label: "Préférences…".into(),
        action: "OpenSettings".into(),
        os_action: None,
    });
    assert!(menus.dirty());
    menus.save().expect("l'enregistrement doit réussir");

    let source = std::fs::read_to_string(&path).unwrap();
    assert!(source.contains("MenuItem::action(\"Préférences…\", OpenSettings)"));
    assert!(source.contains("OpenSettings]"), "déclaré dans actions! : {source}");
    assert!(source.contains("cx.on_action(|_: &OpenSettings,"));

    // And it reads back, twice over, without drifting.
    let mut again = maxx::menufile::MenuFile::load(&path).unwrap();
    assert_eq!(again.menus, menus.menus);
    again.save().unwrap();
    assert_eq!(std::fs::read_to_string(&path).unwrap(), source);
}

#[test]
fn an_unknown_menu_entry_is_carried_through() {
    let root = scratch("maxx_menus_opaque");
    scaffold::create_project(&root, "essai").unwrap();
    let path = root.join("src/menus.rs");

    let source = std::fs::read_to_string(&path).unwrap().replace(
        "MenuItem::action(\"Quitter\", Quit),",
        "MenuItem::submenu(sous_menu()),\n                MenuItem::action(\"Quitter\", Quit),",
    );
    std::fs::write(&path, &source).unwrap();

    let mut menus = maxx::menufile::MenuFile::load(&path).expect("doit se relire quand même");
    assert!(menus
        .menus
        .iter()
        .flat_map(|menu| &menu.items)
        .any(|item| matches!(item, maxx::menu_model::ItemDef::Opaque(_))));

    menus.save().unwrap();
    assert!(
        std::fs::read_to_string(&path)
            .unwrap()
            .contains("MenuItem::submenu(sous_menu())"),
        "ce que maxx ne comprend pas ressort tel quel"
    );
}
