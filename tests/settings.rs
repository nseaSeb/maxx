//! The settings have to survive a round trip through the disk, a missing, empty
//! or damaged file — and above all, writing one key must change nothing else in
//! the file.

use std::path::PathBuf;

use maxx::settings::{Preferences, State, append_key, patch_preferences, splice_key};

#[test]
fn writing_a_key_leaves_every_other_byte_alone() {
    let source = r#"// My own settings.
{
  "$schema": "./settings-schema.json",

  // I am fond of this comment.
  "show_project_panel": true,

  "show_status_bar": true,
  "show_output": false,
  "editor": "auto",
  "terminal": "auto",
  "format_on_save": true,
  "language": "system",
  "theme": "system"
}
"#;

    let preferences = Preferences { show_project_panel: false, ..Preferences::default() };
    let patched = patch_preferences(source, &preferences);

    assert!(patched.contains("// My own settings."));
    assert!(patched.contains("// I am fond of this comment."));
    assert!(patched.contains("\"$schema\": \"./settings-schema.json\""));
    assert!(patched.contains("\"show_project_panel\": false"));
    // The other two did not change value, so no reformatting.
    assert!(patched.contains("\"show_status_bar\": true"));
    assert!(patched.contains("\"show_output\": false"));
    assert_eq!(patched.lines().count(), source.lines().count());
}

#[test]
fn a_missing_key_is_added_rather_than_the_file_rewritten() {
    let source = "{\n  \"show_output\": true\n}\n";
    let patched = patch_preferences(source, &Preferences::default());

    assert!(patched.contains("\"show_project_panel\": true"), "{patched}");
    assert!(patched.contains("\"show_status_bar\": true"), "{patched}");
    // The key that was there was updated in place, not duplicated.
    assert_eq!(patched.matches("\"show_output\"").count(), 1, "{patched}");
    assert!(patched.contains("\"show_output\": false"), "{patched}");
}

#[test]
fn a_trailing_comment_on_the_line_survives() {
    // The case a comment *before* the key does not test: the walk starts after
    // the colon, so this is where it can go wrong.
    let source = "{\n  \"show_output\": false // the bottom panel\n}\n";
    let preferences = Preferences { show_output: true, ..Preferences::default() };
    let patched = patch_preferences(source, &preferences);

    assert!(patched.contains("// the bottom panel"), "{patched}");
    assert!(patched.contains("\"show_output\": true"), "{patched}");
}

#[test]
fn a_comment_that_looks_like_a_member_is_not_one() {
    // `"show_output" : to be revisited` inside a comment: a textual search finds
    // the key and the colon there, and overwrites the comment.
    let source = "{\n  // \"show_output\" : to be revisited\n  \"show_output\": false\n}\n";
    let preferences = Preferences { show_output: true, ..Preferences::default() };
    let patched = patch_preferences(source, &preferences);

    assert!(patched.contains("// \"show_output\" : to be revisited"), "{patched}");
    assert!(patched.contains("\"show_output\": true"), "{patched}");
    let reread: Preferences =
        serde_json_lenient::from_str_lenient(&patched).expect("the file stays readable");
    assert!(reread.show_output);
}

#[test]
fn an_odd_quote_in_a_comment_does_not_eat_the_closing_brace() {
    // A lone quote in a comment used to leave the walk "inside a string" until
    // the end of the file, closing brace included.
    let source = "{\n  \"show_output\": false\n  // 5\" wide\n}\n";
    let preferences = Preferences { show_output: true, ..Preferences::default() };
    let patched = patch_preferences(source, &preferences);

    assert!(patched.trim_end().ends_with('}'), "{patched}");
    let reread: Preferences =
        serde_json_lenient::from_str_lenient(&patched).expect("the file stays readable");
    assert!(reread.show_output);
}

#[test]
fn a_comment_holding_a_brace_does_not_derail_the_patch() {
    let source = "{\n  \"show_output\": false\n  /* a } and a \" here */\n}\n";
    let preferences = Preferences { show_output: true, ..Preferences::default() };
    let patched = patch_preferences(source, &preferences);

    assert!(patched.contains("/* a } and a \" here */"), "{patched}");
    assert!(patched.contains("\"show_output\": true"), "{patched}");
}

