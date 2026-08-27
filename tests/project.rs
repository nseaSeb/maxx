//! `maxx.toml` carries the project, and not only what it copied from maxx: the
//! view its window opens on, and the cargo line that launches it. Both used to
//! be written in stone — the entry inside `main.rs`, the launch as a bare
//! `cargo run` — so both are guarded here.

use std::path::PathBuf;

use maxx::projectfile::{self, ProjectFile, Run};
use maxx::scaffold;

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
    scaffold::create_project(&root, "trial").unwrap();

    assert_eq!(projectfile::entry(&root).as_deref(), Some("home"));
    let main = std::fs::read_to_string(root.join("src/main.rs")).unwrap();
    assert!(main.contains("use crate::ui::home::Home;"));
}

#[test]
fn the_window_can_be_pointed_at_another_view() {
    let root = scratch("maxx_project_entry_change");
    scaffold::create_project(&root, "trial").unwrap();
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
    scaffold::create_project(&root, "trial").unwrap();
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
    scaffold::create_project(&root, "trial").unwrap();
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
