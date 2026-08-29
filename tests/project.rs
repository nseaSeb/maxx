//! `maxx.toml` carries the project, and not only what it copied from maxx: the
//! view its window opens on, and the cargo line that launches it. Both used to
//! be written in stone — the entry inside `main.rs`, the launch as a bare
//! `cargo run` — so both are guarded here.

use std::path::PathBuf;

use maxx::projectfile::{self, ProjectFile, Run};
use maxx::scaffold::{self, Template};
use maxx::view::View;

fn scratch(name: &str) -> PathBuf {
    let dir =
        std::env::var("MAXX_SCRATCH").map(PathBuf::from).unwrap_or_else(|_| std::env::temp_dir());
    let path = dir.join(name);
    let _ = std::fs::remove_dir_all(&path);
    path
}

#[test]
fn a_project_that_says_nothing_is_launched_as_before() {
    let run = Run::default();
    assert_eq!(run.arguments("run"), vec!["run"]);
    assert_eq!(run.arguments("build"), vec!["build"]);
}

#[test]
fn the_run_section_becomes_a_cargo_line() {
    let run = Run {
        profile: Some("release".into()),
        features: vec!["demo".into(), "tracing".into()],
        default_features: false,
        args: vec!["--verbose".into()],
    };

    assert_eq!(
        run.arguments("run"),
        vec![
            "run",
            "--profile",
            "release",
            "--no-default-features",
            "--features",
            "demo,tracing",
            "--",
            "--verbose",
        ]
    );
}

#[test]
fn a_prewarm_builds_the_same_thing_but_hands_nothing_over() {
    let run = Run {
        profile: Some("release".into()),
        features: vec!["demo".into()],
        default_features: true,
        args: vec!["--verbose".into()],
    };

    // The profile and the features have to be there, or the prewarm fills a
    // cache the run will not use; the application's own arguments must not,
    // because `cargo build -- --verbose` is refused.
    assert_eq!(run.arguments("build"), vec!["build", "--profile", "release", "--features", "demo"]);
}

#[test]
fn the_run_section_is_read_from_the_file() {
    let root = scratch("maxx_project_run_read");
    std::fs::create_dir_all(&root).unwrap();
    std::fs::write(
        projectfile::path(&root),
        "[run]\nprofile = \"release\"\ndefault-features = false\nfeatures = [\"demo\"]\n",
    )
    .unwrap();

    assert_eq!(
        projectfile::arguments(&root, "run"),
        vec!["run", "--profile", "release", "--no-default-features", "--features", "demo"]
    );
}

#[test]
fn a_file_written_before_this_still_carries_its_modules() {
    let root = scratch("maxx_project_old_file");
    std::fs::create_dir_all(&root).unwrap();
    // The shape maxx wrote when the file held nothing but the copied modules.
    std::fs::write(
        projectfile::path(&root),
        "[modules.system]\nversion = 1\nfingerprint = \"abcdef0123456789\"\n",
    )
    .unwrap();

    // Recording something else must not lose it, and the sections nobody filled
    // in must not appear.
    projectfile::set_entry(&root, "home").unwrap();

    let written = std::fs::read_to_string(projectfile::path(&root)).unwrap();
    assert!(written.contains("abcdef0123456789"), "the recorded module was lost:\n{written}");
    assert!(written.contains("entry = \"home\""), "the entry was not recorded:\n{written}");
    // The header shows `[run]` as an example, so the section itself is looked
    // for as a line of its own.
    assert!(
        !written.lines().any(|line| line.trim() == "[run]"),
        "an empty section was written:\n{written}"
    );
}

#[test]
fn an_unknown_key_does_not_cost_the_whole_file() {
    let root = scratch("maxx_project_unknown_key");
    std::fs::create_dir_all(&root).unwrap();
    std::fs::write(projectfile::path(&root), "[project]\nentry = \"home\"\nfuture = 12\n").unwrap();

    // A key from a later maxx is ignored rather than fatal: the alternative is
    // a project whose recorded modules all disappear on the next save.
    assert_eq!(projectfile::entry(&root).as_deref(), Some("home"));
}

