use gpui::{
    App, Bounds, Context, Menu, MenuItem, OsAction, SharedString, TitlebarOptions, Window,
    WindowBounds, WindowOptions, actions, div, prelude::*, px, rgb, size,
};
use gpui_component::Root;

use crate::ui::inspector::Inspector;

actions!(
    app,
    [
        About,
        Quit,
        HideApp,
        HideOthers,
        ShowAll,
        Undo,
        Redo,
        Cut,
        Copy,
        Paste,
        SelectAll,
        Minimize,
        OpenInspector
    ]
);

/// Wires what the menu entries do.
pub fn register(cx: &mut App) {
    cx.on_action(|_: &Quit, cx: &mut App| cx.quit());
    cx.on_action(|_: &HideApp, cx: &mut App| cx.hide());
    cx.on_action(|_: &HideOthers, cx: &mut App| cx.hide_other_apps());
    cx.on_action(|_: &ShowAll, cx: &mut App| cx.unhide_other_apps());
    cx.on_action(|_: &About, cx: &mut App| open_about(cx));
    cx.set_global(Inspectors::default());
    cx.on_action(|_: &OpenInspector, cx: &mut App| open_inspector(cx));
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
        KeyBinding::new("cmd-i", OpenInspector, None),
        KeyBinding::new("cmd-z", Undo, None),
        KeyBinding::new("cmd-shift-z", Redo, None),
        KeyBinding::new("cmd-x", Cut, None),
        KeyBinding::new("cmd-c", Copy, None),
        KeyBinding::new("cmd-v", Paste, None),
        KeyBinding::new("cmd-a", SelectAll, None),
        KeyBinding::new("cmd-m", Minimize, None),
    ]
}

/// Opens the inspector in a window of its own.
///
/// Two things this function exists to show.
///
/// First `cx.defer`: an action handler runs inside the update of the window
/// that dispatched it, and gpui refuses to enter a second one. Opening a window
/// straight from `cx.on_action` does nothing at all — no error, no panic, which
/// is far worse.
///
/// Then `Root`: a window drawing the smallest gpui-component widget has to be
/// rooted in it. Several components walk up to it and abort the process when it
/// is missing.
pub fn open_inspector(cx: &mut App) {
    cx.defer(|cx: &mut App| {
        // One at a time. The window is held in a global: every window of the
        // demo is rooted in `Root`, so telling them apart by their type is
        // impossible.
        let known = cx.global::<Inspectors>().0;
        if let Some(handle) = known
            && cx.windows().contains(&handle)
        {
            let _ = handle.update(cx, |_, window, _| window.activate_window());
            return;
        }

        let bounds = Bounds::centered(None, size(px(520.), px(420.)), cx);
        cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                titlebar: Some(TitlebarOptions {
                    title: Some(SharedString::from("Inspector")),
                    ..Default::default()
                }),
                ..Default::default()
            },
            |window, cx| {
                let view = cx.new(|cx| Inspector::new(window, cx));
                cx.new(|cx| Root::new(view, window, cx))
            },
        )
        .map(|handle| cx.set_global(Inspectors(Some(handle.into()))))
        .ok();
    });
}

/// The inspector's window, while it is open.
#[derive(Default)]
struct Inspectors(Option<gpui::AnyWindowHandle>);

impl gpui::Global for Inspectors {}

/// The About window, in plain gpui.
///
/// No component here, so no need for `Root`: it is the one window of the demo
/// that does without it.
fn open_about(cx: &mut App) {
    cx.defer(|cx: &mut App| {
        let bounds = Bounds::centered(None, size(px(320.), px(180.)), cx);
        cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                titlebar: Some(TitlebarOptions {
                    title: Some(SharedString::from("About")),
                    ..Default::default()
                }),
                is_resizable: false,
                ..Default::default()
            },
            |_window, cx| cx.new(|_| AboutWindow),
        )
        .ok();
    });
}

struct AboutWindow;

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
            .child(div().text_2xl().child(env!("CARGO_PKG_NAME")))
            .child(
                div()
                    .text_color(rgb(0x7f8896))
                    .child(format!("version {}", env!("CARGO_PKG_VERSION"))),
            )
    }
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
            items: vec![
                MenuItem::action("Open the inspector", OpenInspector),
                MenuItem::separator(),
                MenuItem::action("Minimize", Minimize),
            ],
        },
    ]
    // maxx:end
}
