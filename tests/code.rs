//! The code reader: what can go wrong is in the extension table and in the two
//! refusals, not in the rendering.

use std::path::Path;

use maxx::workspace::{CodeFile, language_for};

#[test]
fn every_extension_names_its_grammar() {
    assert_eq!(language_for(Path::new("/p/src/main.rs")), "rust");
    assert_eq!(language_for(Path::new("/p/Cargo.toml")), "toml");
    assert_eq!(language_for(Path::new("/p/Cargo.lock")), "toml");
    assert_eq!(language_for(Path::new("/p/README.md")), "markdown");
    assert_eq!(language_for(Path::new("/p/settings.json")), "json");
    assert_eq!(language_for(Path::new("/p/locales/app.yml")), "yaml");
    // The case of the extension says nothing about the content.
    assert_eq!(language_for(Path::new("/p/LISEZMOI.MD")), "markdown");
}

#[test]
fn an_unknown_extension_stays_plain_text() {
    // No approximate grammar: colourising a LICENCE as Markdown would invent a
    // structure it does not have.
    assert_eq!(language_for(Path::new("/p/LICENSE")), "text");
    assert_eq!(language_for(Path::new("/p/.gitignore")), "text");
    assert_eq!(language_for(Path::new("/p/notes.txt")), "text");
}

#[test]
fn a_file_of_the_repository_reads() {
    let Ok(file) = CodeFile::load(Path::new("Cargo.toml")) else {
        panic!("Cargo.toml must read");
    };
    assert_eq!(file.language, "toml");
    assert!(file.text.contains("name = \"maxx\""));
    assert!(file.lines() > 10);
    assert_eq!(file.name().to_string(), "Cargo.toml");
}

#[test]
fn a_binary_file_is_refused() {
    // The refusal has to come from the UTF-8 decoding, not from a list of
    // extensions to keep up to date — this file has one maxx never heard of.
    let path = std::env::temp_dir().join("maxx_reader_binary.dat");
    std::fs::write(&path, [0xff_u8, 0xfe, 0x00, 0x01]).unwrap();

    let Err(error) = CodeFile::load(&path) else {
        panic!("these bytes are not text");
    };
    assert!(!error.is_empty());
}

/// A picture opens as a picture, and not as the text it is not.
///
/// The UTF-8 check would refuse it — right for a binary, wrong for the image
/// the developer has just added to the project and wants to look at.
#[test]
fn a_picture_opens_as_a_picture() {
    let file = CodeFile::load(Path::new("docs/maxx.png")).expect("a PNG must open");
    assert!(file.image);
    assert_eq!(file.name().to_string(), "maxx.png");
    // Nothing was read as text, so nothing is coloured or counted.
    assert!(file.text.is_empty());
    assert_eq!(file.lines(), 0);
    assert!(file.kilobytes() > 0);
}

#[test]
fn a_directory_is_refused_with_its_own_reason() {
    // A right click lands on a directory too, and "this file is not text" would
    // be a wrong answer there.
    let Err(reason) = CodeFile::load(Path::new("src")) else {
        panic!("a directory has no code");
    };
    // The message is translated, so both wordings are accepted: what matters is
    // that the refusal names a folder rather than a decoding failure.
    assert!(reason.contains("dossier") || reason.contains("folder"), "reason: {reason}");
}

#[test]
fn a_missing_file_is_refused_too() {
    assert!(CodeFile::load(Path::new("docs/nothing-at-all.rs")).is_err());
}

/// The smallest file carrying a managed region.
fn file_holding(expression: &str) -> String {
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
fn the_code_shown_is_the_one_save_would_write() {
    // The guarantee behind the canvas / code toggle: `render_source` and `save`
    // are the same text, because the second calls the first. The day one of them
    // gains a step the other does not have, this test falls.
    let directory = std::env::temp_dir().join("maxx-test-reader");
    std::fs::create_dir_all(&directory).expect("the test directory must be created");
    let path = directory.join("home.rs");
    std::fs::write(&path, file_holding("v_flex().gap_2()")).expect("the file must be written");

    let Ok(mut view) = maxx::view::View::load(&path) else {
        panic!("the view must load");
    };
    let Ok(shown) = view.render_source() else {
        panic!("the rendering must succeed");
    };
    // The rendering writes nothing: the disk has not moved.
    assert_eq!(std::fs::read_to_string(&path).unwrap(), file_holding("v_flex().gap_2()"));

    view.save().expect("the save must succeed");
    assert_eq!(std::fs::read_to_string(&path).unwrap(), shown);

    std::fs::remove_dir_all(&directory).ok();
}
