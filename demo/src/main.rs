mod menus;
mod theme;
mod ui;

use gpui::{App, Application, Bounds, WindowBounds, WindowOptions, prelude::*, px, size};
use gpui_component::Root;

use crate::ui::home::Home;

fn main() {
    Application::new().run(|cx: &mut App| {
        gpui_component::init(cx);
        cx.activate(true);
        // Sans cela l'application s'ouvre en clair quoi que dise le système,
        // et c'est la première chose que tout le monde remarque.
        theme::follow_system(None, cx);
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
                let view = cx.new(|cx| Home::new(window, cx));
                cx.new(|cx| Root::new(view, window, cx))
            },
        )
        .expect("the window must open");
    });
}
