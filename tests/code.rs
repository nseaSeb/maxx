//! Le lecteur de code : ce qui peut clocher est dans la table des extensions
//! et dans les deux refus, pas dans le rendu.

use std::path::Path;

use maxx::workspace::{CodeFile, language_for};

#[test]
fn chaque_extension_nomme_sa_grammaire() {
    assert_eq!(language_for(Path::new("/p/src/main.rs")), "rust");
    assert_eq!(language_for(Path::new("/p/Cargo.toml")), "toml");
    assert_eq!(language_for(Path::new("/p/Cargo.lock")), "toml");
    assert_eq!(language_for(Path::new("/p/README.md")), "markdown");
    assert_eq!(language_for(Path::new("/p/settings.json")), "json");
    assert_eq!(language_for(Path::new("/p/locales/app.yml")), "yaml");
    // La casse de l'extension ne dit rien du contenu.
    assert_eq!(language_for(Path::new("/p/LISEZMOI.MD")), "markdown");
}

#[test]
fn ce_qui_na_pas_dextension_connue_reste_du_texte() {
    // Pas de grammaire approchante : coloriser un LICENCE en Markdown lui
    // inventerait une structure qu'il n'a pas.
    assert_eq!(language_for(Path::new("/p/LICENSE")), "text");
    assert_eq!(language_for(Path::new("/p/.gitignore")), "text");
    assert_eq!(language_for(Path::new("/p/notes.txt")), "text");
}

#[test]
fn un_fichier_du_depot_se_lit() {
    let Ok(file) = CodeFile::load(Path::new("Cargo.toml")) else {
        panic!("Cargo.toml doit se lire");
    };
    assert_eq!(file.language, "toml");
    assert!(file.text.contains("name = \"maxx\""));
    assert!(file.lines() > 10);
    assert_eq!(file.name().to_string(), "Cargo.toml");
}

#[test]
fn un_fichier_binaire_est_refuse() {
    // La capture d'écran du README : le refus doit venir du décodage UTF-8, pas
    // d'une liste d'extensions à tenir à jour.
    let Err(error) = CodeFile::load(Path::new("docs/maxx.png")) else {
        panic!("un PNG n'est pas du texte");
    };
    assert!(!error.is_empty());
}

#[test]
fn un_dossier_est_refuse_avec_sa_propre_raison() {
    // Le clic droit s'attrape aussi sur un dossier, et « ce fichier n'est pas
    // du texte » y serait une réponse fausse.
    let Err(raison) = CodeFile::load(Path::new("src")) else {
        panic!("un dossier n'a pas de code");
    };
    assert!(raison.contains("dossier") || raison.contains("folder"), "raison : {raison}");
}

#[test]
fn un_fichier_absent_est_refuse_lui_aussi() {
    assert!(CodeFile::load(Path::new("docs/rien-du-tout.rs")).is_err());
}

/// Le plus petit fichier portant une région gérée.
fn fichier_avec(expression: &str) -> String {
    format!(
        "use gpui::*;\n\n\
         impl Render for Home {{\n\
         \x20   fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {{\n\
         \x20       // maxx:begin\n\
         \x20       {expression}\n\
         \x20       // maxx:end\n\
         \x20   }}\n\
         }}\n"
    )
}

#[test]
fn le_code_montre_est_celui_que_save_ecrirait() {
    // La garantie de la bascule canvas / code : `render_source` et `save` sont
    // le même texte, parce que le second appelle le premier. Un jour où l'un
    // gagnerait une étape que l'autre n'a pas, ce test tombe.
    let dossier = std::env::temp_dir().join("maxx-test-lecteur");
    std::fs::create_dir_all(&dossier).expect("le dossier de test doit se créer");
    let chemin = dossier.join("home.rs");
    std::fs::write(&chemin, fichier_avec("v_flex().gap_2()")).expect("le fichier doit s'écrire");

    let Ok(mut vue) = maxx::view::View::load(&chemin) else {
        panic!("la vue doit se charger");
    };
    let Ok(montre) = vue.render_source() else {
        panic!("le rendu doit aboutir");
    };
    // Rien n'est écrit par le rendu : le disque n'a pas bougé.
    assert_eq!(std::fs::read_to_string(&chemin).unwrap(), fichier_avec("v_flex().gap_2()"));

    vue.save().expect("l'enregistrement doit aboutir");
    assert_eq!(std::fs::read_to_string(&chemin).unwrap(), montre);

    std::fs::remove_dir_all(&dossier).ok();
}
