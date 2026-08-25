//! Project and view templates.
//!
//! Everything written here is ordinary Rust that compiles and runs without
//! `maxx`. The only trace `maxx` leaves is a pair of marker comments around the
//! expression it owns.

use std::io;
use std::path::Path;

/// Creates a runnable GPUI project at `root`.
pub fn create_project(root: &Path, name: &str) -> io::Result<()> {
    // Never write over an existing crate: `src/ui/mod.rs` and `src/main.rs`
    // would go with it.
    if root.join("Cargo.toml").exists() {
        return Err(io::Error::new(
            io::ErrorKind::AlreadyExists,
            format!("{} contient déjà un Cargo.toml", root.display()),
        ));
    }
    std::fs::create_dir_all(root.join("src/ui"))?;
    std::fs::create_dir_all(root.join(".cargo"))?;
    std::fs::write(root.join("Cargo.toml"), cargo_toml(&crate_name(name)))?;
    std::fs::write(root.join(".cargo/config.toml"), cargo_config())?;
    std::fs::write(root.join(".gitignore"), "/target\n/.cargo\n")?;
    std::fs::write(root.join("src/main.rs"), main_rs())?;
    std::fs::write(root.join("src/menus.rs"), menus_rs())?;
    std::fs::write(root.join("src/ui/mod.rs"), "pub mod accueil;\n")?;
    std::fs::write(root.join("src/ui/accueil.rs"), view_rs("Accueil"))?;
    Ok(())
}

/// Adds a view to an existing project and registers it in `src/ui/mod.rs`.
pub fn create_view(root: &Path, module: &str) -> io::Result<()> {
    let type_name = to_type_name(module);
    let file = root.join(format!("src/ui/{module}.rs"));
    if file.exists() {
        return Err(io::Error::new(
            io::ErrorKind::AlreadyExists,
            format!("{} existe déjà", file.display()),
        ));
    }
    std::fs::write(&file, view_rs(&type_name))?;

    // Registered by textual insertion so the rest of `mod.rs` — comments,
    // ordering, anything the developer put there — is untouched.
    let mod_path = root.join("src/ui/mod.rs");
    let mut source = std::fs::read_to_string(&mod_path).unwrap_or_default();
    let line = format!("pub mod {module};\n");
    if !source.contains(&line) {
        if !source.is_empty() && !source.ends_with('\n') {
            source.push('\n');
        }
        source.push_str(&line);
        std::fs::write(&mod_path, source)?;
    }
    Ok(())
}

/// Turns a folder name into a name cargo accepts: lowercase, `_` for anything
/// that is not alphanumeric, and never starting with a digit.
pub fn crate_name(name: &str) -> String {
    let mut out: String = name
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() {
                character.to_ascii_lowercase()
            } else {
                '_'
            }
        })
        .collect();
    if out.is_empty() || out.starts_with(|character: char| character.is_ascii_digit()) {
        out.insert(0, '_');
    }
    out
}

/// `accueil` becomes `Accueil`, `mon_ecran` becomes `MonEcran`.
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
        r#"# Écrit par maxx. Tous les projets maxx compilent dans le même
# répertoire : gpui et gpui-component représentent environ 750 crates, et un
# projet qui a son propre `target/` les recompile intégralement. Ce fichier est
# propre à cette machine, d'où son entrée dans .gitignore.
[build]
target-dir = "{}"
"#,
        crate::run::shared_target_dir().display()
    )
}

/// The menu bar of a generated project.
///
/// A GPUI application gets no menu bar of its own — not even a Quit — unless it
/// calls `set_menus`, so the template ships a usable one and maxx edits it.
fn menus_rs() -> String {
    r#"use gpui::{
    App, Bounds, Context, Menu, MenuItem, OsAction, SharedString, TitlebarOptions, Window,
    WindowBounds, WindowOptions, actions, div, prelude::*, px, rgb, size,
};

actions!(app, [About, Quit, HideApp, HideOthers, ShowAll, Undo, Redo, Cut, Copy, Paste, SelectAll, Minimize]);

