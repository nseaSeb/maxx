//! The palette of a generated project, edited without the file being rewritten.
//!
//! `src/theme.rs` is a module maxx copies and the developer owns. The whole
//! contract of editing it from maxx is that a colour change is a patch of the
//! literal and nothing else — so what these tests hold is not that the value
//! changed, which is easy, but that everything around it did not.

use std::path::PathBuf;

use maxx::scaffold::{self, Template};
use maxx::themefile::{Mode, ThemeFile};

/// A scratch directory of this test's own, removed when it is dropped.
struct Scratch(PathBuf);

impl Scratch {
    fn new(name: &str) -> Self {
        let base =
            std::env::var_os("MAXX_SCRATCH").map(PathBuf::from).unwrap_or_else(std::env::temp_dir);
        let root = base.join(format!("maxx-palette-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).expect("the scratch directory must be creatable");
        Self(root)
    }

    /// A project with its palette module in place.
    fn project(&self, name: &str) -> PathBuf {
        let root = self.0.join(name);
        scaffold::create_project(&root, name, Template::Empty).expect("the project is created");
        scaffold::add_theme_module(&root).expect("the palette is added");
        root
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// A project that has no palette shows none, rather than an empty one.
#[test]
fn a_project_without_the_module_has_no_palette() {
    let scratch = Scratch::new("absent");
    let root = scratch.0.join("bare");
    scaffold::create_project(&root, "bare", Template::Empty).unwrap();
    assert!(ThemeFile::load(&root).is_none());
}

/// Every role maxx writes is read back.
#[test]
fn the_palette_maxx_writes_is_the_palette_maxx_reads() {
    let scratch = Scratch::new("read");
    let root = scratch.project("trial");

    let palette = ThemeFile::load(&root).expect("the palette is there");
    let names: Vec<_> = palette.swatches().iter().map(|swatch| swatch.name.as_str()).collect();
    assert!(names.contains(&"BACKGROUND"), "{names:?}");
    assert!(names.contains(&"ACCENT"), "{names:?}");
    assert_eq!(names.len(), 10, "ten roles: {names:?}");
}

/// Writing a colour changes eight characters of the file, and no others.
///
/// The assertion the feature stands on. A rewrite would also produce a file
/// whose `ACCENT` is right — and would have quietly dropped the comment the
/// developer wrote three lines above it.
#[test]
fn writing_a_colour_leaves_the_rest_of_the_file_alone() {
    let scratch = Scratch::new("write");
    let root = scratch.project("trial");
    let path = ThemeFile::path_in(&root);

    // Something of the developer's own, in the middle of the palette, of a
    // shape maxx does not know: it has to come back out untouched.
    let before = std::fs::read_to_string(&path).unwrap();
    let with_theirs = before.replace(
        "/// Accent, for what the eye should land on first.",
        "// leur note à eux, gardée telle quelle\n/// Accent, for what the eye should land on first.",
    );
    std::fs::write(&path, &with_theirs).unwrap();

    let mut palette = ThemeFile::load(&root).expect("the palette is there");
    assert_eq!(palette.set("ACCENT", Mode::Light, 0x123456), Ok(true));

    let after = std::fs::read_to_string(&path).unwrap();
    assert_eq!(
        after,
        with_theirs.replace("light: 0x0969da", "light: 0x123456"),
        "only the literal moved"
    );
    assert!(after.contains("// leur note à eux, gardée telle quelle"), "their line survived");

    // And it is read back as what was written, from the file and not from
    // memory: the round trip is the point.
    let reread = ThemeFile::load(&root).expect("still there");
    let accent = reread.swatches().iter().find(|swatch| swatch.name == "ACCENT").unwrap();
    assert_eq!(accent.light, 0x123456);
    assert_eq!(accent.dark, 0x61afef, "the other mode did not move");
}

/// A role the developer added is editable like the rest.
#[test]
fn a_role_added_by_hand_is_offered_and_written() {
    let scratch = Scratch::new("added");
    let root = scratch.project("trial");
    let path = ThemeFile::path_in(&root);

    let mut source = std::fs::read_to_string(&path).unwrap();
    source.push_str("/// The colour of a warning.\npub const WARNING: Role = Role { dark: 0xd19a66, light: 0x9a6700 };\n");
    std::fs::write(&path, &source).unwrap();

    let mut palette = ThemeFile::load(&root).expect("the palette is there");
    assert_eq!(palette.swatches().len(), 11, "their role counts too");
    assert_eq!(palette.set("WARNING", Mode::Dark, 0x00ff00), Ok(true));

    let after = std::fs::read_to_string(&path).unwrap();
    assert!(after.contains("pub const WARNING: Role = Role { dark: 0x00ff00, light: 0x9a6700 };"));
}

/// The file maxx just wrote still compiles as Rust.
///
/// `0x00ff00` and `0x123456` are literals whatever they mean, but a writer that
/// dropped the `0x` or wrote seven digits would produce a palette that reads
/// back fine here and breaks the project on its next build. Ignored by default
/// for the reason every compiling test in this repository is: it wants cargo.
#[test]
#[ignore = "runs cargo check on a generated project"]
fn the_written_palette_still_compiles() {
    let scratch = Scratch::new("compiles");
    let root = scratch.project("trial");

    let mut palette = ThemeFile::load(&root).expect("the palette is there");
    for (index, swatch) in palette.swatches().to_vec().iter().enumerate() {
        let value = (index as u32 * 0x111111) & 0xffffff;
        palette.set(&swatch.name, Mode::Dark, value).expect("the palette must be writable");
        palette
            .set(&swatch.name, Mode::Light, 0xffffff - value)
            .expect("the palette must be writable");
    }

    let status = std::process::Command::new("cargo")
        .arg("check")
        .current_dir(&root)
        .status()
        .expect("cargo must run");
    assert!(status.success(), "the palette maxx wrote must compile");
}

/// A palette module brought up to date keeps the colours it was painted with.
///
/// The rule this states: the *values* of that file belong to the project, and
/// the code around them belongs to maxx. Bringing an accessor or a doc comment
/// up to date is not a reason to repaint somebody's application — and the whole
/// mechanism that makes updates possible would otherwise be a trap for anyone
/// who had chosen a colour.
#[test]
fn updating_the_palette_keeps_the_project_s_colours() {
    let scratch = Scratch::new("update-keeps");
    let root = scratch.project("trial");

    // Their colour, written the way maxx writes one.
    let mut palette = ThemeFile::load(&root).expect("the palette is there");
    assert_eq!(palette.set("ACCENT", Mode::Dark, 0x123456), Ok(true));

    // A project recorded as behind: what `File ▸ Update copied modules` acts on.
    let body = std::fs::read_to_string(ThemeFile::path_in(&root)).unwrap();
    maxx::projectfile::record(&root, "theme", 0, &body).unwrap();
    assert_eq!(scaffold::outdated_modules(&root), vec!["theme".to_string()]);

    scaffold::update_module(&root, "theme").expect("the update must go through");

    let after = ThemeFile::load(&root).expect("still there");
    let accent = after.swatches().iter().find(|s| s.name == "ACCENT").unwrap();
    assert_eq!(accent.dark, 0x123456, "their colour survived the update");
    assert_eq!(accent.light, 0x0969da, "and the one they did not touch is unchanged");
    // And it settled: the fingerprint recorded is the one of what was written.
    assert!(scaffold::outdated_modules(&root).is_empty());
}
