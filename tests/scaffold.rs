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
    view.root.children.push(input);
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
        view.root.children.push(node);
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
    view.root.children.push(maxx::registry::instantiate("button").unwrap());
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
    view.root.children.push(button);
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
    view.root.children.push(button);
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