#[test]
fn a_key_added_next_to_a_trailing_comment_stays_valid_json() {
    // The comma added at the end of the object used to land inside the comment,
    // so commented out: two members with no separator.
    let source = "{\n  \"show_project_panel\": true\n  // TODO: add show_output\n}\n";
    let patched = patch_preferences(source, &Preferences::default());

    assert!(patched.contains("// TODO: add show_output"), "{patched}");
    let reread: Preferences =
        serde_json_lenient::from_str_lenient(&patched).expect("the file stays readable");
    assert_eq!(reread, Preferences::default());
}

#[test]
fn splicing_stops_at_the_end_of_the_value_it_replaces() {
    let source = "{\n  \"a\": [1, {\"b\": 2}],\n  \"c\": 3\n}";
    let patched = splice_key(source, "a", "[]").expect("the key is there");
    assert_eq!(patched, "{\n  \"a\": [],\n  \"c\": 3\n}");

    assert!(splice_key(source, "missing", "1").is_none());
}

#[test]
fn appending_a_key_to_an_empty_object_stays_valid() {
    assert_eq!(append_key("{}", "a", "1"), "{\n  \"a\": 1}");

    // Added at the head, not at the tail: it is the only position no comment at
    // the end of the object can spoil.
    let patched = append_key("{\n  \"a\": 1\n}", "b", "2");
    let value: serde_json_lenient::Value =
        serde_json_lenient::from_str_lenient(&patched).expect("{patched}");
    assert_eq!(value["a"], 1);
    assert_eq!(value["b"], 2);
}

#[test]
fn the_documented_defaults_are_readable_and_hold_the_defaults() {
    let source = maxx::settings::documented_defaults();
    assert!(source.contains("// maxx settings."), "{source}");

    let preferences: Preferences =
        serde_json_lenient::from_str_lenient(&source).expect("the comments are tolerated");
    assert_eq!(preferences, Preferences::default());
}

#[test]
fn a_damaged_file_falls_back_to_the_defaults() {
    let path = scratch_file("maxx_settings_damaged.json");
    std::fs::write(&path, "this is not JSON = = =\n").unwrap();

    let preferences: Preferences = maxx::settings::read_json(&path);
    assert_eq!(preferences, Preferences::default());
    // The damaged file stays on the disk: overwriting it would lose whatever the
    // user was in the middle of writing.
    assert!(path.exists());
}

#[test]
fn a_partial_file_keeps_the_defaults_for_what_it_omits() {
    let path = scratch_file("maxx_settings_partial.json");
    std::fs::write(&path, "{ \"show_output\": true }\n").unwrap();

    let preferences: Preferences = maxx::settings::read_json(&path);
    assert!(preferences.show_output);
    assert!(preferences.show_project_panel, "default lost");
    assert!(preferences.show_status_bar, "default lost");
}

#[test]
fn the_recent_list_moves_deduplicates_and_stops_at_ten() {
    let mut state = State::default();

    assert!(state.remember_project(&PathBuf::from("/tmp/one")));
    assert!(state.remember_project(&PathBuf::from("/tmp/two")));
    assert_eq!(state.recent_projects, vec![PathBuf::from("/tmp/two"), PathBuf::from("/tmp/one")]);

    // Reopening the one already at the head changes nothing — so neither a file
    // rewritten nor a menu bar rebuilt.
    assert!(!state.remember_project(&PathBuf::from("/tmp/two")));

    // Reopening an older one brings it back up, without duplicating it.
    assert!(state.remember_project(&PathBuf::from("/tmp/one")));
    assert_eq!(state.recent_projects, vec![PathBuf::from("/tmp/one"), PathBuf::from("/tmp/two")]);

    for index in 0..15 {
        state.remember_project(&PathBuf::from(format!("/tmp/project_{index}")));
    }
    assert_eq!(state.recent_projects.len(), 10);
    assert_eq!(
        state.recent_projects[0],
        PathBuf::from("/tmp/project_14"),
        "the most recent has to be at the head"
    );
}

