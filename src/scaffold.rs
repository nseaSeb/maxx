//! Project and view templates.
//!
//! Everything written here is ordinary Rust that compiles and runs without
//! `maxx`. The only trace `maxx` leaves is a pair of marker comments around the
//! expression it owns.

use std::io;
use std::path::Path;

use menubar::menus_rs;
use views::{declare_ui_module, view_rs};

mod assets;
mod components;
mod menubar;
mod modules;
mod settings;
mod system;
pub mod templates;
mod theme;
mod views;
mod window;

pub use assets::add_assets_module;
pub use assets::{IMAGE_DIRECTORY, import_asset};
pub use components::{add_components_module, add_components_module_with};
pub use menubar::{add_menu_bar, remove_menu_bar};
pub use modules::is_directory as module_is_directory;
pub use modules::{
    MODULES, module_body, module_version, outdated_modules, remove_module, update_module,
};
pub use settings::add_settings_module;
pub use system::add_system_module;
pub use templates::{settings_screen_rs, shell_rs};
pub use theme::{add_theme_module, add_theme_module_with};
pub use views::{create_view, rename_view, set_entry_view};
pub use window::add_window_module;

/// The shape a new project is given.
///
/// Not screens to pick from a gallery. The first three answer the question a
/// desktop project asks on its first day, which is *what holds what*: a sidebar
/// and a settings screen arrive here rather than in the component palette
/// because they are not elements to drop on a canvas, they are what a canvas
/// hangs from. The six after them answer the other question, *what does it do*
/// — a list that has a detail panel, fields that are typed into, steps that
/// move. Each is a shell around pages, and every page is a view maxx keeps
/// drawing.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum Template {
    /// One window, one view, and nothing else to unlearn.
    #[default]
    Empty,
    /// A sidebar on the left, the view of the moment on the right.
    Sidebar,
    /// The same, with the settings module and a screen that reads and writes
    /// it.
    Settings,
    /// A list of items, and a panel that shows the one selected.
    ListDetail,
    /// Fields bound to state, and a button that reads them.
    Form,
    /// A header, a grid of cards, and the numbers they carry.
    Dashboard,
    /// Steps, an indicator, and the two buttons that move between them.
    Wizard,
    /// One compact window and one job: no sidebar, so no shell.
    Utility,
    /// A strip of tabs, a text area, and a status bar.
    Editor,
}

impl Template {
    /// The name this shape carries in `maxx.toml` and in the tests.
    pub fn name(self) -> &'static str {
        match self {
            Self::Empty => "empty",
            Self::Sidebar => "sidebar",
            Self::Settings => "settings",
            Self::ListDetail => "list_detail",
            Self::Form => "form",
            Self::Dashboard => "dashboard",
            Self::Wizard => "wizard",
            Self::Utility => "utility",
            Self::Editor => "editor",
        }
    }

    /// Every shape there is. A variant added above joins this list, which is
    /// what `maxx new --shape` offers and refuses against.
    pub const ALL: &'static [Template] = &[
        Self::Empty,
        Self::Sidebar,
        Self::Settings,
        Self::ListDetail,
        Self::Form,
        Self::Dashboard,
        Self::Wizard,
        Self::Utility,
        Self::Editor,
    ];

    /// The shape `name` names, `name()` read backwards.
    pub fn from_name(name: &str) -> Option<Self> {
        Self::ALL.iter().copied().find(|template| template.name() == name)
    }

    /// Whether the shape hangs its pages off `shell.rs`.
    ///
    /// Which is also the answer to whether the window draws a title bar of its
    /// own, and that is not a coincidence: `shell.rs` is the only file that
    /// draws `TitleBar::new()`, so a window opened with
    /// `TitleBar::title_bar_options()` and no shell behind it is a bare strip
    /// where the traffic lights sit.
    pub fn has_shell(self) -> bool {
        !matches!(self, Self::Empty | Self::Utility)
    }

    /// The window the shape opens, in points.
    ///
    /// A utility is a tool, not an application: it opens at the size of what it
    /// holds, and a 900 by 600 window with four controls in the corner is the
    /// thing that makes it look unfinished.
    fn window_size(self) -> (u32, u32) {
        match self {
            Self::Utility => (480, 360),
            _ => (900, 600),
        }
    }
}

/// The crate name a project gets from where it is written.
///
/// The last segment of the path, which is what the developer typed in the save
/// panel or on the command line: two ways in, one answer, so `maxx new` and
/// `File > New project` cannot name the same directory differently.
/// `.` and `..` are resolved first, and that is the whole reason this is not a
/// one-liner: `Path::new(".").file_name()` is `None`, so
/// `mkdir app && cd app && maxx new .` — an ordinary command-line habit —
/// silently produced a crate called `mon_app`. The save panel can never hand
/// over a relative path, so only the command line ever met it, and the promise
/// of one answer for both ways in was quietly false.
pub fn project_name(root: &Path) -> String {
    let resolved = root.canonicalize();
    let path = resolved.as_deref().unwrap_or(root);
    path.file_name()
        .map(|name| name.to_string_lossy().into_owned())
        // Only left for a path that names no directory at all — `/` — where
        // there is nothing to read the name from.
        .unwrap_or_else(|| "mon_app".into())
}