/// Wires what the menu entries do.
pub fn register(cx: &mut App) {
    cx.on_action(|_: &Quit, cx: &mut App| cx.quit());
    cx.on_action(|_: &HideApp, cx: &mut App| cx.hide());
    cx.on_action(|_: &HideOthers, cx: &mut App| cx.hide_other_apps());
    cx.on_action(|_: &ShowAll, cx: &mut App| cx.unhide_other_apps());
    cx.on_action(|_: &About, cx: &mut App| open_about(cx));
    cx.on_action(|_: &Minimize, cx: &mut App| {
        // Deferred: an action handler runs inside the window's own update, and
        // gpui refuses to enter a second one.
        cx.defer(|cx| {
            if let Some(handle) = cx.active_window() {
                let _ = handle.update(cx, |_, window, _| window.minimize_window());
            }
        });
    });
    // maxx:handlers
}

/// The shortcuts the menu entries display.
pub fn key_bindings() -> Vec<gpui::KeyBinding> {
    use gpui::KeyBinding;
    vec![
        KeyBinding::new("cmd-q", Quit, None),
        KeyBinding::new("cmd-h", HideApp, None),
        KeyBinding::new("cmd-alt-h", HideOthers, None),
        KeyBinding::new("cmd-z", Undo, None),
        KeyBinding::new("cmd-shift-z", Redo, None),
        KeyBinding::new("cmd-x", Cut, None),
        KeyBinding::new("cmd-c", Copy, None),
        KeyBinding::new("cmd-v", Paste, None),
        KeyBinding::new("cmd-a", SelectAll, None),
        KeyBinding::new("cmd-m", Minimize, None),
    ]
}

/// What the About window shows.
///
/// Name and version are read from Cargo.toml at build time: `[package]` is the
/// one place a version number should live, and `cargo set-version` or a hand
/// edit there is enough to change what this window says.
struct AboutWindow {
    name: SharedString,
    version: SharedString,
}

impl Render for AboutWindow {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .flex()
            .flex_col()
            .items_center()
            .justify_center()
            .gap_2()
            .size_full()
            .bg(rgb(0x1e2127))
            .text_color(rgb(0xc8ccd4))
            .child(div().text_2xl().child(self.name.clone()))
            .child(
                div()
                    .text_color(rgb(0x7f8896))
                    .child(format!("version {}", self.version)),
            )
    }
}

/// Opens the About window, or brings it forward when it is already up.
///
/// Plain gpui, no `gpui_component`: a window drawing a component widget has to
/// be rooted in `gpui_component::Root`, and this one does not need it.
///
/// Deferred for the same reason as Minimize above: an action handler runs
/// inside the update of a window, and gpui refuses to enter a second one.
fn open_about(cx: &mut App) {
    cx.defer(open_about_now);
}

fn open_about_now(cx: &mut App) {
    if let Some(existing) = cx
        .windows()
        .into_iter()
        .find(|handle| handle.downcast::<AboutWindow>().is_some())
    {
        let _ = existing.update(cx, |_, window, _| window.activate_window());
        return;
    }

    let bounds = Bounds::centered(None, size(px(320.), px(180.)), cx);
    let options = WindowOptions {
        window_bounds: Some(WindowBounds::Windowed(bounds)),
        titlebar: Some(TitlebarOptions {
            title: Some(SharedString::from("À propos")),
            ..Default::default()
        }),
        is_resizable: false,
        is_minimizable: false,
        ..Default::default()
    };

    cx.open_window(options, |_window, cx| {
        cx.new(|_| AboutWindow {
            name: SharedString::from(env!("CARGO_PKG_NAME")),
            version: SharedString::from(env!("CARGO_PKG_VERSION")),
        })
    })
    .ok();
}

