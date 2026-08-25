//! Les réglages doivent survivre à un aller-retour sur disque, à un fichier
//! absent, vide ou abîmé — et surtout, l'écriture d'une clé ne doit rien
//! changer d'autre dans le fichier.

use std::path::PathBuf;

use maxx::settings::{Preferences, State, append_key, patch_preferences, splice_key};

#[test]
fn writing_a_key_leaves_every_other_byte_alone() {
    let source = r#"// Mes réglages à moi.
{
  "$schema": "./settings-schema.json",

  // J'y tiens, à ce commentaire.
  "show_project_panel": true,

  "show_status_bar": true,
  "show_output": false,
  "editor": "auto",
  "terminal": "auto",
  "format_on_save": true
}
"#;

    let preferences = Preferences {
        show_project_panel: false,
        ..Preferences::default()
    };
    let patched = patch_preferences(source, &preferences);

    assert!(patched.contains("// Mes réglages à moi."));
    assert!(patched.contains("// J'y tiens, à ce commentaire."));
    assert!(patched.contains("\"$schema\": \"./settings-schema.json\""));
    assert!(patched.contains("\"show_project_panel\": false"));
    // Les deux autres n'ont pas bougé de valeur, donc pas de reformatage.
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
    // La valeur présente a été mise à jour sur place, pas dupliquée.
    assert_eq!(patched.matches("\"show_output\"").count(), 1, "{patched}");
    assert!(patched.contains("\"show_output\": false"), "{patched}");
}

#[test]
fn a_trailing_comment_on_the_line_survives() {
    // Le cas que le commentaire *avant* la clé ne teste pas : le balayage
    // commence après le deux-points, donc c'est ici qu'il peut déraper.
    let source = "{\n  \"show_output\": false // le panneau du bas\n}\n";
    let preferences = Preferences {
        show_output: true,
        ..Preferences::default()
    };
    let patched = patch_preferences(source, &preferences);

    assert!(patched.contains("// le panneau du bas"), "{patched}");
    assert!(patched.contains("\"show_output\": true"), "{patched}");
}

#[test]
fn a_comment_that_looks_like_a_member_is_not_one() {
    // « "show_output" : à revoir » dans un commentaire : une recherche
    // textuelle y trouve la clé et le deux-points, et écrase le commentaire.
    let source = "{\n  // \"show_output\" : à revoir\n  \"show_output\": false\n}\n";
    let preferences = Preferences {
        show_output: true,
        ..Preferences::default()
    };
    let patched = patch_preferences(source, &preferences);

    assert!(patched.contains("// \"show_output\" : à revoir"), "{patched}");
    assert!(patched.contains("\"show_output\": true"), "{patched}");
    let reread: Preferences =
        serde_json_lenient::from_str_lenient(&patched).expect("le fichier reste lisible");
    assert!(reread.show_output);
}

#[test]
fn an_odd_quote_in_a_comment_does_not_eat_the_closing_brace() {
    // Un guillemet seul dans un commentaire laissait le balayage « dans une
    // chaîne » jusqu'à la fin du fichier, accolade finale comprise.
    let source = "{\n  \"show_output\": false\n  // 5\" de large\n}\n";
    let preferences = Preferences {
        show_output: true,
        ..Preferences::default()
    };
    let patched = patch_preferences(source, &preferences);

    assert!(patched.trim_end().ends_with('}'), "{patched}");
    let reread: Preferences =
        serde_json_lenient::from_str_lenient(&patched).expect("le fichier reste lisible");
    assert!(reread.show_output);
}

#[test]
fn a_comment_holding_a_brace_does_not_derail_the_patch() {
    let source = "{\n  \"show_output\": false\n  /* un } et une \" ici */\n}\n";
    let preferences = Preferences {
        show_output: true,
        ..Preferences::default()
    };
    let patched = patch_preferences(source, &preferences);

    assert!(patched.contains("/* un } et une \" ici */"), "{patched}");
    assert!(patched.contains("\"show_output\": true"), "{patched}");
}

#[test]
fn a_key_added_next_to_a_trailing_comment_stays_valid_json() {
    // La virgule ajoutée en fin d'objet atterrissait dans le commentaire, donc
    // commentée : deux membres sans séparateur.
    let source = "{\n  \"show_project_panel\": true\n  // TODO : ajouter show_output\n}\n";
    let patched = patch_preferences(source, &Preferences::default());

    assert!(patched.contains("// TODO : ajouter show_output"), "{patched}");
    let reread: Preferences =
        serde_json_lenient::from_str_lenient(&patched).expect("le fichier reste lisible");
    assert_eq!(reread, Preferences::default());
}

#[test]
fn splicing_stops_at_the_end_of_the_value_it_replaces() {
    let source = "{\n  \"a\": [1, {\"b\": 2}],\n  \"c\": 3\n}";
    let patched = splice_key(source, "a", "[]").expect("la clé est là");
    assert_eq!(patched, "{\n  \"a\": [],\n  \"c\": 3\n}");

    assert!(splice_key(source, "absente", "1").is_none());
}

