//! maxx — a visual workshop that builds GPUI views and writes them out as real
//! Rust source.

pub mod about;
pub mod actions;
pub mod codegen;
pub mod designer;
pub mod menu_model;
pub mod menufile;
pub mod menus;
pub mod model;
pub mod parser;
pub mod project;
pub mod run;
pub mod registry;
pub mod scaffold;
pub mod view;
pub mod theme;
pub mod workspace;

use gpui::{App, Application};

/// Boots the application: actions, keymap, menus, first window.
pub fn run() {
    Application::new().run(|cx: &mut App| {
        // Without this the menu bar stays behind whatever was frontmost when
        // the app was launched from a terminal.
        gpui_component::init(cx);
        cx.activate(true);

        actions::register_handlers(cx);
        cx.bind_keys(actions::key_bindings());
        cx.set_menus(menus::app_menus());

        // `maxx <chemin>` ouvre directement un projet, comme `zed <chemin>`.
        let path = std::env::args()
            .nth(1)
            .map(std::path::PathBuf::from)
            .filter(|path| path.is_dir());
        workspace::open_workspace_window(path, cx);
    });
}
