mod assets;
mod menus;
mod system;
mod theme;
mod ui;
mod window;

use gpui::{App, Application, Bounds, WindowBounds, WindowOptions, prelude::*, px, size};
use gpui_component::Root;

use crate::ui::home::Home;

fn main() {
    Application::new()
        .with_assets(assets::Assets)
        .run(|cx: &mut App| {
            gpui_component::init(cx);
            cx.activate(true);
            // Without this the application opens in light mode whatever the system
            // says, and that is the first thing everybody notices.
            theme::follow_system(None, cx);
            menus::register(cx);
            cx.bind_keys(menus::key_bindings());
            cx.set_menus(menus::app_menus());

            let bounds = Bounds::centered(None, size(px(900.), px(600.)), cx);
            let bounds = window::bounds(bounds);
            cx.open_window(
                WindowOptions {
                    window_bounds: Some(WindowBounds::Windowed(bounds)),
                    ..Default::default()
                },
                |window, cx| {
                    window::remember(&window, cx);
                    let view = cx.new(|cx| Home::new(window, cx));
                    cx.new(|cx| Root::new(view, window, cx))
                },
            )
            .expect("the window must open");
        });
}
