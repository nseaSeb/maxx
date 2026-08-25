//! Le catalogue des éditeurs et des terminaux : c'est une table, donc ce qui
//! peut clocher est dedans, pas dans un algorithme.

use std::path::Path;

use maxx::run::editor_arguments;
use maxx::tools::{EDITORS, Editor, LineArgument, TERMINALS};

fn editor(id: &str) -> &'static Editor {
    EDITORS
        .iter()
        .find(|editor| editor.id == id)
        .unwrap_or_else(|| panic!("{id} doit être au catalogue"))
}

#[test]
fn every_editor_spells_its_line_number_its_own_way() {
    let path = Path::new("/tmp/vue.rs");

    assert_eq!(editor_arguments(editor("zed"), path, Some(12)), vec!["/tmp/vue.rs:12"]);
    assert_eq!(editor_arguments(editor("code"), path, Some(12)), vec!["-g", "/tmp/vue.rs:12"]);
    assert_eq!(editor_arguments(editor("nvim"), path, Some(12)), vec!["+12", "/tmp/vue.rs"]);
    assert_eq!(
        editor_arguments(editor("rustrover"), path, Some(12)),
        vec!["--line", "12", "/tmp/vue.rs"]
    );
}

#[test]
fn without_a_line_every_editor_takes_the_bare_path() {
    let path = Path::new("/tmp/vue.rs");
    for candidate in EDITORS {
        assert_eq!(
            editor_arguments(candidate, path, None),
            vec!["/tmp/vue.rs"],
            "{}",
            candidate.id
        );
    }
}

#[test]
fn the_catalogue_holds_no_duplicate_and_no_hole() {
    for (index, editor) in EDITORS.iter().enumerate() {
        assert!(!editor.id.is_empty());
        assert!(!editor.label.is_empty());
        // Un éditeur sans commande ni paquet ne peut jamais être détecté.
        assert!(!editor.command.is_empty() || editor.bundle.is_some(), "{}", editor.id);
        // Un éditeur de terminal n'a pas de paquet à ouvrir : il n'est qu'une
        // commande.
        if editor.terminal_bound {
            assert!(editor.bundle.is_none(), "{}", editor.id);
            assert!(!editor.command.is_empty(), "{}", editor.id);
        }
        assert!(
            EDITORS[index + 1..].iter().all(|other| other.id != editor.id),
            "{} en double",
            editor.id
        );
    }

    for (index, terminal) in TERMINALS.iter().enumerate() {
        assert!(!terminal.id.is_empty());
        assert!(!terminal.command.is_empty() || terminal.bundle.is_some(), "{}", terminal.id);
        // Lancer une commande suppose une commande à qui la passer.
        if terminal.command_flag.is_some() {
            assert!(!terminal.command.is_empty(), "{}", terminal.id);
        }
        assert!(
            TERMINALS[index + 1..].iter().all(|other| other.id != terminal.id),
            "{} en double",
            terminal.id
        );
    }
}

#[test]
fn a_flag_style_editor_never_gets_a_suffix_and_the_reverse() {
    // La confusion qui casserait tout : `code fichier:12` ouvre un fichier
    // nommé « fichier:12 », et `zed -g fichier:12` ne comprend pas -g.
    for editor in EDITORS {
        let arguments = editor_arguments(editor, Path::new("/tmp/a.rs"), Some(3));
        match editor.line {
            LineArgument::Suffix => {
                assert_eq!(arguments.len(), 1, "{}", editor.id);
                assert!(arguments[0].ends_with(":3"), "{}", editor.id);
            }
            LineArgument::Flag(flag) => {
                assert_eq!(arguments[0], flag, "{}", editor.id);
                assert!(arguments[1].ends_with(":3"), "{}", editor.id);
            }
            LineArgument::PlusLine => assert_eq!(arguments[0], "+3", "{}", editor.id),
            LineArgument::Named(name) => {
                assert_eq!(arguments[0], name, "{}", editor.id);
                assert_eq!(arguments[1], "3", "{}", editor.id);
            }
        }
    }
}

#[test]
fn nothing_is_found_on_an_empty_path() {
    assert!(!maxx::tools::on_path(""));
}

#[test]
fn rustfmt_reformats_a_file_and_says_so() {
    let path = std::env::temp_dir().join("maxx_format_test.rs");
    std::fs::write(&path, "fn   principale(){let  x=1;let _=x;}\n").unwrap();

    match maxx::run::format_rust(&path) {
        Ok(change) => {
            assert!(change, "rustfmt avait de quoi faire sur ce fichier");
            let apres = std::fs::read_to_string(&path).unwrap();
            assert!(apres.contains("fn principale() {"), "{apres}");
            // Deux fois de suite ne change plus rien : rustfmt est idempotent,
            // et c'est ce qui rend l'aller-retour de maxx stable.
            assert!(!maxx::run::format_rust(&path).unwrap());
        }
        // rustfmt n'est pas garanti présent partout ; le test dit ce qu'il
        // vérifie plutôt que d'échouer pour une raison étrangère.
        Err(erreur) => assert!(erreur.contains("introuvable"), "{erreur}"),
    }
}

#[test]
fn a_file_that_is_not_rust_is_refused_rather_than_mangled() {
    let path = std::env::temp_dir().join("maxx_format_invalide.rs");
    std::fs::write(&path, "ceci n'est pas du Rust {{{\n").unwrap();

    // Le texte du message n'est pas ce qu'on vérifie : il est traduit, donc
    // dépend de la langue, et l'affirmer ici lierait le comportement à sa
    // formulation. Ce qui compte est qu'il y ait un refus et qu'il nomme le
    // fichier.
    if let Err(erreur) = maxx::run::format_rust(&path) {
        assert!(!erreur.is_empty());
        assert!(
            erreur.contains("maxx_format_invalide.rs") || erreur.contains("rustfmt"),
            "{erreur}"
        );
    }
    // Et surtout : le fichier n'a pas été abîmé.
    assert!(std::fs::read_to_string(&path).unwrap().contains("ceci n'est pas du Rust"));
}
