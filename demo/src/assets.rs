//! The project's own files, served to gpui.
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
