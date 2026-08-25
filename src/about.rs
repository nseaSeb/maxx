//! The About window.
//!
//! Its own window rather than a panel inside the workspace: that is where macOS
//! users look for it, and it must stay reachable when no project is open.
//!
//! Everything it draws is plain `gpui`. A window that renders a
//! `gpui-component` widget has to be rooted in `gpui_component::Root`, and the
//! About box is not worth that.

use gpui::prelude::*;
use gpui::{
    App, Bounds, Context, SharedString, TitlebarOptions, Window, WindowBounds, WindowOptions, div,
    point, px, rgb, size,
};

use crate::theme;

/// The version of `gpui` this build links against.
///
/// Read off `Cargo.toml` at build time rather than written twice: the file is
/// the one place the version already lives.
pub const GPUI_VERSION: &str = env!("MAXX_GPUI_VERSION");

/// What the window shows.
struct About {
    name: SharedString,
    version: SharedString,
    gpui: SharedString,
}

impl Render for About {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .flex()
            .flex_col()
            .items_center()
            .justify_center()
            .gap_2()
            .size_full()
            .bg(rgb(theme::BG))
            .text_color(rgb(theme::TEXT))
            .child(div().text_2xl().child(self.name.clone()))
            .child(
                div().text_color(rgb(theme::TEXT_MUTED)).child(format!("version {}", self.version)),
            )
            .child(
                div()
                    .text_xs()
                    .text_color(rgb(theme::TEXT_MUTED))
                    .child(format!("GPUI {}", self.gpui)),
            )
            .child(
                div()
                    .text_xs()
                    .text_color(rgb(theme::TEXT_MUTED))
                    .child(crate::tr("about.licence")),
            )
    }
}

/// Opens the About window, or brings it forward when it is already up.
///
/// Deferred: an action handler runs inside the update of the window it was
/// dispatched from, and gpui refuses to enter a second one — opening or
/// activating a window from there does nothing at all, without an error.
pub fn open(cx: &mut App) {
    cx.defer(open_now);
}

fn open_now(cx: &mut App) {
    // A second About window would be a bug you can click twice.
    if let Some(existing) =
        cx.windows().into_iter().find(|handle| handle.downcast::<About>().is_some())
    {
        let _ = existing.update(cx, |_, window, _| window.activate_window());
        return;
    }

    let bounds = Bounds::centered(None, size(px(360.), px(220.)), cx);
    let options = WindowOptions {
        window_bounds: Some(WindowBounds::Windowed(bounds)),
        titlebar: Some(TitlebarOptions {
            title: Some(crate::tr("about.title")),
            appears_transparent: true,
            traffic_light_position: Some(point(px(9.), px(9.))),
        }),
        is_resizable: false,
        is_minimizable: false,
        ..Default::default()
    };

    cx.open_window(options, |_window, cx| {
        cx.new(|_| About {
            name: SharedString::from(env!("CARGO_PKG_NAME")),
            version: SharedString::from(env!("CARGO_PKG_VERSION")),
            gpui: SharedString::from(GPUI_VERSION),
        })
    })
    .ok();
}