#[test]
fn a_new_project_says_which_view_it_opens_on() {
    let root = scratch("maxx_project_entry_new");
    scaffold::create_project(&root, "trial", Template::Empty).unwrap();

    assert_eq!(projectfile::entry(&root).as_deref(), Some("home"));
    let main = std::fs::read_to_string(root.join("src/main.rs")).unwrap();
    assert!(main.contains("use crate::ui::home::Home;"));
}

#[test]
fn the_window_can_be_pointed_at_another_view() {
    let root = scratch("maxx_project_entry_change");
    scaffold::create_project(&root, "trial", Template::Empty).unwrap();
    scaffold::create_view(&root, "settings_screen").unwrap();

    scaffold::set_entry_view(&root, "settings_screen").expect("the entry must move");

    let main = std::fs::read_to_string(root.join("src/main.rs")).unwrap();
    assert!(main.contains("use crate::ui::settings_screen::SettingsScreen;"), "{main}");
    assert!(main.contains("SettingsScreen::new(window, cx)"), "{main}");
    // The old import has to go, or the file no longer compiles without a
    // warning — and names a view the window does not open.
    assert!(!main.contains("use crate::ui::home::Home;"), "{main}");
    assert!(!main.contains("Home::new(window, cx)"), "{main}");

    assert_eq!(projectfile::entry(&root).as_deref(), Some("settings_screen"));
}

#[test]
fn the_entry_is_read_from_the_view_rather_than_from_its_name() {
    let root = scratch("maxx_project_entry_type");
    scaffold::create_project(&root, "trial", Template::Empty).unwrap();
    scaffold::create_view(&root, "second").unwrap();
    // A view adopted from a project maxx did not write is called whatever its
    // author called it; deriving the type from the module name would import a
    // type that does not exist.
    let path = root.join("src/ui/second.rs");
    let source = std::fs::read_to_string(&path).unwrap().replace("Second", "Dashboard");
    std::fs::write(&path, source).unwrap();

    scaffold::set_entry_view(&root, "second").unwrap();

    let main = std::fs::read_to_string(root.join("src/main.rs")).unwrap();
    assert!(main.contains("use crate::ui::second::Dashboard;"), "{main}");
    assert!(main.contains("Dashboard::new(window, cx)"), "{main}");
}

#[test]
fn a_main_maxx_cannot_read_is_left_alone() {
    let root = scratch("maxx_project_entry_refused");
    scaffold::create_project(&root, "trial", Template::Empty).unwrap();
    scaffold::create_view(&root, "second").unwrap();

    // A `main.rs` that opens its window some other way: maxx says so instead of
    // writing a `maxx.toml` claiming an entry the code does not open.
    let main_path = root.join("src/main.rs");
    std::fs::write(&main_path, "fn main() {\n    println!(\"hand written\");\n}\n").unwrap();

    let error = scaffold::set_entry_view(&root, "second").expect_err("this cannot be guessed");
    assert!(error.to_string().contains("main.rs"), "{error}");
    assert_eq!(projectfile::entry(&root).as_deref(), Some("home"), "the record must not move");
}

#[test]
fn the_file_keeps_its_sections_in_an_order_toml_accepts() {
    // Every section is a table, and a table may not be followed by a bare
    // value: a file maxx writes has to be a file maxx reads back.
    let root = scratch("maxx_project_round_trip");
    std::fs::create_dir_all(&root).unwrap();

    let mut file = ProjectFile::default();
    file.project.entry = Some("home".into());
    file.run.profile = Some("release".into());
    file.run.args = vec!["--verbose".into()];
    projectfile::record(&root, "system", 1, "body").unwrap();
    projectfile::save(&root, &file).unwrap();

    let written = std::fs::read_to_string(projectfile::path(&root)).unwrap();
    let read: ProjectFile = toml::from_str(&written).expect("maxx must read its own file");
    assert_eq!(read, file);
}

/// A `main.rs` that names its view in full instead of importing it.
fn qualify_entry(root: &std::path::Path) {
    let main_path = root.join("src/main.rs");
    let source = std::fs::read_to_string(&main_path)
        .unwrap()
        .replace("use crate::ui::home::Home;\n", "")
        .replace("Home::new(window, cx)", "crate::ui::home::Home::new(window, cx)");
    std::fs::write(&main_path, source).unwrap();
}

