//! La démo du dépôt est la référence : ce que maxx doit savoir relire, et ce
//! qu'il doit savoir réécrire sans rien abîmer.
//!
//! Elle vit dans `demo/`, versionnée, à un chemin relatif au dépôt — l'ancienne
//! référence était un chemin absolu vers un dossier personnel, et le test
//! s'arrêtait sans échouer quand il manquait : chez quelqu'un d'autre, la
//! couverture était nulle et silencieuse.

use std::path::{Path, PathBuf};

use maxx::menufile::MenuFile;
use maxx::view::View;

fn demo() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("demo")
}

#[test]
fn every_demo_view_reads_back() {
    let ui = demo().join("src/ui");
    let mut seen = 0;

    for entry in std::fs::read_dir(&ui).expect("la démo doit avoir des vues") {
        let path = entry.unwrap().path();
        if path.file_name().is_some_and(|name| name == "mod.rs") {
            continue;
        }
        let view = View::load(&path).unwrap_or_else(|error| {
            panic!("{} ne se relit pas : {error}", path.display())
        });
        assert!(
            !view.root.children.is_empty(),
            "{} : arbre vide, la région gérée est mal repérée",
            path.display()
        );
        seen += 1;
    }

    assert!(seen >= 2, "la démo doit garder au moins deux vues");
}

#[test]
fn rewriting_a_demo_view_changes_nothing() {
    // La propriété qui compte, et sa formulation exacte : relire puis réécrire
    // sans rien avoir modifié doit rendre le fichier à l'octet près — *à
    // rustfmt près*.
    //
    // La nuance n'est pas un aveu, c'est la description du système. `codegen`
    // n'écrit pas ce que rustfmt écrirait, et un éditeur Rust formate à
    // l'enregistrement ; maxx passe donc rustfmt après lui, et c'est la
    // composition des deux qui doit être stable. Un fichier de démo mis en
    // forme est aussi ce qu'un projet réel serait.
    let path = demo().join("src/ui/accueil.rs");
    let before = std::fs::read_to_string(&path).unwrap();

    let view = View::load(&path).expect("la vue doit se relire");
    let spliced = maxx::parser::splice(&before, &maxx::codegen::render(&view.root, 0))
        .expect("la région gérée doit se retrouver");

    let temporaire = std::env::temp_dir().join("maxx_demo_aller_retour.rs");
    std::fs::write(&temporaire, &spliced).unwrap();
    match maxx::run::format_rust(&temporaire) {
        Ok(_) => {
            let after = std::fs::read_to_string(&temporaire).unwrap();
            assert_eq!(before, after, "la réécriture suivie de rustfmt n'est pas neutre");
        }
        Err(erreur) => assert!(erreur.contains("introuvable"), "{erreur}"),
    }
}

#[test]
fn the_demo_uses_the_components_it_advertises() {
    let path = demo().join("src/ui/accueil.rs");
    let view = View::load(&path).expect("la vue doit se relire");

    let mut bases = Vec::new();
    collect(&view.root, &mut bases);

    for expected in [
        "v_flex",
        "h_flex",
        "Label::new",
        "Input::new",
        "Button::new",
        "Checkbox::new",
        "Switch::new",
        "GroupBox::new",
        "Divider::horizontal",
    ] {
        assert!(
            bases.iter().any(|base| base == expected),
            "{expected} a disparu de la démo : {bases:?}"
        );
    }
}

#[test]
fn the_demo_input_is_bound_to_a_field() {
    let path = demo().join("src/ui/accueil.rs");
    let view = View::load(&path).expect("la vue doit se relire");

    let fields = view.state_fields();
    assert!(
        fields.iter().any(|field| field.name == "nom"),
        "le champ texte de la démo doit être lié à un champ de la vue"
    );
    assert!(view.method_line("on_ouvrir").is_some(), "le gestionnaire du bouton doit exister");
}

#[test]
fn the_demo_menu_bar_reads_back() {
    let path = demo().join("src/menus.rs");
    let menus = MenuFile::load(&path).expect("la barre de menus doit se relire");

    assert_eq!(menus.menus.len(), 3, "app, Édition, Fenêtre");

    let fenetre = menus
        .menus
        .iter()
        .find(|menu| menu.name == "Fenêtre")
        .expect("le menu Fenêtre doit être là");
    assert!(
        fenetre.items.iter().any(|item| item.label() == "Ouvrir l'inspecteur"),
        "l'entrée qui ouvre une fenêtre est ce que la démo existe pour montrer"
    );
    assert!(
        menus.handler_line("OuvrirInspecteur").is_some(),
        "son action doit avoir un gestionnaire"
    );
}

/// Ramasse la base de chaque nœud de l'arbre.
fn collect(node: &maxx::model::Node, out: &mut Vec<String>) {
    if let Some(path) = node.base.path() {
        out.push(path.to_string());
    }
    for child in &node.children {
        collect(child, out);
    }
}