/// Creates a runnable GPUI project at `root`, in the shape `template` asks for.
pub fn create_project(root: &Path, name: &str, template: Template) -> io::Result<()> {
    // Never write over an existing crate: `src/ui/mod.rs` and `src/main.rs`
    // would go with it.
    if root.join("Cargo.toml").exists() {
        return Err(io::Error::new(
            io::ErrorKind::AlreadyExists,
            format!("{} already holds a Cargo.toml", root.display()),
        ));
    }
    std::fs::create_dir_all(root.join("src/ui"))?;
    std::fs::create_dir_all(root.join(".cargo"))?;
    std::fs::write(root.join("Cargo.toml"), cargo_toml(&crate_name(name)))?;
    std::fs::write(root.join(".cargo/config.toml"), cargo_config())?;
    // `maxx.toml` carries the project, so it starts with the project: the view
    // the window opens on. What was copied from maxx joins it later, module by
    // module. Versioned with the project, both halves.
    std::fs::write(root.join(".gitignore"), "/target\n/.cargo\n")?;
    // A shape with a shell draws its own title bar; one without keeps the
    // system's, having neither anything to put in a bar of its own nor anything
    // that would draw it.
    std::fs::write(root.join("src/main.rs"), main_rs(template))?;
    std::fs::write(root.join("src/menus.rs"), menus_rs())?;
    std::fs::write(root.join("src/ui/mod.rs"), "pub mod home;\n")?;
    // The utility shape has one view and it is this one, so it is written here
    // rather than written twice: a shape that has no shell has no second place
    // to put its page, and `main.rs` already opens on `home`.
    let home = match template {
        Template::Utility => page_source("home"),
        _ => view_rs("Home", "home"),
    };
    std::fs::write(root.join("src/ui/home.rs"), home)?;
    crate::projectfile::set_entry(root, "home")?;

    const HOME: (&str, &str, &str) = ("home", "Home", "Home");
    match template {
        Template::Empty => Ok(()),
        // No shell, so nothing to point the window at that it is not already
        // pointing at — but the record is written the way every other shape
        // writes it, through the one function that keeps `main.rs` and
        // `maxx.toml` saying the same thing.
        Template::Utility => set_entry_view(root, "home"),
        Template::Sidebar => add_shell(root, &[HOME, ("library", "Library", "Library")]),
        Template::Settings => {
            // The module first: the screen is written against it, and a screen
            // whose module failed to arrive would not compile.
            add_settings_module(root)?;
            std::fs::write(
                root.join("src/ui/settings_screen.rs"),
                crate::scaffold::settings_screen_rs(),
            )?;
            declare_ui_module(root, "settings_screen")?;
            add_shell(root, &[HOME, ("settings_screen", "SettingsScreen", "Settings")])
        }
        // The page the shape is named after comes first, so the window opens on
        // what the shape is for rather than on the blank view beside it. `home`
        // stays, and it is where the next screen goes.
        Template::ListDetail => {
            add_page(root, "items")?;
            add_shell(root, &[("items", "Items", "Items"), HOME])
        }
        Template::Form => {
            add_page(root, "form")?;
            add_shell(root, &[("form", "Form", "Form"), HOME])
        }
        Template::Dashboard => {
            add_page(root, "dashboard")?;
            add_shell(root, &[("dashboard", "Dashboard", "Dashboard"), HOME])
        }
        Template::Wizard => {
            add_page(root, "wizard")?;
            add_shell(root, &[("wizard", "Wizard", "Wizard"), HOME])
        }
        Template::Editor => {
            add_page(root, "editor")?;
            add_shell(root, &[("editor", "Editor", "Editor"), HOME])
        }
    }
}

/// Writes the page a shape brings for `module`, and declares it.
///
/// A page is a view, markers and all — so what lands here is designed on from
/// the first minute, unlike `shell.rs` and the settings screen, which maxx
/// writes once and never opens again.
fn add_page(root: &Path, module: &str) -> io::Result<()> {
    std::fs::write(root.join(format!("src/ui/{module}.rs")), page_source(module))?;
    declare_ui_module(root, module)
}

/// The source of the page a shape writes to `module`.
///
/// Read out of the one table `build.rs` compiles from, so a page that is
/// written into a project is a page the build has already compiled.
fn page_source(module: &str) -> String {
    templates::SHAPE_PAGES
        .iter()
        .find(|(name, _)| *name == module)
        .map(|(_, source)| source())
        .expect("every page a shape writes is in SHAPE_PAGES")
}

