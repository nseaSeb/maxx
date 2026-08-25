//! Les modules que maxx copie dans un projet sont des copies, donc une dette :
//! un défaut corrigé ici doit pouvoir atteindre les projets qui portent
//! l'ancienne version. `maxx.toml` est ce qui rend ça possible.

use std::path::PathBuf;

use maxx::projectfile::{self, fingerprint};
use maxx::scaffold;

fn scratch(name: &str) -> PathBuf {
    let dir = std::env::var("MAXX_SCRATCH")
        .map(PathBuf::from)
        .unwrap_or_else(|_| std::env::temp_dir());
    let path = dir.join(name);
    let _ = std::fs::remove_dir_all(&path);
    path
}

/// L'empreinte de chaque gabarit, telle qu'elle est à sa version courante.
///
/// Ce tableau est un garde-fou, pas une donnée : modifier un gabarit fait
/// échouer ce test, ce qui oblige à décider si la version doit monter. Sans
/// lui, une correction n'atteindrait jamais les projets déjà écrits — et
/// personne ne s'en apercevrait.
const EMPREINTES: &[(&str, u32, &str)] = &[
    ("systeme", 1, "9f760f0126a35c23"),
    ("reglages", 1, "b28297b08580657f"),
];

#[test]
fn changing_a_template_forces_a_decision_about_its_version() {
    for (module, version, empreinte) in EMPREINTES {
        let body = scaffold::module_body(module).expect("le gabarit doit exister");
        assert_eq!(
            scaffold::module_version(module),
            Some(*version),
            "{module} : la version a changé sans que ce tableau suive"
        );
        assert_eq!(
            fingerprint(&body),
            *empreinte,
            "{module} : le gabarit a changé. Si la correction doit atteindre les \
             projets qui en portent une copie, montez sa version dans \
             scaffold::MODULES, puis reportez l'empreinte ici."
        );
    }
    assert_eq!(
        EMPREINTES.len(),
        scaffold::MODULES.len(),
        "un module a été ajouté sans son empreinte"
    );
}

#[test]
fn adding_a_module_records_what_the_project_took() {
    let root = scratch("maxx_modules_record");
    scaffold::create_project(&root, "essai").expect("le projet doit être créé");
    scaffold::add_system_module(&root).expect("le module doit être ajouté");

    let file = projectfile::load(&root);
    let recorded = file.modules.get("systeme").expect("maxx.toml doit le noter");
    assert_eq!(recorded.version, scaffold::module_version("systeme").unwrap());

    let body = std::fs::read_to_string(root.join("src/systeme.rs")).unwrap();
    assert_eq!(recorded.empreinte, fingerprint(&body));

    // Et le fichier reste lisible à la main.
    let source = std::fs::read_to_string(projectfile::path(&root)).unwrap();
    assert!(source.starts_with("# Écrit par maxx"), "{source}");
}

#[test]
fn an_old_copy_is_offered_an_update_and_a_touched_one_is_not() {
    let root = scratch("maxx_modules_update");
    scaffold::create_project(&root, "essai").expect("le projet doit être créé");
    scaffold::add_system_module(&root).expect("le module doit être ajouté");

    // Rien à faire tant que le projet est à jour.
    assert!(scaffold::outdated_modules(&root).is_empty());

    // Un projet écrit par un maxx plus ancien : version en retard, fichier
    // conforme à ce que cette version-là écrivait.
    let path = root.join("src/systeme.rs");
    let ancien = "// une version plus ancienne\n";
    std::fs::write(&path, ancien).unwrap();
    projectfile::record(&root, "systeme", 0, ancien).unwrap();

    assert_eq!(scaffold::outdated_modules(&root), vec!["systeme".to_string()]);
    scaffold::update_module(&root, "systeme").expect("la mise à jour doit passer");

    let body = std::fs::read_to_string(&path).unwrap();
    assert_eq!(body, scaffold::module_body("systeme").unwrap());
    assert!(scaffold::outdated_modules(&root).is_empty());
    // L'empreinte notée suit le nouveau contenu.
    assert_eq!(
        projectfile::load(&root).modules["systeme"].empreinte,
        fingerprint(&body)
    );
}

#[test]
fn a_module_the_developer_edited_is_left_alone() {
    let root = scratch("maxx_modules_touched");
    scaffold::create_project(&root, "essai").expect("le projet doit être créé");
    scaffold::add_system_module(&root).expect("le module doit être ajouté");

    let path = root.join("src/systeme.rs");
    let ancien = "// une version plus ancienne\n";
    std::fs::write(&path, ancien).unwrap();
    projectfile::record(&root, "systeme", 0, ancien).unwrap();

    // Le développeur y touche.
    let modifie = format!("{ancien}// et ma ligne à moi\n");
    std::fs::write(&path, &modifie).unwrap();

    // Il n'est plus proposé…
    assert!(
        scaffold::outdated_modules(&root).is_empty(),
        "un fichier modifié n'est plus à maxx"
    );
    // …et forcer la mise à jour est refusé, sans rien écraser.
    let erreur = scaffold::update_module(&root, "systeme").expect_err("doit être refusé");
    assert!(erreur.to_string().contains("modifié"), "{erreur}");
    assert_eq!(std::fs::read_to_string(&path).unwrap(), modifie);
}

#[test]
fn line_endings_alone_do_not_count_as_an_edit() {
    // Un fichier passé par un outil qui convertit les fins de ligne n'a pas été
    // modifié pour autant : le refuser à la mise à jour serait un faux positif.
    assert_eq!(fingerprint("a\nb\n"), fingerprint("a\r\nb\r\n"));
    assert_ne!(fingerprint("a\nb\n"), fingerprint("a\nb\nc\n"));
}
