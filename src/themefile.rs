//! The project's palette, read out of `src/theme.rs` and written back into it.
//!
//! `src/theme.rs` is a **copied module**, not a managed region: maxx wrote it
//! once and the developer owns it from there. So this does not rewrite the
//! file. It finds the hexadecimal literal of the value being changed and
//! replaces *those bytes and no others* — the same move `parser::splice` makes
//! inside a view and the settings patcher makes inside a JSON file. Comments,
//! blank lines, a role the developer added, a role they reordered, a helper
//! they wrote underneath: all survive by construction rather than by care.
//!
//! What is not recognised is not offered, and not touched either. A role
//! written in a shape this reader does not know keeps its place in the file and
//! simply does not appear on the screen — the same bargain [`Base::Opaque`]
//! strikes on the canvas.
//!
//! [`Base::Opaque`]: crate::model::Base::Opaque
//!
//! One consequence worth stating, because it is a decision and not an
//! oversight: writing here makes the file differ from every version of the
//! template, so maxx stops recognising it as a copy it wrote and stops
//! offering to update it. That is the rule that already holds for the module
//! edited by hand in Zed, and choosing a colour is exactly as much of a claim
//! on the file.
//!
//! And the copy this holds is never what gets written back. It is what the
//! screen draws from, and it goes stale the moment anything else touches the
//! file — the developer in Zed, or `File ▸ Update copied modules`. Every write
//! therefore re-reads the file first and locates the literal in *those* bytes;
//! see [`ThemeFile::set`].

use std::ops::Range;
use std::path::{Path, PathBuf};

/// Which of the two values of a role.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    /// The value used when the application is in the dark mode.
    Dark,
    /// The value used when it is in the light one.
    Light,
}

impl Mode {
    /// The field name this mode is written under.
    fn field(self) -> &'static str {
        match self {
            Mode::Dark => "dark",
            Mode::Light => "light",
        }
    }
}

/// One role of the palette, as it stands in the file.
#[derive(Clone, Debug, PartialEq)]
pub struct Swatch {
    /// The constant's name, e.g. `BACKGROUND`.
    pub name: String,
    /// The doc comment written above it, without its `///`, joined into one
    /// line. Empty when there is none.
    pub doc: String,
    /// The dark value, as a 24-bit RGB number.
    pub dark: u32,
    /// The light one.
    pub light: u32,
    /// Byte range of the dark literal in the source.
    dark_at: Range<usize>,
    /// Byte range of the light one.
    light_at: Range<usize>,
}

impl Swatch {
    /// The value for one mode.
    pub fn value(&self, mode: Mode) -> u32 {
        match mode {
            Mode::Dark => self.dark,
            Mode::Light => self.light,
        }
    }

    /// Byte range of the literal for one mode.
    fn range(&self, mode: Mode) -> Range<usize> {
        match mode {
            Mode::Dark => self.dark_at.clone(),
            Mode::Light => self.light_at.clone(),
        }
    }
}

/// The palette file of an open project.
#[derive(Clone, Debug)]
pub struct ThemeFile {
    /// Where it lives, i.e. `<root>/src/theme.rs`.
    pub path: PathBuf,
    /// The whole source, carried so a write can be a patch rather than a
    /// rewrite.
    source: String,
    /// The roles this reader recognised, in the order the file writes them.
    swatches: Vec<Swatch>,
}

impl ThemeFile {
    /// Where the palette of `root` would live.
    pub fn path_in(root: &Path) -> PathBuf {
        root.join("src/theme.rs")
    }

    /// Whether `path` is a project's palette.
    ///
    /// By place and not by name alone: a `theme.rs` under `src/ui/` is a view,
    /// and opening it in the palette editor would put a designer's file behind
    /// a screen that cannot draw it.
    pub fn is_palette_file(path: &Path) -> bool {
        path.file_name().is_some_and(|name| name == "theme.rs")
            && path
                .parent()
                .and_then(|parent| parent.file_name())
                .is_some_and(|parent| parent == "src")
    }

