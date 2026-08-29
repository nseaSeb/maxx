//! Assets: an image dropped on a view, and the module that serves it.

use rust_i18n::t;
use std::io;
use std::path::Path;

use super::modules::{header_end, joined, legacy_copy, module_version};
/// Where an image dropped on a view is kept.
///
/// Under `assets/`, because that is the name every framework gives it and the
/// one a developer opening the project will look for; under `images/`, because
/// fonts and icons will want their own place beside it.
pub const IMAGE_DIRECTORY: &str = "assets/images";

/// Copies `file` into the project and answers the path to write, relative to
/// the root, with forward slashes.
///
/// Relative and inside the project, or the image shows on one machine only:
/// `img(PathBuf::from(..))` is read from the directory the binary starts in,
/// which is the project root. Picking a file from the desktop and writing its
/// absolute path would draw on the canvas and nowhere else.
///
/// A file already inside the project is left exactly where it is — that is the
/// developer's layout, and moving it would break whatever else points at it.
///
/// A name already taken by a *different* file is numbered rather than
/// overwritten; the same file imported twice is recognised by its bytes and
/// imported once.
pub fn import_asset(root: &Path, file: &Path) -> Result<String, String> {
    if !crate::project::is_image(file) {
        return Err(crate::tr("error.not_an_image").to_string());
    }
    let extension =
        file.extension().and_then(|value| value.to_str()).unwrap_or_default().to_lowercase();

    let resolved = file.canonicalize().unwrap_or_else(|_| file.to_path_buf());
    // The same ceiling the reader holds, and before the shortcut below: a file
    // already in the project is not a file maxx can draw, and accepting one it
    // would then refuse to open is the two paths disagreeing about the same
    // picture.
    // A size that cannot be read is not zero: treating it as zero would let
    // through the very whole-file read this ceiling exists to prevent.
    let size = std::fs::metadata(&resolved).map_err(|error| error.to_string())?.len();
    if size > crate::project::MAX_IMAGE_BYTES {
        return Err(t!("error.file_too_large", size = size / 1024).into_owned());
    }

    let inside = root.canonicalize().unwrap_or_else(|_| root.to_path_buf());
    if let Ok(relative) = resolved.strip_prefix(&inside) {
        return Ok(slashed(relative));
    }

    let stem = file.file_stem().and_then(|value| value.to_str()).unwrap_or("image").to_string();
    let directory = root.join(IMAGE_DIRECTORY);
    std::fs::create_dir_all(&directory).map_err(|error| error.to_string())?;

    let bytes = std::fs::read(&resolved).map_err(|error| error.to_string())?;
    let mut name = format!("{stem}.{extension}");
    let mut index = 2;
    // A file that cannot be read is a file that differs, not a file to write
    // over: `is_ok_and` answered the same thing for "same bytes" and "could not
    // look", and the second case overwrote what it had not seen.
    while directory.join(&name).exists()
        && !std::fs::read(directory.join(&name)).is_ok_and(|existing| existing == bytes)
    {
        name = format!("{stem}-{index}.{extension}");
        index += 1;
    }

    std::fs::write(directory.join(&name), &bytes).map_err(|error| error.to_string())?;
    Ok(format!("{IMAGE_DIRECTORY}/{name}"))
}

/// A path written the one way `PathBuf` reads everywhere.
fn slashed(path: &Path) -> String {
    path.components()
        .map(|component| component.as_os_str().to_string_lossy().into_owned())
        .collect::<Vec<_>>()
        .join("/")
}