/// The menu bar itself.
pub fn app_menus() -> Vec<Menu> {
    // maxx:begin
    vec![
        Menu {
            name: "app".into(),
            items: vec![
                MenuItem::action("À propos", About),
                MenuItem::separator(),
                MenuItem::action("Masquer", HideApp),
                MenuItem::action("Masquer les autres", HideOthers),
                MenuItem::action("Tout afficher", ShowAll),
                MenuItem::separator(),
                MenuItem::action("Quitter", Quit),
            ],
        },
        Menu {
            name: "Édition".into(),
            items: vec![
                MenuItem::os_action("Annuler", Undo, OsAction::Undo),
                MenuItem::os_action("Rétablir", Redo, OsAction::Redo),
                MenuItem::separator(),
                MenuItem::os_action("Couper", Cut, OsAction::Cut),
                MenuItem::os_action("Copier", Copy, OsAction::Copy),
                MenuItem::os_action("Coller", Paste, OsAction::Paste),
                MenuItem::os_action("Tout sélectionner", SelectAll, OsAction::SelectAll),
            ],
        },
        Menu {
            name: "Fenêtre".into(),
            items: vec![MenuItem::action("Réduire", Minimize)],
        },
    ]
    // maxx:end
}
"#
    .to_string()
}

fn main_rs() -> String {
    r#"mod menus;
mod ui;

use gpui::{
    App, Application, Bounds, WindowBounds, WindowOptions, prelude::*, px, size,
};
use gpui_component::Root;

use crate::ui::accueil::Accueil;

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
                ..Default::default()
            },
            |window, cx| {
                let view = cx.new(|cx| Accueil::new(window, cx));
                cx.new(|cx| Root::new(view, window, cx))
            },
        )
        .expect("la fenêtre doit s'ouvrir");
    });
}
"#
    .to_string()
}

fn view_rs(type_name: &str) -> String {
    format!(
        r#"use gpui::{{Context, Window, prelude::*}};
use gpui_component::label::Label;
use gpui_component::v_flex;

pub struct {type_name} {{}}

impl {type_name} {{
    pub fn new(_window: &mut Window, _cx: &mut Context<Self>) -> Self {{
        Self {{}}
    }}
}}

impl Render for {type_name} {{
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {{
        // maxx:begin
        v_flex()
            .gap_2()
            .p_4()
            .child(Label::new("Bienvenue"))
        // maxx:end
    }}
}}
"#
    )
}

/// Gives an existing project a menu bar: writes `src/menus.rs` and wires it
/// into `src/main.rs`.
///
/// Wired by textual insertion, like `create_view`: the project may predate the
/// template entirely, and rewriting its `main.rs` from the template would throw
/// away whatever it does at startup.
pub fn add_menu_bar(root: &Path) -> io::Result<()> {
    // `main.rs` is patched first, and nothing is written until it is known to
    // work: a `src/menus.rs` left behind by a failed wiring would make the next
    // attempt believe the project already has a menu bar.
    let main_path = root.join("src/main.rs");
    let source = std::fs::read_to_string(&main_path)?;
    let mut lines: Vec<String> = source.lines().map(str::to_string).collect();

    if !lines.iter().any(|line| line.trim() == "mod menus;") {
        // Not line 0: an inner doc comment or an inner attribute has to stay
        // ahead of every item, or the crate stops compiling.
        let after_header = lines
            .iter()
            .position(|line| {
                let line = line.trim_start();
                !(line.is_empty() || line.starts_with("//") || line.starts_with("#!["))
            })
            .unwrap_or(lines.len());
        lines.insert(after_header, "mod menus;".into());
    }

    if !source.contains("menus::app_menus()") {
        // `cx.activate` is what every gpui `main` does first; failing that, the
        // line that opens the closure `run` was given.
        let anchor = lines
            .iter()
            .position(|line| line.contains(".activate("))
            .or_else(|| {
                lines
                    .iter()
                    .position(|line| line.contains(".run(") && line.trim_end().ends_with('{'))
            });
        let Some(anchor) = anchor else {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "src/main.rs : ni cx.activate(…) ni Application::new().run(…) — \
                 ajoutez menus::register(cx), cx.bind_keys(menus::key_bindings()) \
                 et cx.set_menus(menus::app_menus()) à la main",
            ));
        };

        // The three calls need the name this `main` gave its application. Both
        // anchors carry it, in different places: `cx.activate(true)` names it
        // as the receiver, `run(|app| {` as the closure's argument. Assuming
        // `cx` would hand a project written as `run(|app| {` three lines naming
        // something that does not exist.
        let Some(app) = application_binding(&lines[anchor]) else {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!(
                    "src/main.rs : impossible de lire le nom de l'application dans « {} » — \
                     ajoutez menus::register(…), …bind_keys(menus::key_bindings()) \
                     et …set_menus(menus::app_menus()) à la main",
                    lines[anchor].trim()
                ),
            ));
        };

        let indent: String = lines[anchor]
            .chars()
            .take_while(|character| character.is_whitespace())
            .collect();
        for (offset, call) in [
            format!("menus::register({app});"),
            format!("{app}.bind_keys(menus::key_bindings());"),
            format!("{app}.set_menus(menus::app_menus());"),
        ]
        .iter()
        .enumerate()
        {
            lines.insert(anchor + 1 + offset, format!("{indent}{call}"));
        }
    }

    let menus_path = root.join("src/menus.rs");
    let created = !menus_path.exists();
    if created {
        std::fs::write(&menus_path, menus_rs())?;
    }

    let mut out = lines.join("\n");
    out.push('\n');
    if let Err(error) = std::fs::write(&main_path, out) {
        // A `menus.rs` left behind by a failed wiring would make the next
        // attempt believe the project already has a menu bar, and skip the
        // wiring for good.
        if created {
            let _ = std::fs::remove_file(&menus_path);
        }
        return Err(error);
    }
    Ok(())
}

