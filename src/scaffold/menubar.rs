//! The menu bar: the application's own menus, and the About window they open.

use std::io;
use std::path::Path;

use super::identifier;
use super::modules::{header_end, joined};
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
        lines.insert(header_end(&lines), "mod menus;".into());
    }

    if !source.contains("menus::app_menus()") {
        // `cx.activate` is what every gpui `main` does first; failing that, the
        // line that opens the closure `run` was given.
        let anchor = lines.iter().position(|line| line.contains(".activate(")).or_else(|| {
            lines.iter().position(|line| line.contains(".run(") && line.trim_end().ends_with('{'))
        });
        let Some(anchor) = anchor else {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "src/main.rs: neither cx.activate(…) nor Application::new().run(…) — \
                 add menus::register(cx), cx.bind_keys(menus::key_bindings()) \
                 and cx.set_menus(menus::app_menus()) by hand",
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
                    "src/main.rs: cannot read the application's name in “{}” — \
                     add menus::register(…), …bind_keys(menus::key_bindings()) \
                     and …set_menus(menus::app_menus()) by hand",
                    lines[anchor].trim()
                ),
            ));
        };

        let indent: String =
            lines[anchor].chars().take_while(|character| character.is_whitespace()).collect();
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

    if let Err(error) = std::fs::write(&main_path, joined(&lines, &source)) {
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
    if let Some(argument) =
        line.strip_prefix("menus::register(").and_then(|rest| rest.strip_suffix(");"))
    {
        return identifier(argument).is_some();
    }
    for call in [".bind_keys(menus::key_bindings());", ".set_menus(menus::app_menus());"] {
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
    let kept: Vec<String> =
        source.lines().filter(|line| !is_menu_wiring(line.trim())).map(str::to_string).collect();
    std::fs::write(&main_path, joined(&kept, &source))
}

/// The menu bar of a generated project.
///
/// A GPUI application gets no menu bar of its own — not even a Quit — unless it
/// calls `set_menus`, so the template ships a usable one and maxx edits it.
pub(super) fn menus_rs() -> String {
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
            title: Some(SharedString::from("About")),
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
                MenuItem::action("About", About),
                MenuItem::separator(),
                MenuItem::action("Hide", HideApp),
                MenuItem::action("Hide Others", HideOthers),
                MenuItem::action("Show All", ShowAll),
                MenuItem::separator(),
                MenuItem::action("Quit", Quit),
            ],
        },
        Menu {
            name: "Edit".into(),
            items: vec![
                MenuItem::os_action("Undo", Undo, OsAction::Undo),
                MenuItem::os_action("Redo", Redo, OsAction::Redo),
                MenuItem::separator(),
                MenuItem::os_action("Cut", Cut, OsAction::Cut),
                MenuItem::os_action("Copy", Copy, OsAction::Copy),
                MenuItem::os_action("Paste", Paste, OsAction::Paste),
                MenuItem::os_action("Select All", SelectAll, OsAction::SelectAll),
            ],
        },
        Menu {
            name: "Window".into(),
            items: vec![MenuItem::action("Minimize", Minimize)],
        },
    ]
    // maxx:end
}
"#
    .to_string()
}