/// Gives the project a shell: a sidebar and the pages it switches between.
///
/// `pages` is the whole list, in the order the sidebar shows them, and the
/// first is the one the window comes up on — which is why a shape that has
/// something to show puts it before `home`.
///
/// The window then opens on the shell rather than on a view — which is a fact
/// about the project, so `set_entry_view` records it in `maxx.toml` on the way
/// through.
fn add_shell(root: &Path, pages: &[(&str, &str, &str)]) -> io::Result<()> {
    for (module, type_name, _) in pages {
        // A page maxx designs is created as a view; one the shape brought whole
        // is already on disk.
        if !root.join(format!("src/ui/{module}.rs")).exists() {
            std::fs::write(root.join(format!("src/ui/{module}.rs")), view_rs(type_name, module))?;
            declare_ui_module(root, module)?;
        }
    }

    std::fs::write(root.join("src/ui/shell.rs"), crate::scaffold::shell_rs(pages))?;
    declare_ui_module(root, "shell")?;
    set_entry_view(root, "shell")
}

fn cargo_toml(name: &str) -> String {
    format!(
        r#"[package]
name = "{name}"
version = "0.1.0"
edition = "2024"

[dependencies]
# `runtime_shaders` compiles the Metal shaders at startup instead of at build
# time. Xcode 26 ships the Metal toolchain as a separate downloadable
# component, and without this feature the build fails on a missing `metal`
# tool. Remove it only once that component is installed.
gpui = {{ version = "0.2.2", features = ["runtime_shaders"] }}
gpui-component = "0.5.1"

[profile.dev.package."*"]
opt-level = 2
"#
    )
}

/// Points the project at the cache every maxx project shares.
///
/// The path is absolute, so it is machine-local — hence the `.gitignore` entry.
/// Losing it costs a rebuild, nothing more.
fn cargo_config() -> String {
    format!(
        r#"# Written by maxx. Every maxx project builds into the same directory: gpui
# and gpui-component are about 750 crates, and a project with a `target/` of
# its own rebuilds all of them. This file is local to this machine, hence its
# entry in .gitignore.
[build]
target-dir = "{}"
"#,
        // A basic TOML string treats `\` as an escape, and `C:\Users\…` holds
        // no valid one: the file becomes unreadable and `cargo` refuses to
        // start before it even compiles.
        crate::run::shared_target_dir()
            .display()
            .to_string()
            .replace('\\', "\\\\")
            .replace('"', "\\\"")
    )
}

/// Turns a folder name into a name cargo accepts: lowercase, `_` for anything
/// that is not alphanumeric, and never starting with a digit.
pub fn crate_name(name: &str) -> String {
    let mut out: String =
        name.chars()
            .map(|character| {
                if character.is_ascii_alphanumeric() { character.to_ascii_lowercase() } else { '_' }
            })
            .collect();
    if out.is_empty() || out.starts_with(|character: char| character.is_ascii_digit()) {
        out.insert(0, '_');
    }
    out
}

/// `home` becomes `Home`, `my_screen` becomes `MyScreen`.
pub fn to_type_name(module: &str) -> String {
    module
        .split('_')
        .filter(|part| !part.is_empty())
        .map(|part| {
            let mut chars = part.chars();
            match chars.next() {
                Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
                None => String::new(),
            }
        })
        .collect()
}

/// `name` when it can be a Rust binding, nothing otherwise.
fn identifier(name: &str) -> Option<String> {
    let valid = !name.is_empty()
        && name.chars().all(|character| character.is_alphanumeric() || character == '_')
        && !name.starts_with(|character: char| character.is_ascii_digit());
    valid.then(|| name.to_string())
}

/// The entry point of the generated project.
///
/// Two shape decisions are taken here and nowhere else: the size of the window,
/// and whether it carries a title bar of its own. The second has to be taken
/// here and not in the shell: `TitleBar::title_bar_options()` is what makes the
/// system bar transparent, and `gpui_component::TitleBar`, which only
/// `shell.rs` draws, is what goes in its place. Either one without the other is
/// visible at a glance — a doubled bar, or a bare strip where the traffic
/// lights sit — which is why the question is `has_shell` and not a flag of its
/// own.
fn main_rs(template: Template) -> String {
    let (width, height) = template.window_size();
    let (import, option) = if template.has_shell() {
        (
            "use gpui_component::{Root, TitleBar};\n\n",
            "                titlebar: Some(TitleBar::title_bar_options()),\n",
        )
    } else {
        ("use gpui_component::Root;\n\n", "")
    };

    let body = r#"mod menus;
mod ui;

use gpui::{App, Application, Bounds, WindowBounds, WindowOptions, prelude::*, px, size};
IMPORT
use crate::ui::home::Home;

fn main() {
    Application::new().run(|cx: &mut App| {
        gpui_component::init(cx);
        cx.activate(true);

        menus::register(cx);
        cx.bind_keys(menus::key_bindings());
        cx.set_menus(menus::app_menus());

        let bounds = Bounds::centered(None, size(px(WIDTH.), px(HEIGHT.)), cx);
        cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
OPTION                ..Default::default()
            },
            |window, cx| {
                let view = cx.new(|cx| Home::new(window, cx));
                cx.new(|cx| Root::new(view, window, cx))
            },
        )
        .expect("the window must open");
    });
}
"#;

    body.replace("IMPORT\n", import)
        .replace("OPTION", option)
        .replace("WIDTH", &width.to_string())
        .replace("HEIGHT", &height.to_string())
}
