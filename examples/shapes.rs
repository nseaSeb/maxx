//! The project shapes, compiled once here.
//!
//! `scaffold::templates` writes a sidebar and a settings screen into a project
//! maxx never builds. A method `gpui-component` does not have, or one it only
//! offers on another type, is a project that stops compiling on a line maxx
//! wrote itself — and the developer finds out, not maxx. The same guard
//! `examples/catalogue.rs` gives the catalogue.
//!
//! The text compiled here is not a copy: `build.rs` calls the very functions
//! the projects get and writes their output into `OUT_DIR`, so the two cannot
//! drift apart.
//!
//! What is written by hand below is the surface the shapes are written
//! against: a view as `scaffold::view_rs` declares it, and the parts of
//! `src/settings.rs` the screen calls. Their shape is the contract; changing
//! one of those templates without changing its stand-in here is what this file
//! would let through.
//!
//! Nothing is run: `main` builds nothing. It is the compiler that answers the
//! question.

// Nothing here is called: the compiler is the only reader this file has.
#![allow(dead_code)]

mod ui {
    pub mod home {
        use gpui::{Context, Window, prelude::*};
        use gpui_component::label::Label;
        use gpui_component::v_flex;

        /// A view as `scaffold::view_rs` writes one.
        pub struct Home {}

        impl Home {
            pub fn new(_window: &mut Window, _cx: &mut Context<Self>) -> Self {
                Self {}
            }
        }

        impl Render for Home {
            fn render(
                &mut self,
                _window: &mut Window,
                _cx: &mut Context<Self>,
            ) -> impl IntoElement {
                v_flex().id("home").size_full().child(Label::new("Welcome"))
            }
        }
    }

    pub mod settings_screen {
        include!(concat!(env!("OUT_DIR"), "/settings_screen.rs"));
    }
}

/// The part of `src/settings.rs` the settings screen calls.
mod settings {
    #[derive(Clone, Debug, Default)]
    pub struct Settings {
        pub dark_theme: bool,
    }

    pub fn load() -> Settings {
        Settings::default()
    }

    pub fn save(_settings: &Settings) -> std::io::Result<()> {
        Ok(())
    }

    pub fn displayable_path() -> String {
        String::new()
    }
}

mod shell {
    include!(concat!(env!("OUT_DIR"), "/shell.rs"));
}

fn main() {}

/// The bodies maxx writes into a handler, compiled where they are written.
///
/// Wrapped in the two parameters a handler stub already carries: what the
/// developer gets is the same text inside a method, and what fails here is a
/// call `gpui-component` no longer offers.
mod boxes {
    use gpui::prelude::*;
    use gpui::{App, Window};

    include!(concat!(env!("OUT_DIR"), "/boxes.rs"));
}

/// The sub-tree templates the palette drops, compiled where they are written.
mod subtrees {
    use gpui::prelude::*;

    include!(concat!(env!("OUT_DIR"), "/subtrees.rs"));
}