    /// Reads the palette of the project rooted at `root`.
    ///
    /// `None` when the project has no palette module — which is the ordinary
    /// case, since it is added on demand — rather than an error: nothing has
    /// gone wrong, there is simply nothing to show.
    pub fn load(root: &Path) -> Option<Self> {
        Self::open(&Self::path_in(root))
    }

    /// The same, from the file itself.
    ///
    /// `None` also when nothing in it has the shape of a role: a `src/theme.rs`
    /// rewritten past recognition is a file, not an empty palette, and the
    /// caller shows it as one.
    pub fn open(path: &Path) -> Option<Self> {
        let source = std::fs::read_to_string(path).ok()?;
        let swatches = read_swatches(&source);
        if swatches.is_empty() {
            return None;
        }
        Some(Self { path: path.to_path_buf(), source, swatches })
    }

    /// The roles, in file order.
    pub fn swatches(&self) -> &[Swatch] {
        &self.swatches
    }

    /// The roles as `(name, dark, light)`, in file order.
    ///
    /// The whole state of the palette, for a caller that has to tell whether
    /// anything moved — including a value written by someone else.
    pub fn roles(&self) -> Vec<(String, u32, u32)> {
        self.swatches.iter().map(|s| (s.name.clone(), s.dark, s.light)).collect()
    }

    /// The names of the roles, in file order.
    ///
    /// What the pickers are built from, so what tells whether they have to be
    /// built again: a role added or removed elsewhere means one row of the
    /// editor has no picker behind it, or one picker has no row.
    pub fn role_names(&self) -> Vec<String> {
        self.swatches.iter().map(|swatch| swatch.name.clone()).collect()
    }

    /// Re-reads the file, answering whether anything in it moved.
    ///
    /// The copy held here is what the screen draws from, and it goes stale as
    /// soon as anything else writes: the developer in Zed, or `File ▸ Update
    /// copied modules`. Writing is safe without this — [`ThemeFile::set`]
    /// re-reads on its own — but the screen would go on showing colours the
    /// project no longer has, which is a lie a screen must not tell.
    pub fn reload(&mut self) -> bool {
        let Ok(source) = std::fs::read_to_string(&self.path) else {
            return false;
        };
        if source == self.source {
            return false;
        }
        self.swatches = read_swatches(&source);
        self.source = source;
        true
    }

    /// Sets one value, and writes the file.
    ///
    /// Reading and writing are one step on purpose. The copy held in memory is
    /// what the screen draws from, and it goes stale the moment anything else
    /// touches the file — the developer editing it in Zed, which maxx itself
    /// invites them to do, or `File ▸ Update copied modules` rewriting it. A
    /// write built on that copy would put the old file back and take their work
    /// with it, and nothing would say so. So the bytes patched here are the
    /// bytes on disk, read a moment ago, and the literal is located in *those*.
    ///
    /// Answers whether anything moved. A role that is gone from the file, or a
    /// value already equal to the one asked for, writes nothing — which is also
    /// what keeps a repaint from turning into a disk write.
    pub fn set(&mut self, name: &str, mode: Mode, value: u32) -> Result<bool, String> {
        let source = std::fs::read_to_string(&self.path).map_err(|error| error.to_string())?;
        let swatches = read_swatches(&source);

        let Some(swatch) = swatches.iter().find(|swatch| swatch.name == name) else {
            // The role was renamed or removed under us. Take what the file says
            // and write nothing: it is theirs.
            self.source = source;
            self.swatches = swatches;
            return Ok(false);
        };
        if swatch.value(mode) == value {
            self.source = source;
            self.swatches = swatches;
            return Ok(false);
        }

        let mut patched = source;
        patched.replace_range(swatch.range(mode), &format!("{value:#08x}"));
        std::fs::write(&self.path, &patched).map_err(|error| error.to_string())?;
        // The offsets of everything after the patch have moved; re-reading is
        // both the simplest way to restore them and the only one that cannot
        // drift out of step with the write above.
        self.swatches = read_swatches(&patched);
        self.source = patched;
        Ok(true)
    }