#[test]
fn a_view_named_in_full_is_replaced_whole() {
    let root = scratch("maxx_project_entry_qualified");
    scaffold::create_project(&root, "trial", Template::Empty).unwrap();
    scaffold::create_view(&root, "second").unwrap();
    qualify_entry(&root);

    scaffold::set_entry_view(&root, "second").expect("the entry must move");

    let main = std::fs::read_to_string(root.join("src/main.rs")).unwrap();
    // Replacing only the last segment would leave the old module holding the
    // new type, which does not compile.
    assert!(!main.contains("crate::ui::home::Second"), "{main}");
    assert!(main.contains("Second::new(window, cx)"), "{main}");
    // And the import has to arrive, since the call no longer carries the path.
    assert_eq!(
        main.matches("use crate::ui::second::Second;").count(),
        1,
        "the import is missing or written twice:\n{main}"
    );
}

#[test]
fn a_view_already_imported_is_not_imported_twice() {
    let root = scratch("maxx_project_entry_twice");
    scaffold::create_project(&root, "trial", Template::Empty).unwrap();
    scaffold::create_view(&root, "second").unwrap();

    // A `main.rs` naming several views, which is the case the doc anticipates.
    let main_path = root.join("src/main.rs");
    let source = std::fs::read_to_string(&main_path).unwrap().replace(
        "use crate::ui::home::Home;",
        "use crate::ui::home::Home;\nuse crate::ui::second::Second;",
    );
    std::fs::write(&main_path, source).unwrap();

    scaffold::set_entry_view(&root, "second").expect("the entry must move");

    let main = std::fs::read_to_string(root.join("src/main.rs")).unwrap();
    assert_eq!(
        main.matches("use crate::ui::second::Second;").count(),
        1,
        "a second import is E0252:\n{main}"
    );
    assert!(!main.contains("use crate::ui::home::Home;"), "{main}");
    assert!(main.contains("Second::new(window, cx)"), "{main}");
}

#[test]
fn setting_the_same_view_twice_changes_nothing() {
    let root = scratch("maxx_project_entry_idempotent");
    scaffold::create_project(&root, "trial", Template::Empty).unwrap();

    scaffold::set_entry_view(&root, "home").unwrap();
    let once = std::fs::read_to_string(root.join("src/main.rs")).unwrap();
    scaffold::set_entry_view(&root, "home").unwrap();
    let twice = std::fs::read_to_string(root.join("src/main.rs")).unwrap();

    assert_eq!(once, twice);
    assert!(twice.contains("use crate::ui::home::Home;"), "{twice}");
}

#[test]
fn the_entry_is_the_view_handed_to_root() {
    let root = scratch("maxx_project_entry_root");
    scaffold::create_project(&root, "trial", Template::Empty).unwrap();
    scaffold::create_view(&root, "second").unwrap();

    // A `main.rs` that builds something else before its root view: the first
    // `::new(window, cx)` of the file is not the window's view.
    let main_path = root.join("src/main.rs");
    let source = std::fs::read_to_string(&main_path).unwrap().replace(
        "                let view = cx.new(|cx| Home::new(window, cx));",
        "                let toolbar = cx.new(|cx| Toolbar::new(window, cx));\n\
                 let _ = &toolbar;\n\
                 let view = cx.new(|cx| Home::new(window, cx));",
    );
    std::fs::write(&main_path, source).unwrap();

    scaffold::set_entry_view(&root, "second").expect("the entry must move");

    let main = std::fs::read_to_string(root.join("src/main.rs")).unwrap();
    assert!(main.contains("Toolbar::new(window, cx)"), "the toolbar was rewritten:\n{main}");
    assert!(main.contains("let view = cx.new(|cx| Second::new(window, cx));"), "{main}");
}

