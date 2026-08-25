//! Les réglages doivent survivre à un aller-retour sur disque, et surtout à un
//! fichier absent, vide ou abîmé : maxx doit démarrer dans tous les cas.

use std::path::PathBuf;

use maxx::settings::Settings;

/// Un fichier de réglages à nous.
///
/// `load_from` et `save_to` prennent un chemin, donc rien ici ne touche aux
/// réglages de la machine — et les tests peuvent tourner en parallèle, ce
/// qu'une variable d'environnement partagée interdisait.
fn scratch(name: &str) -> PathBuf {
    let root = std::env::temp_dir().join(name);
    let _ = std::fs::remove_dir_all(&root);
    root.join("settings.toml")
}

#[test]
fn settings_survive_a_round_trip() {
    let path = scratch("maxx_settings_round_trip");

    let settings = Settings {
        show_project_panel: false,
        show_output: true,
        recent_projects: vec![PathBuf::from("/tmp/a"), PathBuf::from("/tmp/b")],
        window: Some(maxx::settings::WindowGeometry {
            x: 12.0,
            y: 34.0,
            width: 800.0,
            height: 600.0,
        }),
        ..Settings::default()
    };
    settings.save_to(&path).expect("les réglages doivent s'écrire");

    assert!(path.exists(), "{} n'existe pas", path.display());
    let source = std::fs::read_to_string(&path).unwrap();
    assert!(source.starts_with("# Réglages de maxx."), "{source}");

    assert_eq!(Settings::load_from(&path), settings);
}

#[test]
fn a_damaged_file_falls_back_to_the_defaults() {
    let path = scratch("maxx_settings_damaged");
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(&path, "ceci n'est pas du TOML = = =\n").unwrap();

    assert_eq!(Settings::load_from(&path), Settings::default());
    // Le fichier abîmé reste sur le disque : l'écraser perdrait ce que
    // l'utilisateur était en train d'y écrire.
    assert!(path.exists());
}

#[test]
fn a_partial_file_keeps_the_defaults_for_what_it_omits() {
    let path = scratch("maxx_settings_partial");
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(&path, "show_output = true\n").unwrap();

    let settings = Settings::load_from(&path);
    assert!(settings.show_output);
    assert!(settings.show_project_panel, "défaut perdu");
    assert!(settings.show_status_bar, "défaut perdu");
    assert!(settings.recent_projects.is_empty());
}

#[test]
fn the_recent_list_moves_deduplicates_and_stops_at_ten() {
    let mut settings = Settings::default();

    assert!(settings.remember_project(&PathBuf::from("/tmp/un")));
    assert!(settings.remember_project(&PathBuf::from("/tmp/deux")));
    assert_eq!(
        settings.recent_projects,
        vec![PathBuf::from("/tmp/deux"), PathBuf::from("/tmp/un")]
    );

    // Rouvrir celui qui est déjà en tête ne change rien — donc ni fichier
    // réécrit, ni barre de menus reconstruite.
    assert!(!settings.remember_project(&PathBuf::from("/tmp/deux")));

    // Rouvrir un ancien le remonte, sans le dupliquer.
    assert!(settings.remember_project(&PathBuf::from("/tmp/un")));
    assert_eq!(
        settings.recent_projects,
        vec![PathBuf::from("/tmp/un"), PathBuf::from("/tmp/deux")]
    );

    for index in 0..15 {
        settings.remember_project(&PathBuf::from(format!("/tmp/projet_{index}")));
    }
    assert_eq!(settings.recent_projects.len(), 10);
    assert_eq!(
        settings.recent_projects[0],
        PathBuf::from("/tmp/projet_14"),
        "le plus récent doit être en tête"
    );
}

#[test]
fn a_project_that_no_longer_exists_leaves_the_list() {
    let root = std::env::temp_dir().join("maxx_settings_missing");
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();

    let mut settings = Settings {
        recent_projects: vec![root.clone(), root.join("parti")],
        ..Settings::default()
    };
    settings.forget_missing_projects();

    assert_eq!(settings.recent_projects, vec![root]);
}