/// Adds the assets module to an existing project, with its build script, and
/// hands it to the application.
///
/// Two files, and both are needed: `src/assets.rs` declares the `AssetSource`,
/// `build.rs` is what embeds the files it serves. The wiring is a single call
/// on `Application::new()`, so removing the module leaves a line that still
/// compiles.
pub fn add_assets_module(root: &Path) -> io::Result<()> {
    if let Some(error) = legacy_copy(root, "assets") {
        return Err(error);
    }

    // A build script the developer wrote is theirs: appending to it would put
    // maxx in the middle of a file it does not understand.
    let build_path = root.join("build.rs");
    let ours = std::fs::read_to_string(&build_path).is_ok_and(|body| body.contains(BUILD_MARKER));
    if build_path.exists() && !ours {
        return Err(io::Error::new(
            io::ErrorKind::AlreadyExists,
            "build.rs is already yours — copy the body of maxx's own build script into \
             its main by hand, or move yours aside and add the module again",
        ));
    }

    // `main.rs` is patched first, and nothing is written until it is known to
    // work: the two files left behind by a failed wiring would make the next
    // attempt believe the project already carries the module.
    let main_path = root.join("src/main.rs");
    let source = std::fs::read_to_string(&main_path)?;
    let mut lines: Vec<String> = source.lines().map(str::to_string).collect();

    if !lines.iter().any(|line| line.trim() == "mod assets;") {
        lines.insert(header_end(&lines), "mod assets;".into());
    }

    if !source.contains(WITH_ASSETS) {
        let Some(anchor) = lines.iter().position(|line| line.contains(APPLICATION_NEW)) else {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "src/main.rs: no {APPLICATION_NEW} — add {WITH_ASSETS} to whatever \
                     builds the application by hand"
                ),
            ));
        };
        lines[anchor] =
            lines[anchor].replacen(APPLICATION_NEW, &format!("{APPLICATION_NEW}{WITH_ASSETS}"), 1);
    }

    let path = root.join("src/assets.rs");
    let body = assets_rs();
    let wrote_module = !path.exists();
    if wrote_module {
        std::fs::write(&path, &body)?;
    }
    let wrote_build = !build_path.exists();
    if wrote_build {
        std::fs::write(&build_path, assets_build_rs())?;
    }

    if let Err(error) = std::fs::write(&main_path, joined(&lines, &source)) {
        if wrote_module {
            let _ = std::fs::remove_file(&path);
        }
        if wrote_build {
            let _ = std::fs::remove_file(&build_path);
        }
        return Err(error);
    }
    if wrote_module {
        crate::projectfile::record(root, "assets", module_version("assets").unwrap_or(1), &body)?;
    }
    Ok(())
}

/// What tells maxx's build script apart from one the developer wrote.
pub(super) const BUILD_MARKER: &str = "maxx:assets";

/// The call the assets module hangs on, and the one it is added to.
const APPLICATION_NEW: &str = "Application::new()";

pub(super) const WITH_ASSETS: &str = ".with_assets(assets::Assets)";

/// The assets module of a generated project.
pub(super) fn assets_rs() -> String {
    r##"//! The project's own files, served to gpui.
//!
//! `img("assets/images/logo.png")` asks gpui for an *asset*, and gpui asks the
//! `AssetSource` the application was built with. Without one, nothing is drawn
//! and a single line goes to the log — so this module is what makes the
//! pictures appear.
//!
//! It answers from two places, in this order:
//!
//! 1. What `build.rs` embedded in the binary: everything under `assets/` and
//!    `icons/` at build time is compiled into the executable, so the
//!    application carries its pictures wherever it goes. A binary someone
//!    double-clicks has no idea where the project directory is.
//! 2. Failing that, the file on disk, read from the directory the process
//!    started in. That is what picks up a picture added since the last build,
//!    and what serves the ones kept outside `assets/`.
//!
//! The price is plain, and it is worth saying out loud: the binary grows by
//! the size of what is embedded.
//!
//! The contract with `build.rs` — change one, change the other: the build
//! script writes `assets.rs` into `OUT_DIR`, declaring `ASSETS`, a table of
//! project-relative path and bytes.
//!
//! `icons/` is walked for `gpui_component`, whose `IconName` asks for
//! `icons/*.svg` as assets: dropping that folder at the root of the project is
//! all it takes for the icons to appear.

#![allow(dead_code)]

use std::borrow::Cow;

use gpui::{AssetSource, Result, SharedString};

// Written by build.rs: `pub static ASSETS: &[(&str, &[u8])]`.
include!(concat!(env!("OUT_DIR"), "/assets.rs"));

/// Hand it over at startup: `Application::new().with_assets(assets::Assets)`.
pub struct Assets;

impl AssetSource for Assets {
    fn load(&self, path: &str) -> Result<Option<Cow<'static, [u8]>>> {
        for (key, bytes) in ASSETS {
            if *key == path {
                return Ok(Some(Cow::Borrowed(*bytes)));
            }
        }
        match std::fs::read(path) {
            Ok(bytes) => Ok(Some(Cow::Owned(bytes))),
            // Missing is not an error: gpui logs what it cannot load and draws
            // the fallback, and one absent picture is no reason to stop.
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(error) => Err(error.into()),
        }
    }

    fn list(&self, path: &str) -> Result<Vec<SharedString>> {
        let mut names: Vec<SharedString> = ASSETS
            .iter()
            // On a segment boundary: `list("icons")` has no business answering
            // for a sibling `icons_extra/`.
            .filter(|(key, _)| key.strip_prefix(path).is_some_and(|rest| rest.starts_with('/')))
            .map(|(key, _)| SharedString::from(*key))
            .collect();

        if let Ok(entries) = std::fs::read_dir(path) {
            for entry in entries.flatten() {
                let name = entry.path().to_string_lossy().replace('\\', "/");
                if !names.iter().any(|known| known.as_ref() == name) {
                    names.push(SharedString::from(name));
                }
            }
        }
        Ok(names)
    }
}
"##
    .to_string()
}