#[test]
fn a_view_type_is_the_one_that_renders() {
    let root = scratch("maxx_project_entry_helper");
    scaffold::create_project(&root, "trial", Template::Empty).unwrap();
    scaffold::create_view(&root, "second").unwrap();

    // A helper declared above the view, as a file maxx did not write may well
    // do: importing and building it would not compile.
    let path = root.join("src/ui/second.rs");
    let source = std::fs::read_to_string(&path)
        .unwrap()
        .replace("pub struct Second {}", "pub struct Row {}\n\npub struct Second {}");
    std::fs::write(&path, source).unwrap();

    scaffold::set_entry_view(&root, "second").unwrap();

    let main = std::fs::read_to_string(root.join("src/main.rs")).unwrap();
    assert!(main.contains("use crate::ui::second::Second;"), "{main}");
    assert!(!main.contains("Row"), "{main}");
}

#[test]
fn a_file_that_does_not_parse_is_never_written_over() {
    let root = scratch("maxx_project_broken_file");
    scaffold::create_project(&root, "trial", Template::Empty).unwrap();
    projectfile::record(&root, "system", 1, "body").unwrap();

    // One missing bracket, hand-written: rewriting from an empty file would
    // erase every module record the project holds.
    let broken = format!(
        "{}\n[run]\nfeatures = [\"demo\"\n",
        std::fs::read_to_string(projectfile::path(&root)).unwrap()
    );
    std::fs::write(projectfile::path(&root), &broken).unwrap();

    let error = projectfile::set_entry(&root, "home").expect_err("a broken file is an error");
    assert!(error.to_string().contains("maxx.toml"), "{error}");
    projectfile::record(&root, "theme", 1, "body").expect_err("and so is recording a module");
    assert_eq!(std::fs::read_to_string(projectfile::path(&root)).unwrap(), broken);
}

#[test]
fn a_main_with_two_views_and_no_root_is_left_alone() {
    let root = scratch("maxx_project_entry_ambiguous");
    scaffold::create_project(&root, "trial", Template::Empty).unwrap();
    scaffold::create_view(&root, "second").unwrap();

    // Two candidates and nothing naming the one the window opens on: rewriting
    // either would be a guess, and `maxx.toml` would record it as a fact.
    std::fs::write(
        root.join("src/main.rs"),
        "fn main() {\n    \
         let first = Home::new(window, cx);\n    \
         let second = Other::new(window, cx);\n}\n",
    )
    .unwrap();

    scaffold::set_entry_view(&root, "second").expect_err("this cannot be guessed");
    assert_eq!(projectfile::entry(&root).as_deref(), Some("home"), "the record must not move");
}

#[test]
fn the_sidebar_shape_hangs_two_views_off_a_shell() {
    let root = scratch("maxx_template_sidebar");
    scaffold::create_project(&root, "trial", Template::Sidebar).unwrap();

    let modules = std::fs::read_to_string(root.join("src/ui/mod.rs")).unwrap();
    for module in ["home", "library", "shell"] {
        assert!(modules.contains(&format!("pub mod {module};")), "{modules}");
        assert!(root.join(format!("src/ui/{module}.rs")).exists(), "src/ui/{module}.rs is missing");
    }

    // The window opens on the shell, and `maxx.toml` says so.
    assert_eq!(projectfile::entry(&root).as_deref(), Some("shell"));
    let main = std::fs::read_to_string(root.join("src/main.rs")).unwrap();
    assert!(main.contains("use crate::ui::shell::Shell;"), "{main}");
    assert!(main.contains("Shell::new(window, cx)"), "{main}");

    let shell = std::fs::read_to_string(root.join("src/ui/shell.rs")).unwrap();
    // The name is written once, in `Page::label`, and the sidebar reads it from
    // there — the title bar shows the same one, and the two cannot drift.
    assert!(shell.contains("Self::Home => \"Home\","), "{shell}");
    assert!(shell.contains("Self::Library => \"Library\","), "{shell}");
    assert!(shell.contains("SidebarMenuItem::new(Page::Home.label())"), "{shell}");
    assert!(shell.contains("SidebarMenuItem::new(Page::Library.label())"), "{shell}");
    // The shell draws its own title bar, and `main.rs` opens the window with the
    // options that make room for it. One without the other is a doubled bar.
    assert!(shell.contains("TitleBar::new()"), "{shell}");
    assert!(main.contains("TitleBar::title_bar_options()"), "{main}");

    // Both pages stay views maxx can design: the shape is around them, not
    // instead of them.
    for module in ["home", "library"] {
        View::load(&root.join(format!("src/ui/{module}.rs")))
            .unwrap_or_else(|error| panic!("{module} must read back: {error}"));
    }
}