    /// The source as it stands, for the tests and for whoever wants to look.
    pub fn source(&self) -> &str {
        &self.source
    }
}

/// Finds every `pub const NAME: Role = Role { dark: 0x…, light: 0x… };`.
///
/// Deliberately narrow, and narrow in a way that fails safe: a shape this does
/// not match is not read, so it is never written either.
fn read_swatches(source: &str) -> Vec<Swatch> {
    let mut out = Vec::new();
    let bytes = source.as_bytes();
    let mut at = 0;

    while let Some(found) = source[at..].find("pub const ") {
        let start = at + found;
        let head = start + "pub const ".len();
        at = head;
        // At the start of a line, or it is a `pub const` inside something else.
        if start > 0 && bytes[start - 1] != b'\n' {
            continue;
        }
        let Some(colon) = source[head..].find(':') else {
            break;
        };
        let name = source[head..head + colon].trim().to_string();
        let rest = head + colon;
        // The statement ends at its semicolon, and everything below looks only
        // inside it. Without that bound, a `pub const WIDTH: f32 = 4.0;` reaches
        // past its own end and reads the braces of the role written after it —
        // a constant that is not a role, holding the values of one that is.
        let Some(semi) = source[rest..].find(';') else {
            break;
        };
        let statement = rest..rest + semi;
        at = statement.end;

        let inside = &source[statement.clone()];
        let (Some(open), Some(close)) = (inside.find('{'), inside.rfind('}')) else {
            continue;
        };
        if open >= close {
            continue;
        }
        let body = statement.start + open..statement.start + close;
        // Only a `Role`, and only between the braces: a `const` of another type
        // whose initializer happens to hold `dark:` is not a role.
        if !inside[..open].contains("Role") {
            continue;
        }
        let (Some(dark_at), Some(light_at)) = (
            hex_range(source, body.clone(), Mode::Dark),
            hex_range(source, body.clone(), Mode::Light),
        ) else {
            continue;
        };
        let (Ok(dark), Ok(light)) = (
            u32::from_str_radix(source[dark_at.clone()].trim_start_matches("0x"), 16),
            u32::from_str_radix(source[light_at.clone()].trim_start_matches("0x"), 16),
        ) else {
            continue;
        };

        out.push(Swatch { name, doc: doc_above(source, start), dark, light, dark_at, light_at });
    }
    out
}

/// The byte range of the hexadecimal literal of `mode` inside `body`.
fn hex_range(source: &str, body: Range<usize>, mode: Mode) -> Option<Range<usize>> {
    let field = mode.field();
    let inside = &source[body.clone()];
    // The field name, then its colon: `dark_grey: 0x…` is not `dark`.
    let mut search = 0;
    let key = loop {
        let found = inside[search..].find(field)? + search;
        let after = found + field.len();
        let before_ok = found == 0 || !is_word(inside.as_bytes()[found - 1]);
        let after_ok = inside[after..].trim_start().starts_with(':');
        if before_ok && after_ok {
            break after;
        }
        search = after;
    };
    // Bounded to this field. Unbounded, the search walks past the comma into
    // the *next* field and answers with its literal — so a role written
    // `Role { dark: shade(2), light: 0x222222 }` would hand back the same range
    // for both modes, and setting the dark one would overwrite the light one
    // the developer never touched. The reader must not read what the writer
    // would then wreck.
    let field_end = inside[key..].find(',').map(|at| key + at).unwrap_or(inside.len());
    let value = inside[key..field_end].find("0x")? + key;
    let end = inside[value + 2..field_end]
        .find(|c: char| !c.is_ascii_hexdigit() && c != '_')
        .map(|offset| value + 2 + offset)
        .unwrap_or(field_end);
    Some(body.start + value..body.start + end)
}

/// Whether `byte` can be part of a Rust identifier.
fn is_word(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || byte == b'_'
}

