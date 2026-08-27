//! The modules maxx copies into a project are copies, therefore a debt: a defect
//! fixed here has to be able to reach the projects carrying the older version.
//! `maxx.toml` is what makes that possible.

use std::path::PathBuf;

use maxx::projectfile::{self, fingerprint};
use maxx::scaffold::{self, Template};

fn scratch(name: &str) -> PathBuf {
    let dir =
        std::env::var("MAXX_SCRATCH").map(PathBuf::from).unwrap_or_else(|_| std::env::temp_dir());
    let path = dir.join(name);
    let _ = std::fs::remove_dir_all(&path);
    path
}

/// The fingerprint of each template, as it stands at its current version.
///
/// This table is a guard rail, not data: changing a template makes this test
/// fail, which forces a decision about whether the version should go up. Without
/// it, a fix would never reach the projects already written — and nobody would
/// notice.
const FINGERPRINTS: &[(&str, u32, &str)] = &[
    ("system", 1, "c2efcda0672f77c9"),
    ("settings", 1, "f3e4f7d28ee2ba66"),
    ("theme", 1, "d4768642faff2027"),
    ("assets", 1, "90cb5efc780a59f8"),
    ("window", 1, "73935d644407d44a"),
];

#[test]
fn changing_a_template_forces_a_decision_about_its_version() {
    for (module, version, print) in FINGERPRINTS {
        let body = scaffold::module_body(module).expect("the template must exist");
        assert_eq!(
            scaffold::module_version(module),
            Some(*version),
            "{module}: the version changed without this table following"
        );
        assert_eq!(
            fingerprint(&body),
            *print,
            "{module}: the template changed. If the fix has to reach the projects \
             carrying a copy of it, raise its version in scaffold::MODULES, then \
             report the fingerprint here."
        );
    }
    assert_eq!(
        FINGERPRINTS.len(),
        scaffold::MODULES.len(),
        "a module was added without its fingerprint"
    );
}

#[test]
fn adding_a_module_records_what_the_project_took() {
    let root = scratch("maxx_modules_record");
    scaffold::create_project(&root, "trial", Template::Empty).expect("the project must be created");
    scaffold::add_system_module(&root).expect("the module must be added");

    let file = projectfile::load(&root);
    let recorded = file.modules.get("system").expect("maxx.toml must note it");
    assert_eq!(recorded.version, scaffold::module_version("system").unwrap());

    let body = std::fs::read_to_string(root.join("src/system.rs")).unwrap();
    assert_eq!(recorded.fingerprint, fingerprint(&body));

    // And the file stays readable by hand.
    let source = std::fs::read_to_string(projectfile::path(&root)).unwrap();
    assert!(source.starts_with("# Written by maxx"), "{source}");
}

#[test]
fn an_old_copy_is_offered_an_update_and_a_touched_one_is_not() {
    let root = scratch("maxx_modules_update");
    scaffold::create_project(&root, "trial", Template::Empty).expect("the project must be created");
    scaffold::add_system_module(&root).expect("the module must be added");

    // Nothing to do as long as the project is up to date.
    assert!(scaffold::outdated_modules(&root).is_empty());

    // A project written by an older maxx: version behind, file matching what
    // that version used to write.
    let path = root.join("src/system.rs");
    let older = "// an older version\n";
    std::fs::write(&path, older).unwrap();
    projectfile::record(&root, "system", 0, older).unwrap();

    assert_eq!(scaffold::outdated_modules(&root), vec!["system".to_string()]);
    scaffold::update_module(&root, "system").expect("the update must go through");

    let body = std::fs::read_to_string(&path).unwrap();
    assert_eq!(body, scaffold::module_body("system").unwrap());
    assert!(scaffold::outdated_modules(&root).is_empty());
    // The recorded fingerprint follows the new content.
    assert_eq!(projectfile::load(&root).modules["system"].fingerprint, fingerprint(&body));
}

#[test]
fn a_module_the_developer_edited_is_left_alone() {
    let root = scratch("maxx_modules_touched");
    scaffold::create_project(&root, "trial", Template::Empty).expect("the project must be created");
    scaffold::add_system_module(&root).expect("the module must be added");

    let path = root.join("src/system.rs");
    let older = "// an older version\n";
    std::fs::write(&path, older).unwrap();
    projectfile::record(&root, "system", 0, older).unwrap();

    // The developer touches it.
    let edited = format!("{older}// and a line of my own\n");
    std::fs::write(&path, &edited).unwrap();

    // It is no longer offered…
    assert!(scaffold::outdated_modules(&root).is_empty(), "an edited file is no longer maxx's");
    // …and forcing the update is refused, without overwriting anything.
    let error = scaffold::update_module(&root, "system").expect_err("must be refused");
    assert!(error.to_string().contains("has been modified"), "{error}");
    assert_eq!(std::fs::read_to_string(&path).unwrap(), edited);
}