#[test]
fn the_settings_shape_brings_its_module_with_it() {
    let root = scratch("maxx_template_settings");
    scaffold::create_project(&root, "trial", Template::Settings).unwrap();

    // The screen is written against the module, so the module has to be there
    // — and the system module under it, which is what knows where the file
    // goes.
    assert!(root.join("src/settings.rs").exists());
    assert!(root.join("src/system.rs").exists());
    let file = projectfile::load(&root);
    assert!(file.modules.contains_key("settings"), "the module was not recorded");
    assert!(file.modules.contains_key("system"), "the module was not recorded");

    let main = std::fs::read_to_string(root.join("src/main.rs")).unwrap();
    assert!(main.contains("mod settings;"), "{main}");
    assert!(main.contains("Shell::new(window, cx)"), "{main}");

    let screen = std::fs::read_to_string(root.join("src/ui/settings_screen.rs")).unwrap();
    assert!(screen.contains("settings::save(&self.settings)"), "{screen}");
    assert_eq!(projectfile::entry(&root).as_deref(), Some("shell"));

    let cargo = std::fs::read_to_string(root.join("Cargo.toml")).unwrap();
    assert!(cargo.contains("serde"), "the module's crates were not declared:\n{cargo}");
}

#[test]
fn the_empty_shape_is_what_it_always_was() {
    let root = scratch("maxx_template_empty");
    scaffold::create_project(&root, "trial", Template::Empty).unwrap();

    assert!(!root.join("src/ui/shell.rs").exists(), "the empty shape holds no shell");
    assert_eq!(std::fs::read_to_string(root.join("src/ui/mod.rs")).unwrap(), "pub mod home;\n");
    assert_eq!(projectfile::entry(&root).as_deref(), Some("home"));
}

/// The deep proof, and the slow one: what maxx writes has to compile.
///
/// Ignored by default — it is a `cargo check` over some 750 crates, minutes on
/// a cold cache — and run by hand after touching a template:
/// `cargo test --test project -- --ignored`. What it catches that
/// `examples/shapes.rs` cannot is the wiring: `main.rs`, `src/ui/mod.rs`, the
/// settings module and its crates, all of it together.
#[test]
#[ignore = "builds two whole projects"]
fn every_shape_compiles() {
    for (name, template) in [
        ("maxx_template_build_sidebar", Template::Sidebar),
        ("maxx_template_build_settings", Template::Settings),
    ] {
        let root = scratch(name);
        scaffold::create_project(&root, "trial", template).unwrap();

        let status = std::process::Command::new("cargo")
            .arg("check")
            .current_dir(&root)
            .status()
            .expect("cargo must run");
        assert!(status.success(), "{} does not compile", template.name());
    }
}

/// The component library, used by a view, compiled for real.
///
/// Ignored like its neighbours, and run after touching `templates::COMPONENTS`:
/// `cargo test --test project -- --ignored`. `examples/shapes.rs` proves each
/// brick compiles on its own; this proves the rest — the `mod.rs` maxx
/// generates, the re-exports a view writes against, the `mod components;` line
/// in `main.rs`, and the palette the bricks reach for.
#[test]
#[ignore = "builds a whole project"]
fn the_component_library_compiles_where_a_view_uses_it() {
    let root = scratch("maxx_components_build");
    scaffold::create_project(&root, "trial", Template::Empty).unwrap();
    scaffold::add_components_module(&root).expect("the components must be added");

    // Used, and not merely present: a library that compiles alone and not at
    // its call site has proved nothing about the shape it offers.
    let path = root.join("src/ui/home.rs");
    let source = std::fs::read_to_string(&path).unwrap();
    let source = source.replace(
        "use gpui_component::v_flex;",
        "use gpui_component::v_flex;\n\nuse crate::components::{Card, EmptyState, Toolbar};",
    );
    let source = source.replace(
        "            .child(Label::new(\"Welcome\"))",
        "            .child(Toolbar::new().separated().child(Label::new(\"Bar\")))\n\
         \x20           .child(Card::new(\"Title\").subtitle(\"Subtitle\").child(Label::new(\"In\")))\n\
         \x20           .child(EmptyState::new(\"Nothing here\").hint(\"Add something\"))",
    );
    assert!(source.contains("Card::new"), "the view must actually use the library");
    std::fs::write(&path, source).unwrap();

    let status = std::process::Command::new("cargo")
        .arg("check")
        .current_dir(&root)
        .status()
        .expect("cargo must run");
    assert!(status.success(), "a view using the component library must compile");
}