#[test]
fn a_project_that_no_longer_exists_leaves_the_list() {
    let root = scratch_file("maxx_settings_missing");
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();

    let mut state =
        State { recent_projects: vec![root.clone(), root.join("gone")], ..State::default() };
    state.forget_missing_projects();

    assert_eq!(state.recent_projects, vec![root]);
}

#[test]
fn a_hand_written_file_with_every_trap_at_once_survives() {
    // The three traps at once: a key and a colon inside a comment, an odd quote
    // in that same comment, and an end-of-line comment right after a value.
    let source = r#"// My own file.
{
  "$schema": "./settings-schema.json",

  // "show_output" : to be revisited one day — 5" wide
  "show_project_panel": true, // the explorer
  "show_status_bar": false
}
"#;

    let preferences = Preferences {
        show_project_panel: false,
        show_status_bar: false,
        show_output: true,
        ..Preferences::default()
    };
    let patched = patch_preferences(source, &preferences);

    assert!(patched.contains("// My own file."), "{patched}");
    assert!(
        patched.contains(r#"// "show_output" : to be revisited one day — 5" wide"#),
        "{patched}"
    );
    assert!(patched.contains("// the explorer"), "{patched}");
    assert!(patched.contains(r#""$schema": "./settings-schema.json""#), "{patched}");

    let reread: Preferences =
        serde_json_lenient::from_str_lenient(&patched).expect("the file stays readable: {patched}");
    assert_eq!(reread, preferences);
}

#[test]
fn a_brace_in_the_header_comment_does_not_anchor_the_walk() {
    // A user writing `// editor: {code, zed}` above the opening brace used to
    // anchor the whole walk inside the comment.
    let source = "// editor: {code, zed}\n{\n  \"show_output\": false\n}\n";
    let preferences = Preferences { show_output: true, ..Preferences::default() };
    let patched = patch_preferences(source, &preferences);

    assert!(patched.starts_with("// editor: {code, zed}\n{"), "{patched}");
    let reread: Preferences =
        serde_json_lenient::from_str_lenient(&patched).expect("the file stays readable: {patched}");
    assert!(reread.show_output);
}

#[test]
fn the_panel_sizes_survive_a_round_trip_and_default_when_absent() {
    let state = State {
        panel_width: Some(300.),
        inspector_width: Some(320.),
        output_height: Some(260.),
        menu_inspector_width: Some(420.),
        ..State::default()
    };
    // The very writer `save_state` uses, so what the file holds is what this
    // produces.
    let body = serde_json_lenient::to_string_pretty(&state).unwrap();
    let path = scratch_file("maxx_state_sizes.json");
    std::fs::write(&path, format!("{body}\n")).unwrap();

    let read: State = maxx::settings::read_json(&path);
    assert_eq!(read, state);

    // A `state.json` written by an older maxx knows neither key; the handles
    // then start where they always did rather than at zero.
    let old = scratch_file("maxx_state_old.json");
    std::fs::write(&old, "{ \"panel_width\": 300.0 }\n").unwrap();
    let read: State = maxx::settings::read_json(&old);
    assert_eq!(read.panel_width, Some(300.));
    assert_eq!(read.output_height, None, "an absent key is the default, not a zero size");
    assert_eq!(read.menu_inspector_width, None, "an absent key is the default, not a zero size");
}

/// A directory of this run's own, under `MAXX_SCRATCH` when it is set.
///
/// Fixed names under `temp_dir()` collide whenever two `cargo test` runs
/// overlap — a second checkout, a CI job beside a local run — and the failure
/// then lands on whichever test read a file the other had just removed. The
/// pid separates two runs even when the variable is unset. Repeated per file
/// because each integration test is a crate of its own.
fn scratch_file(name: &str) -> std::path::PathBuf {
    let root = std::env::var_os("MAXX_SCRATCH")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(std::env::temp_dir);
    let directory = root.join(format!("maxx-{}-{}", "settings", std::process::id()));
    std::fs::create_dir_all(&directory).expect("the test directory must be created");
    directory.join(name)
}