#[test]
fn line_endings_alone_do_not_count_as_an_edit() {
    // A file passed through a tool that converts line endings has not been
    // modified for all that: refusing it an update would be a false positive.
    assert_eq!(fingerprint("a\nb\n"), fingerprint("a\r\nb\r\n"));
    assert_ne!(fingerprint("a\nb\n"), fingerprint("a\nb\nc\n"));
}

/// A module already there under its old name is not copied again under the new
/// one.
///
/// `systeme.rs` and `system.rs` are the same module at two points in time.
/// Writing both would leave the project compiling with two almost identical
/// copies, and nobody to say which one its code calls.
#[test]
fn a_module_already_there_under_its_old_name_is_not_copied_again() {
    let root = scratch("maxx_old_name");
    scaffold::create_project(&root, "trial", Template::Empty).unwrap();
    std::fs::write(root.join("src/systeme.rs"), "// my earlier copy\n").unwrap();

    let error = scaffold::add_system_module(&root).expect_err("must be refused");
    assert!(error.to_string().contains("src/systeme.rs"), "{error}");
    assert!(!root.join("src/system.rs").exists(), "and nothing is written beside it");

    std::fs::write(root.join("src/reglages.rs"), "// and this one too\n").unwrap();
    let error = scaffold::add_settings_module(&root).expect_err("must be refused");
    assert!(error.to_string().contains("src/reglages.rs"), "{error}");
    assert!(!root.join("src/settings.rs").exists());
}

/// A `maxx.toml` written before the move to English still reads.
///
/// The key was called `empreinte`. Without the alias, `Module` fails to
/// deserialise, `load` answers an empty file, and the first `record` writes that
/// emptiness over it — the other modules lose their version and their
/// fingerprint.
#[test]
fn a_project_file_written_before_the_rename_still_reads() {
    let root = scratch("maxx_old_toml");
    std::fs::create_dir_all(&root).unwrap();
    std::fs::write(
        projectfile::path(&root),
        "[modules.systeme]\nversion = 1\nempreinte = \"9f760f0126a35c23\"\n",
    )
    .unwrap();

    let file = projectfile::load(&root);
    let recorded = file.modules.get("systeme").expect("the module must be read back");
    assert_eq!(recorded.version, 1);
    assert_eq!(recorded.fingerprint, "9f760f0126a35c23");
}

/// A project formatted by its own developer is still a project maxx knows.
///
/// The defect this guards: maxx recognised a module it had written by the bytes
/// it left, and `cargo fmt` — the most ordinary gesture there is — changes those
/// bytes without touching a line of code. Measured on the templates, the
/// default layout moves ten lines of `system.rs` and fifty-six of `theme.rs`.
/// maxx then took the file for one the developer had edited and, silently,
/// stopped offering the fixes it had for it.
#[test]
fn a_formatted_module_is_still_recognised() {
    let root = scratch("maxx_modules_formatted");
    scaffold::create_project(&root, "trial", Template::Empty).unwrap();
    scaffold::add_system_module(&root).expect("the module must be copied");

    let path = root.join("src/system.rs");
    let written = std::fs::read_to_string(&path).unwrap();
    let recorded = projectfile::load(&root).modules.get("system").cloned().expect("recorded");
    assert!(recorded.holds(&written), "the file maxx just wrote");

    // What `cargo fmt` does to it, in the layout every project gets by default.
    let formatted = maxx::run::formatted_default(&written).expect("rustfmt must run");
    std::fs::write(&path, &formatted).unwrap();
    assert_ne!(formatted, written, "this template is one the default layout moves");
    assert!(recorded.holds(&formatted), "a formatted copy is not an edited copy");

    // And what an edit does: a function of the developer's own, which maxx must
    // never write over.
    let edited = format!("{formatted}\npub fn mine() {{}}\n");
    assert!(!recorded.holds(&edited), "an edited copy must stay the developer's");
}

/// A comment added by the developer counts as an edit.
///
/// `rustfmt` keeps comments, so the shape carries them too — which is what
/// stops maxx from replacing a file someone annotated.
#[test]
fn a_comment_added_to_a_module_makes_it_the_developers() {
    let root = scratch("maxx_modules_annotated");
    scaffold::create_project(&root, "trial", Template::Empty).unwrap();
    scaffold::add_system_module(&root).unwrap();

    let recorded = projectfile::load(&root).modules.get("system").cloned().expect("recorded");
    let written = std::fs::read_to_string(root.join("src/system.rs")).unwrap();
    let annotated = format!("// une note du développeur\n{written}");
    assert!(!recorded.holds(&annotated));
}

/// A file written before the shape existed still answers.
#[test]
fn a_record_without_a_shape_falls_back_to_the_bytes() {
    let body = scaffold::module_body("system").expect("template");
    let old = projectfile::Module { version: 1, fingerprint: fingerprint(&body), shape: None };

    assert!(old.holds(&body), "the bytes still answer");
    // And without a shape there is nothing to fall back on: a formatted copy
    // reads as an edited one, which is what maxx did before.
    let formatted = maxx::run::formatted_default(&body).expect("rustfmt must run");
    assert!(!old.holds(&formatted));
}