/// A component of the project, dropped from the palette, compiled for real.
///
/// The loop the library was written to close: maxx writes the bricks, reads
/// them back out of the developer's own source, offers them, and what it drops
/// is a view that builds. Ignored like its neighbours:
/// `cargo test --test project -- --ignored`.
#[test]
#[ignore = "builds a whole project"]
fn a_brick_dropped_from_the_palette_compiles() {
    let root = scratch("maxx_brick_drop");
    scaffold::create_project(&root, "trial", Template::Empty).unwrap();
    scaffold::add_components_module(&root).expect("the components must be added");

    let bricks = maxx::bricks::read(&root);
    assert!(!bricks.is_empty(), "the library maxx just wrote must be readable");

    let path = root.join("src/ui/home.rs");
    let mut view = View::load(&path).expect("the fresh view must read back");
    for brick in &bricks {
        let node = maxx::parser::parse_expr(&brick.expression())
            .unwrap_or_else(|error| panic!("{}: {error}", brick.type_name));
        assert!(!node.is_opaque(), "{}", brick.expression());
        let at = vec![view.root.children.len()];
        assert!(view.root.insert(&at, node));
        view.extra_imports.push(brick.import());
    }
    view.save().expect("the view must save");

    let status = std::process::Command::new("cargo")
        .arg("check")
        .current_dir(&root)
        .status()
        .expect("cargo must run");
    assert!(status.success(), "a view built from the project's own bricks must compile");
}

/// The whole catalogue, dropped into one view, compiled for real.
///
/// Ignored like its neighbour above, and run by hand after touching the
/// tables: `cargo test --test project -- --ignored`. `examples/catalogue.rs`
/// proves each call exists on its type; this proves the rest — the `use` lines
/// maxx writes, the state fields it declares on the view, the initializers it
/// gives them, and the handler stubs — which no amount of table reading can
/// answer.
#[test]
#[ignore = "builds a whole project"]
fn every_component_of_the_catalogue_compiles_where_maxx_puts_it() {
    let root = scratch("maxx_catalogue_build");
    scaffold::create_project(&root, "trial", Template::Empty).unwrap();

    let path = root.join("src/ui/home.rs");
    let mut view = View::load(&path).expect("the fresh view must read back");
    for spec in maxx::registry::CATALOGUE {
        let mut node =
            maxx::registry::instantiate(spec.id).unwrap_or_else(|| panic!("{}", spec.id));
        // The same rename the workspace does on a drop: two entities sharing
        // one field compile but mirror each other, and the second field would
        // never be declared.
        if spec.state.is_some()
            && let maxx::model::Base::Known { args, .. } = &mut node.base
        {
            let field = maxx::registry::unique_input_field(&view.root);
            *args = vec![maxx::model::Arg::Verbatim(format!("&self.{field}"))];
        }
        let end = view.root.children.len();
        assert!(view.root.insert(&[end], node), "{} must go into the view", spec.id);
    }
    view.save().expect("the view must be written");

    let output = std::process::Command::new("cargo")
        .arg("check")
        .current_dir(&root)
        .output()
        .expect("cargo must run");
    let report = String::from_utf8_lossy(&output.stderr);
    assert!(output.status.success(), "the catalogue does not compile:\n{report}");
    // And it compiles clean: an import written for a call the node does not
    // make is a warning in a project maxx has just written, which is the
    // developer being handed maxx's untidiness.
    assert!(
        !report.contains("warning:"),
        "what maxx wrote does not compile without warnings:\n{report}"
    );
}