#[test]
fn appending_a_key_to_an_empty_object_stays_valid() {
    assert_eq!(append_key("{}", "a", "1"), "{\n  \"a\": 1}");

    // Ajoutée en tête, pas en queue : c'est la seule position qu'aucun
    // commentaire de fin d'objet ne peut gâter.
    let patched = append_key("{\n  \"a\": 1\n}", "b", "2");
    let value: serde_json_lenient::Value =
        serde_json_lenient::from_str_lenient(&patched).expect("{patched}");
    assert_eq!(value["a"], 1);
    assert_eq!(value["b"], 2);
}

#[test]
fn the_documented_defaults_are_readable_and_hold_the_defaults() {
    let source = maxx::settings::documented_defaults();
    assert!(source.contains("// Réglages de maxx."), "{source}");

    let preferences: Preferences =
        serde_json_lenient::from_str_lenient(&source).expect("les commentaires sont tolérés");
    assert_eq!(preferences, Preferences::default());
}

#[test]
fn a_damaged_file_falls_back_to_the_defaults() {
    let path = std::env::temp_dir().join("maxx_settings_damaged.json");
    std::fs::write(&path, "ceci n'est pas du JSON = = =\n").unwrap();

    let preferences: Preferences = maxx::settings::read_json(&path);
    assert_eq!(preferences, Preferences::default());
    // Le fichier abîmé reste sur le disque : l'écraser perdrait ce que
    // l'utilisateur était en train d'y écrire.
    assert!(path.exists());
}

#[test]
fn a_partial_file_keeps_the_defaults_for_what_it_omits() {
    let path = std::env::temp_dir().join("maxx_settings_partial.json");
    std::fs::write(&path, "{ \"show_output\": true }\n").unwrap();

    let preferences: Preferences = maxx::settings::read_json(&path);
    assert!(preferences.show_output);
    assert!(preferences.show_project_panel, "défaut perdu");
    assert!(preferences.show_status_bar, "défaut perdu");
}

#[test]
fn the_recent_list_moves_deduplicates_and_stops_at_ten() {
    let mut state = State::default();

    assert!(state.remember_project(&PathBuf::from("/tmp/un")));
    assert!(state.remember_project(&PathBuf::from("/tmp/deux")));
    assert_eq!(
        state.recent_projects,
        vec![PathBuf::from("/tmp/deux"), PathBuf::from("/tmp/un")]
    );

    // Rouvrir celui qui est déjà en tête ne change rien — donc ni fichier
    // réécrit, ni barre de menus reconstruite.
    assert!(!state.remember_project(&PathBuf::from("/tmp/deux")));

    // Rouvrir un ancien le remonte, sans le dupliquer.
    assert!(state.remember_project(&PathBuf::from("/tmp/un")));
    assert_eq!(
        state.recent_projects,
        vec![PathBuf::from("/tmp/un"), PathBuf::from("/tmp/deux")]
    );

    for index in 0..15 {
        state.remember_project(&PathBuf::from(format!("/tmp/projet_{index}")));
    }
    assert_eq!(state.recent_projects.len(), 10);
    assert_eq!(
        state.recent_projects[0],
        PathBuf::from("/tmp/projet_14"),
        "le plus récent doit être en tête"
    );
}

#[test]
fn a_project_that_no_longer_exists_leaves_the_list() {
    let root = std::env::temp_dir().join("maxx_settings_missing");
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();

    let mut state = State {
        recent_projects: vec![root.clone(), root.join("parti")],
        ..State::default()
    };
    state.forget_missing_projects();

    assert_eq!(state.recent_projects, vec![root]);
}

#[test]
fn a_hand_written_file_with_every_trap_at_once_survives() {
    // Les trois pièges réunis : une clé et un deux-points dans un commentaire,
    // un guillemet impair dans ce même commentaire, et un commentaire de fin de
    // ligne juste après une valeur.
    let source = r#"// Mon fichier à moi.
{
  "$schema": "./settings-schema.json",

  // "show_output" : à revoir un jour — 5" de large
  "show_project_panel": true, // l'explorateur
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

    assert!(patched.contains("// Mon fichier à moi."), "{patched}");
    assert!(patched.contains(r#"// "show_output" : à revoir un jour — 5" de large"#), "{patched}");
    assert!(patched.contains("// l'explorateur"), "{patched}");
    assert!(patched.contains(r#""$schema": "./settings-schema.json""#), "{patched}");

    let reread: Preferences =
        serde_json_lenient::from_str_lenient(&patched).expect("le fichier reste lisible : {patched}");
    assert_eq!(reread, preferences);
}

#[test]
fn a_brace_in_the_header_comment_does_not_anchor_the_walk() {
    // Un utilisateur qui écrit « // éditeur : {code, zed} » au-dessus de
    // l'accolade ouvrante faisait ancrer tout le parcours dans le commentaire.
    let source = "// éditeur : {code, zed}\n{\n  \"show_output\": false\n}\n";
    let preferences = Preferences {
        show_output: true,
        ..Preferences::default()
    };
    let patched = patch_preferences(source, &preferences);

    assert!(patched.starts_with("// éditeur : {code, zed}\n{"), "{patched}");
    let reread: Preferences =
        serde_json_lenient::from_str_lenient(&patched).expect("le fichier reste lisible : {patched}");
    assert!(reread.show_output);
}