/// The build script that goes with the assets module.
pub(super) fn assets_build_rs() -> String {
    r##"//! Embeds the project's own files into the binary. maxx:assets
//!
//! Walks the directories of `ROOTS` and writes `assets.rs` into `OUT_DIR`,
//! which `src/assets.rs` includes. The contract between the two is this file's
//! output: `pub static ASSETS: &[(&str, &[u8])]`, keyed by the path relative
//! to the project root — the very string the code hands to `img(…)`.
//!
//! Written by maxx, yours from here. Add a directory to `ROOTS` and it travels
//! inside the binary too.

use std::fmt::Write as _;
use std::path::{Path, PathBuf};

/// The directories whose contents travel inside the binary.
const ROOTS: &[&str] = &["assets", "icons"];

fn main() {
    let mut table = String::from(
        "// Written by build.rs. Do not edit.\npub static ASSETS: &[(&str, &[u8])] = &[\n",
    );
    for root in ROOTS {
        println!("cargo::rerun-if-changed={root}");
        collect(Path::new(root), &mut table);
    }
    table.push_str("];\n");

    let out = PathBuf::from(std::env::var("OUT_DIR").expect("cargo sets OUT_DIR"));
    std::fs::write(out.join("assets.rs"), table).expect("the asset table must be written");
}

/// Adds every file under `directory` to the table, recursively.
///
/// A directory that is not there is not a failure: a project keeps its
/// pictures where it likes, and `icons/` is only there once someone wants the
/// gpui-component icons.
fn collect(directory: &Path, table: &mut String) {
    let Ok(entries) = std::fs::read_dir(directory) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let name = path.file_name().and_then(|name| name.to_str()).unwrap_or(".");
        // Dotfiles belong to the system, not to the project: `.DS_Store` inside
        // the binary is bytes nobody asked for.
        if name.starts_with('.') {
            continue;
        }
        if path.is_dir() {
            collect(&path, table);
            continue;
        }
        // Forward slashes, whatever the system: the key has to match the string
        // written in the source, and that one is written once.
        let key = path.to_string_lossy().replace('\\', "/");
        println!("cargo::rerun-if-changed={key}");
        // `{key:?}` twice, and not once: a quote or a backslash in a file name
        // has to be escaped in the key and in the path just the same.
        let _ = writeln!(
            table,
            "    ({key:?}, include_bytes!(concat!(env!(\"CARGO_MANIFEST_DIR\"), \"/\", {key:?}))),"
        );
    }
}
"##
    .to_string()
}