/// The `///` lines written straight above `start`, joined into one.
fn doc_above(source: &str, start: usize) -> String {
    let mut lines: Vec<&str> = Vec::new();
    let mut end = start;
    while end > 0 {
        let line_start = source[..end - 1].rfind('\n').map(|at| at + 1).unwrap_or(0);
        let line = source[line_start..end - 1].trim();
        match line.strip_prefix("///") {
            Some(text) => lines.push(text.trim()),
            None => break,
        }
        end = line_start;
    }
    lines.reverse();
    lines.join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> String {
        crate::scaffold::module_body("theme").expect("the palette template")
    }

    /// A palette file of this test's own, removed when it is dropped.
    struct Scratch(PathBuf);

    impl Scratch {
        fn with(source: &str, name: &str) -> Self {
            let root =
                std::env::temp_dir().join(format!("maxx-themefile-{name}-{}", std::process::id()));
            let _ = std::fs::remove_dir_all(&root);
            std::fs::create_dir_all(root.join("src")).expect("the directory must be creatable");
            std::fs::write(ThemeFile::path_in(&root), source).expect("the file must be writable");
            Self(root)
        }

        fn load(&self) -> ThemeFile {
            ThemeFile::load(&self.0).expect("the palette must load")
        }

        fn source(&self) -> String {
            std::fs::read_to_string(ThemeFile::path_in(&self.0)).expect("the file must be readable")
        }
    }

    impl Drop for Scratch {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn every_role_of_the_template_is_read() {
        let swatches = read_swatches(&sample());
        let names: Vec<_> = swatches.iter().map(|swatch| swatch.name.as_str()).collect();
        assert_eq!(
            names,
            [
                "BACKGROUND",
                "PANEL",
                "HOVER",
                "SELECTED",
                "BORDER",
                "TEXT",
                "TEXT_MUTED",
                "ACCENT",
                "ON_ACCENT",
                "DANGER"
            ]
        );
        let background = &swatches[0];
        assert_eq!(background.dark, 0x1e2127);
        assert_eq!(background.light, 0xfafafa);
        assert_eq!(background.doc, "Background of the main area.");
    }

    /// A write moves the value and nothing else.
    #[test]
    fn setting_a_value_touches_only_its_literal() {
        let scratch = Scratch::with(&sample(), "only-literal");
        let mut file = scratch.load();
        assert_eq!(file.set("ACCENT", Mode::Light, 0x123456), Ok(true));

        assert_eq!(scratch.source(), sample().replace("light: 0x0969da };", "light: 0x123456 };"));
        assert_eq!(file.swatches()[7].light, 0x123456, "and the offsets were rebuilt");
    }

    #[test]
    fn a_value_already_there_writes_nothing() {
        let scratch = Scratch::with(&sample(), "no-op");
        let mut file = scratch.load();
        assert_eq!(file.set("ACCENT", Mode::Light, 0x0969da), Ok(false), "the same value");
        assert_eq!(file.set("NOWHERE", Mode::Dark, 0x000000), Ok(false), "an unknown role");
        assert_eq!(scratch.source(), sample());
    }

    /// What was written into the file after it was read is not overwritten.
    ///
    /// The failure this guards against had no symptom: the developer edits
    /// `src/theme.rs` in Zed — which maxx invites them to do — comes back,
    /// changes one colour, and their work is silently replaced by the copy maxx
    /// happened to be holding. Nothing errors, and the file looks plausible.
    #[test]
    fn an_edit_made_after_loading_survives_the_next_write() {
        let scratch = Scratch::with(&sample(), "stale");
        let mut file = scratch.load();

        // Their edit, after maxx read the file and while it still holds a copy.
        let theirs = format!(
            "{}\n/// The colour of a warning.\npub const WARNING: Role = Role {{ dark: 0xd19a66, light: 0x9a6700 }};\n",
            sample()
        );
        std::fs::write(ThemeFile::path_in(&scratch.0), &theirs).unwrap();

        assert_eq!(file.set("ACCENT", Mode::Light, 0x123456), Ok(true));

        let after = scratch.source();
        assert!(after.contains("pub const WARNING"), "their role survived: {after}");
        assert_eq!(after, theirs.replace("light: 0x0969da };", "light: 0x123456 };"));
    }

    /// A role that disappeared under us is not put back.
    #[test]
    fn a_role_removed_after_loading_is_not_written_again() {
        let scratch = Scratch::with(&sample(), "removed");
        let mut file = scratch.load();

        let theirs = sample().replace(
            "/// Accent, for what the eye should land on first.\npub const ACCENT: Role = Role { dark: 0x61afef, light: 0x0969da };\n",
            "",
        );
        std::fs::write(ThemeFile::path_in(&scratch.0), &theirs).unwrap();

        assert_eq!(file.set("ACCENT", Mode::Light, 0x123456), Ok(false), "nothing to write");
        assert_eq!(scratch.source(), theirs, "and nothing was written");
    }

    /// A role the developer added is read like the others.
    #[test]
    fn a_role_added_by_hand_is_read_too() {
        let source = format!(
            "{}\n/// The colour of a warning.\npub const WARNING: Role = Role {{ dark: 0xd19a66, light: 0x9a6700 }};\n",
            sample()
        );
        let swatches = read_swatches(&source);
        let warning = swatches.last().expect("at least one role");
        assert_eq!(warning.name, "WARNING");
        assert_eq!(warning.dark, 0xd19a66);
        assert_eq!(warning.doc, "The colour of a warning.");
    }

    /// What the reader does not understand, it leaves alone.
    #[test]
    fn an_unrecognised_shape_is_neither_read_nor_written() {
        let source = "\
/// Computed elsewhere.
pub const ODD: Role = Role { dark: shade(2), light: shade(9) };
/// A constant that is not a role at all.
pub const WIDTH: f32 = 4.0;
/// Plain.
pub const FINE: Role = Role { dark: 0x111111, light: 0x222222 };
";
        let swatches = read_swatches(source);
        let names: Vec<_> = swatches.iter().map(|swatch| swatch.name.as_str()).collect();
        assert_eq!(names, ["FINE"], "only the shape the writer can put back");
    }

    /// A role whose other mode is not a literal does not lend it its own.
    ///
    /// The failure this guards against destroys a value nobody touched: with
    /// the search for `0x` unbounded, `dark:` finds nothing before the comma
    /// and walks into `light:`, so both modes point at the same literal and
    /// setting one overwrites the other. The reader must not read what the
    /// writer would then wreck.
    #[test]
    fn a_computed_mode_does_not_borrow_the_other_s_literal() {
        let source = "pub const ODD: Role = Role { dark: shade(2), light: 0x222222 };\n";
        assert!(read_swatches(source).is_empty(), "half a role is not a role");

        // And the mirror, so the bound is checked on both sides.
        let source = "pub const ODD: Role = Role { dark: 0x111111, light: shade(9) };\n";
        assert!(read_swatches(source).is_empty(), "half a role is not a role");
    }

    /// What the file gains while the page is open is picked up.
    #[test]
    fn reload_takes_what_the_file_says_now() {
        let scratch = Scratch::with(&sample(), "reload");
        let mut file = scratch.load();
        assert!(!file.reload(), "nothing moved yet");

        std::fs::write(
            ThemeFile::path_in(&scratch.0),
            format!(
                "{}\npub const WARNING: Role = Role {{ dark: 0xd19a66, light: 0x9a6700 }};\n",
                sample()
            ),
        )
        .unwrap();

        assert!(file.reload(), "the file moved");
        assert!(file.role_names().contains(&"WARNING".to_string()), "{:?}", file.role_names());
    }

    /// The field name is matched whole.
    #[test]
    fn a_longer_field_name_is_not_mistaken_for_the_short_one() {
        let source =
            "pub const ONE: Role = Role { darkest: 0xaaaaaa, dark: 0xbbbbbb, light: 0xcccccc };\n";
        let swatches = read_swatches(source);
        assert_eq!(swatches.len(), 1);
        assert_eq!(swatches[0].dark, 0xbbbbbb, "`darkest` is not `dark`");
        assert_eq!(swatches[0].light, 0xcccccc);
    }
}
