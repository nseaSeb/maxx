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
pub mod palette;
pub mod parser;
pub mod preferences;
pub mod project;
pub mod projectfile;
pub mod registry;
pub mod run;
pub mod scaffold;
pub mod settings;
pub mod theme;
pub mod tools;
pub mod view;
pub mod workspace;

use gpui::{App, Application, SharedString};

rust_i18n::i18n!("locales", fallback = "en");

/// The translation of `key` in the current language.
///
/// `t!` answers a `Cow`, which almost nothing in gpui accepts; this hands back
/// what an element takes as a child. English is the fallback, so a key missing
/// from a translation shows its English text rather than its key.
pub fn tr(key: &str) -> SharedString {
    SharedString::from(rust_i18n::t!(key).into_owned())
}

/// The language maxx speaks, from the preferences.
///
/// `System` reads the locale the system reports and keeps only its language:
/// `fr-CA` and `fr-FR` are one language as far as maxx's few hundred strings
/// are concerned. Anything maxx does not translate lands on English.
pub fn apply_locale(cx: &App) {
    let chosen = settings::prefs(cx).language.clone();
    let locale = match chosen.as_str() {
        "system" => sys_locale(),
        other => other.to_string(),
    };
    let locale = if rust_i18n::available_locales!().contains(&locale.as_str()) {
        locale
    } else {
        "en".to_string()
    };
    // The same global on both sides: gpui-component's own strings follow.
    gpui_component::set_locale(&locale);
}

/// The palette maxx draws with, from the preferences.
///
/// One switch for two things: maxx's own chrome, painted from [`crate::theme`],
/// and `gpui_component`'s widgets, which carry their own theme. They have to
/// agree — half the window in one mode is worse than either mode.
pub fn apply_theme(cx: &mut App) {
    let chosen = settings::prefs(cx).theme.clone();
    let dark = match chosen.as_str() {
        "light" => false,
        "dark" => true,
        // `system`, and anything a hand-written file put there.
        _ => matches!(
            cx.window_appearance(),
            gpui::WindowAppearance::Dark | gpui::WindowAppearance::VibrantDark
        ),
    };
    theme::set_dark(dark);
    let mode =
        if dark { gpui_component::ThemeMode::Dark } else { gpui_component::ThemeMode::Light };
    gpui_component::Theme::change(mode, None, cx);
}

/// The language the system reports, from the usual environment variables.
///
/// No dependency for this: `LANG=fr_FR.UTF-8` is the whole of what has to be
/// read, and on macOS gpui has already put the user's locale in the
/// environment by the time this runs.
fn sys_locale() -> String {
    for name in ["LC_ALL", "LC_MESSAGES", "LANG"] {
        let Ok(value) = std::env::var(name) else {
            continue;
        };
        let language = value.split(['.', '_', '-']).next().unwrap_or("").to_lowercase();
        if !language.is_empty() && language != "c" && language != "posix" {
            return language;
        }
    }
    "en".to_string()
}

/// Boots the application: actions, keymap, menus, first window.
pub fn run() {
    Application::new().run(|cx: &mut App| {
        // Without this the menu bar stays behind whatever was frontmost when
        // the app was launched from a terminal.
        gpui_component::init(cx);
        cx.activate(true);

        settings::init(cx);
        // Before the menus and the first window: both are built from strings
        // that have to be in the right language already.
        apply_locale(cx);
        apply_theme(cx);
        // The window geometry is only staged in memory as it moves; this is
        // where it is written. `detach` because the subscription has to outlive
        // this closure, and the application ends right after it fires.
        cx.on_app_quit(|cx: &mut App| {
            settings::flush(cx);
            async {}
        })
        .detach();
        actions::register_handlers(cx);
        cx.bind_keys(actions::key_bindings());
        cx.set_menus(menus::app_menus(cx));

        // `maxx <path>` opens a project straight away, the way `zed <path>` does.
        let path =
            std::env::args().nth(1).map(std::path::PathBuf::from).filter(|path| path.is_dir());
        workspace::open_workspace_window(path, cx);
    });
}