#[test]
fn a_view_imported_inside_a_group_is_not_imported_again() {
    let root = scratch("maxx_project_entry_group");
    scaffold::create_project(&root, "trial", Template::Empty).unwrap();
    scaffold::create_view(&root, "second").unwrap();

    // What a `rustfmt` with `imports_granularity = "Crate"` leaves, and what a
    // developer writes by hand: one statement for both views.
    let main_path = root.join("src/main.rs");
    let source = std::fs::read_to_string(&main_path)
        .unwrap()
        .replace("use crate::ui::home::Home;", "use crate::ui::{home::Home, second::Second};");
    std::fs::write(&main_path, source).unwrap();

    scaffold::set_entry_view(&root, "second").expect("the entry must move");

    let main = std::fs::read_to_string(&main_path).unwrap();
    assert!(
        !main.contains("\nuse crate::ui::second::Second;"),
        "a second import of the same type is E0252:\n{main}"
    );
    assert!(main.contains("Second::new(window, cx)"), "{main}");
}

#[test]
fn a_broken_file_stops_the_entry_before_main_is_touched() {
    let root = scratch("maxx_project_entry_broken_file");
    scaffold::create_project(&root, "trial", Template::Empty).unwrap();
    scaffold::create_view(&root, "second").unwrap();

    let before = std::fs::read_to_string(root.join("src/main.rs")).unwrap();
    std::fs::write(projectfile::path(&root), "[run]\nfeatures = [\"demo\"\n").unwrap();

    scaffold::set_entry_view(&root, "second").expect_err("an unreadable file is an error");

    // Otherwise the code opens the new view while the file still names the old
    // one — the exact disagreement the write order exists to prevent.
    assert_eq!(std::fs::read_to_string(root.join("src/main.rs")).unwrap(), before);
}

#[test]
fn a_run_field_is_taken_as_a_person_writes_it() {
    use maxx::projectfile::{Run, RunField};

    let mut run = Run::default();

    // Empty is no profile at all, not the empty one: a project that says
    // nothing gets cargo's own default.
    run.set(RunField::Profile, "  release  ");
    assert_eq!(run.profile.as_deref(), Some("release"));
    run.set(RunField::Profile, "   ");
    assert_eq!(run.profile, None);

    // A trailing separator leaves an empty piece behind, and half a list typed
    // is not half a feature.
    run.set(RunField::Features, " metal ,  wayland , ");
    assert_eq!(run.features, ["metal", "wayland"]);
    run.set(RunField::Features, "");
    assert!(run.features.is_empty());

    run.set(RunField::Args, "--verbose   --colour always");
    assert_eq!(run.args, ["--verbose", "--colour", "always"]);
}

#[test]
fn a_run_field_comes_back_the_way_it_went_in() {
    use maxx::projectfile::{Run, RunField};

    let mut run = Run::default();
    for (field, written) in [
        (RunField::Profile, "release"),
        (RunField::Features, "metal, wayland"),
        (RunField::Args, "--verbose --quiet"),
    ] {
        run.set(field, written);
        assert_eq!(run.get(field), written, "{field:?} must survive the round trip");
    }
}

#[test]
fn the_run_section_written_from_the_screen_reaches_the_command_line() {
    let root = scratch("maxx_run_edited");
    scaffold::create_project(&root, "trial", Template::Empty).unwrap();

    let mut file = projectfile::load(&root);
    file.run.set(maxx::projectfile::RunField::Profile, "release");
    file.run.set(maxx::projectfile::RunField::Features, "metal");
    file.run.default_features = false;
    file.run.set(maxx::projectfile::RunField::Args, "--verbose");
    projectfile::save(&root, &file).unwrap();

    assert_eq!(
        projectfile::arguments(&root, "run"),
        [
            "run",
            "--profile",
            "release",
            "--no-default-features",
            "--features",
            "metal",
            "--",
            "--verbose"
        ]
    );
    // A build has nobody to hand the application's own arguments to.
    assert_eq!(
        projectfile::arguments(&root, "build"),
        ["build", "--profile", "release", "--no-default-features", "--features", "metal"]
    );
}
