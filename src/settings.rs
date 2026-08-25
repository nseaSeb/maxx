//! What maxx remembers between two launches.
//!
//! One TOML file, hand-editable, in the place the running system puts
//! configuration. TOML rather than JSON because the rest of a Rust project is
//! already TOML, and because a file the user is invited to open should accept
//! comments — even if writing it back drops them, which is why maxx only ever
//! rewrites the whole file from the values it holds.
//!
//! Everything here has a default that reproduces the behaviour maxx had before
//! it read any settings: a missing, empty or damaged file must never be worse
//! than no file at all.

use std::path::{Path, PathBuf};

use gpui::{App, Global};
use serde::{Deserialize, Serialize};

/// How many projects the "Open Recent" list keeps.
const RECENT_LIMIT: usize = 10;

/// The saved position and size of the workspace window.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct WindowGeometry {
    /// Distance from the left of the display, in logical pixels.
    pub x: f32,
    /// Distance from the top of the display, in logical pixels.
    pub y: f32,
    /// Width in logical pixels.
    pub width: f32,
    /// Height in logical pixels.
    pub height: f32,
}

/// Everything maxx keeps between two launches.
///
/// `serde(default)` on the struct is what makes an old file — or one a hand
/// edit truncated — still load: a missing key falls back to the default rather
/// than failing the whole parse.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    /// Projects opened before, most recent first.
    pub recent_projects: Vec<PathBuf>,
    /// Whether the project panel is shown.
    pub show_project_panel: bool,
    /// Whether the status bar is shown.
    pub show_status_bar: bool,
    /// Whether the output panel is shown.
    pub show_output: bool,
    /// Where the workspace window was left.
    pub window: Option<WindowGeometry>,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            recent_projects: Vec::new(),
            show_project_panel: true,
            show_status_bar: true,
            show_output: false,
            window: None,
        }
    }
}

impl Settings {
    /// Reads the settings file, falling back to the defaults.
    ///
    /// A damaged file is reported on stderr and then ignored. Refusing to start
    /// over a stray comma would be a poor trade, and overwriting it silently
    /// would lose whatever the user was in the middle of writing — so it stays
    /// on disk, untouched, until something else asks for a save.
    pub fn load() -> Self {
        match Self::path() {
            Some(path) => Self::load_from(&path),
            None => Self::default(),
        }
    }

    /// Reads a settings file by path.
    ///
    /// Split out from [`load`](Self::load) so it can be exercised without
    /// touching the settings of the machine running the tests — and so a
    /// future per-project settings file can reuse it.
    pub fn load_from(path: &Path) -> Self {
        let Ok(source) = std::fs::read_to_string(path) else {
            return Self::default();
        };
        match toml::from_str(&source) {
            Ok(settings) => settings,
            Err(error) => {
                eprintln!("{} illisible : {error}", path.display());
                Self::default()
            }
        }
    }

    /// Writes the settings file, creating its directory.
    ///
    /// Written to a neighbouring temporary file and renamed over the target:
    /// a crash halfway through a direct write leaves a truncated file, which
    /// the next launch would read as a damaged one.
    pub fn save(&self) -> std::io::Result<()> {
        match Self::path() {
            Some(path) => self.save_to(&path),
            None => Ok(()),
        }
    }

    /// Writes a settings file by path.
    pub fn save_to(&self, path: &Path) -> std::io::Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let body = toml::to_string_pretty(self)
            .map_err(|error| std::io::Error::new(std::io::ErrorKind::InvalidData, error))?;
        let temporary = path.with_extension("toml.tmp");
        std::fs::write(&temporary, header() + &body)?;
        std::fs::rename(&temporary, path)
    }

    /// Where the settings file lives on this system.
    ///
    /// The three conventions, in order: `XDG_CONFIG_HOME` when the user set it,
    /// `APPDATA` on Windows, and otherwise the home directory — under
    /// `Library/Application Support` on macOS, `.config` elsewhere.
    pub fn path() -> Option<PathBuf> {
        let directory = if cfg!(target_os = "windows") {
            PathBuf::from(std::env::var("APPDATA").ok()?).join("maxx")
        } else if let Ok(xdg) = std::env::var("XDG_CONFIG_HOME")
            && !xdg.is_empty()
        {
            PathBuf::from(xdg).join("maxx")
        } else {
            let home = PathBuf::from(std::env::var("HOME").ok()?);
            if cfg!(target_os = "macos") {
                home.join("Library/Application Support/maxx")
            } else {
                home.join(".config/maxx")
            }
        };
        Some(directory.join("settings.toml"))
    }

    /// Puts `path` at the head of the recent list.
    ///
    /// Answers whether the list changed, so the caller can skip rewriting the
    /// menu bar and the file when reopening the project already at the head.
    pub fn remember_project(&mut self, path: &Path) -> bool {
        if self.recent_projects.first().map(PathBuf::as_path) == Some(path) {
            return false;
        }
        self.recent_projects.retain(|recent| recent != path);
        self.recent_projects.insert(0, path.to_path_buf());
        self.recent_projects.truncate(RECENT_LIMIT);
        true
    }

    /// Drops the recent entries whose directory no longer exists.
    ///
    /// Run at startup: a menu offering a project that was moved or deleted is
    /// worse than a shorter menu.
    pub fn forget_missing_projects(&mut self) {
        self.recent_projects.retain(|path| path.is_dir());
    }
}

/// The settings of the running application.
struct Store(Settings);

impl Global for Store {}

/// Loads the settings into the application.
///
/// Called once, before the first window: everything downstream reads them
/// through [`get`].
pub fn init(cx: &mut App) {
    let mut settings = Settings::load();
    settings.forget_missing_projects();
    cx.set_global(Store(settings));
}

/// The settings as they stand.
pub fn get(cx: &App) -> &Settings {
    &cx.global::<Store>().0
}

/// Changes the settings and writes them to disk.
///
/// Saved on every change rather than at quit: the file is a few hundred bytes,
/// and a workshop that loses your preferences when it crashes is a workshop
/// whose preferences you stop setting.
pub fn update(cx: &mut App, change: impl FnOnce(&mut Settings)) {
    let mut settings = cx.global::<Store>().0.clone();
    change(&mut settings);
    if settings == cx.global::<Store>().0 {
        return;
    }
    if let Err(error) = settings.save() {
        eprintln!("réglages non enregistrés : {error}");
    }
    cx.set_global(Store(settings));
}

/// The comment maxx puts at the top of the file it writes.
fn header() -> String {
    "# Réglages de maxx. Écrit par l'application : les commentaires que vous\n\
     # ajoutez ici disparaissent au prochain enregistrement.\n\n"
        .to_string()
}

/// Changes the settings in memory, without touching the disk.
///
/// For the values that move continuously — the window being dragged or
/// resized — where writing a file per frame would be absurd. [`flush`] puts
/// them away at quit.
pub fn stage(cx: &mut App, change: impl FnOnce(&mut Settings)) {
    let mut settings = cx.global::<Store>().0.clone();
    change(&mut settings);
    if settings != cx.global::<Store>().0 {
        cx.set_global(Store(settings));
    }
}

/// Writes whatever [`stage`] left in memory.
pub fn flush(cx: &App) {
    if let Err(error) = cx.global::<Store>().0.save() {
        eprintln!("réglages non enregistrés : {error}");
    }
}