/// Whether `line` is one of the three calls `add_menu_bar` writes, or the
/// module declaration that goes with them.
fn is_menu_wiring(line: &str) -> bool {
    if line == "mod menus;" || line == "menus::register" {
        return true;
    }
    if let Some(argument) = line
        .strip_prefix("menus::register(")
        .and_then(|rest| rest.strip_suffix(");"))
    {
        return identifier(argument).is_some();
    }
    for call in [
        ".bind_keys(menus::key_bindings());",
        ".set_menus(menus::app_menus());",
    ] {
        if let Some(receiver) = line.strip_suffix(call)
            && identifier(receiver).is_some()
        {
            return true;
        }
    }
    false
}

/// The name `line` gives the application.
///
/// Either the receiver of `.activate(`, or the argument of the closure handed
/// to `run` — `|cx|` as much as `|app: &mut App|`.
fn application_binding(line: &str) -> Option<String> {
    if let Some(dot) = line.find(".activate(") {
        let receiver: String = line[..dot]
            .chars()
            .rev()
            .take_while(|character| character.is_alphanumeric() || *character == '_')
            .collect();
        return identifier(&receiver.chars().rev().collect::<String>());
    }

    let start = line.find('|')?;
    let rest = &line[start + 1..];
    let end = rest.find('|')?;
    identifier(rest[..end].split(':').next()?.trim())
}

/// `name` when it can be a Rust binding, nothing otherwise.
fn identifier(name: &str) -> Option<String> {
    let valid = !name.is_empty()
        && name
            .chars()
            .all(|character| character.is_alphanumeric() || character == '_')
        && !name.starts_with(|character: char| character.is_ascii_digit());
    valid.then(|| name.to_string())
}

/// Unwires the menu bar from `src/main.rs`.
///
/// The file `src/menus.rs` is the caller's business — the project panel puts it
/// in the Trash — but leaving `mod menus;` behind would stop the project from
/// compiling.
pub fn remove_menu_bar(root: &Path) -> io::Result<()> {
    let main_path = root.join("src/main.rs");
    let source = std::fs::read_to_string(&main_path)?;
    // Matched on shape, not on the exact text: `add_menu_bar` writes these
    // with the name the project gave its application, which is `cx` in the
    // template and anything at all in a hand-written `main.rs`. Filtering
    // literal `cx` lines would leave a call to a module that no longer exists.
    let kept: Vec<&str> = source
        .lines()
        .filter(|line| !is_menu_wiring(line.trim()))
        .collect();
    let mut out = kept.join("\n");
    out.push('\n');
    std::fs::write(&main_path, out)
}
