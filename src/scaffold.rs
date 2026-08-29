//! Project and view templates.
//!
//! Everything written here is ordinary Rust that compiles and runs without
//! `maxx`. The only trace `maxx` leaves is a pair of marker comments around the
//! expression it owns.

use std::io;
use std::path::Path;

use menubar::menus_rs;
use views::declare_ui_module;

pub mod assets;
pub mod menubar;
pub mod modules;
pub mod settings;
pub mod system;
pub mod templates;
pub mod theme;
pub mod views;
pub mod window;

pub use assets::add_assets_module;
pub use assets::{IMAGE_DIRECTORY, import_asset};
pub use menubar::{add_menu_bar, remove_menu_bar};
pub use modules::{
    MODULES, module_body, module_version, outdated_modules, remove_module, update_module,
};
pub use settings::add_settings_module;
pub use system::add_system_module;
pub use templates::{settings_screen_rs, shell_rs};
pub use theme::add_theme_module;
pub use views::{create_view, rename_view, set_entry_view, view_rs};
pub use window::add_window_module;

/// The shape a new project is given.
///
/// Not screens to pick from a gallery: the three answer the question a desktop
/// project asks on its first day, which is *what holds what*. A sidebar and a
/// settings screen arrive here rather than in the component palette for that
/// reason — they are not elements to drop on a canvas, they are what a canvas
/// hangs from.
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
}

impl Template {
    /// The name this shape carries in `maxx.toml` and in the tests.
    pub fn name(self) -> &'static str {
        match self {
            Self::Empty => "empty",
            Self::Sidebar => "sidebar",
            Self::Settings => "settings",
        }
    }
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
    // A shape with a shell draws its own title bar; the empty one keeps the
    // system's, having nothing to put in a bar of its own.
    std::fs::write(root.join("src/main.rs"), main_rs(template != Template::Empty))?;
    std::fs::write(root.join("src/menus.rs"), menus_rs())?;
    std::fs::write(root.join("src/ui/mod.rs"), "pub mod home;\n")?;
    std::fs::write(root.join("src/ui/home.rs"), view_rs("Home", "home"))?;
    crate::projectfile::set_entry(root, "home")?;

    match template {
        Template::Empty => Ok(()),
        Template::Sidebar => add_shell(root, &[("library", "Library", "Library")]),
        Template::Settings => {
            // The module first: the screen is written against it, and a screen
            // whose module failed to arrive would not compile.
            add_settings_module(root)?;
            std::fs::write(
                root.join("src/ui/settings_screen.rs"),
                crate::scaffold::settings_screen_rs(),
            )?;
            declare_ui_module(root, "settings_screen")?;
            add_shell(root, &[("settings_screen", "SettingsScreen", "Settings")])
        }
    }
}

/// Gives the project a shell: a sidebar, `home`, and whatever `pages` adds.
///
/// The window then opens on the shell rather than on a view — which is a fact
/// about the project, so `set_entry_view` records it in `maxx.toml` on the way
/// through.
fn add_shell(root: &Path, pages: &[(&str, &str, &str)]) -> io::Result<()> {
    for (module, type_name, _) in pages {
        // A page maxx designs is created as a view; one it wrote whole, like
        // the settings screen, is already on disk.
        if !root.join(format!("src/ui/{module}.rs")).exists() {
            std::fs::write(root.join(format!("src/ui/{module}.rs")), view_rs(type_name, module))?;
            declare_ui_module(root, module)?;
        }
    }

    let mut all = vec![("home", "Home", "Home")];
    all.extend_from_slice(pages);
    std::fs::write(root.join("src/ui/shell.rs"), crate::scaffold::shell_rs(&all))?;
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

/// The entry point of the generated project.
///
/// `own_title_bar` says whether the window carries a title bar of its own — a
/// shape decision, taken once at creation. It has to be taken here and not in
/// the shell: `TitleBar::title_bar_options()` is what makes the system bar
/// transparent, and `gpui_component::TitleBar` is what is drawn in its place.
/// Either one without the other is visible at a glance — a doubled bar, or a
/// bare strip where the traffic lights sit.
fn main_rs(own_title_bar: bool) -> String {
    let (import, option) = if own_title_bar {
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

        let bounds = Bounds::centered(None, size(px(900.), px(600.)), cx);
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

    body.replace("IMPORT\n", import).replace("OPTION", option)
}
