// The project shapes: code maxx writes once, at creation, and never touches
// again.
//
// Apart from the rest of `scaffold` because `build.rs` includes this file
// verbatim to write the same text into `OUT_DIR`, where `examples/shapes.rs`
// compiles it. Nothing in maxx's own build otherwise checks that a call maxx
// writes into a project exists — `font_medium()`, which never existed, lived
// here for a while — and a sidebar is the part of a project maxx writes with
// the least chance of the developer ever reading it before running it.
//
// Hence the rule this file follows: **no `use` of anything in maxx**, and
// nothing but `std`. `build.rs` is a program of its own and would not compile
// otherwise.

/// The shell that holds a title bar, a sidebar and the view it shows.
///
/// `pages` is, per entry: the module under `src/ui/`, the type it declares, and
/// the label the sidebar shows.
///
/// Hand-written Rust in the project, not a view maxx designs: a sidebar is the
/// shape of the project rather than an element to drop on a canvas. That is
/// also why the pages are fixed at creation — adding one afterwards is four
/// lines the compiler names one by one.
pub fn shell_rs(pages: &[(&str, &str, &str)]) -> String {
    let imports: String = pages
        .iter()
        .map(|(module, type_name, _)| format!("use crate::ui::{module}::{type_name};\n"))
        .collect();

    let variants: String =
        pages.iter().map(|(_, type_name, _)| format!("    {type_name},\n")).collect();

    let labels: String = pages
        .iter()
        .map(|(_, type_name, label)| format!("            Self::{type_name} => \"{label}\",\n"))
        .collect();

    let fields: String = pages
        .iter()
        .map(|(module, type_name, _)| format!("    {module}: Entity<{type_name}>,\n"))
        .collect();

    let built: String = pages
        .iter()
        .map(|(module, type_name, _)| {
            format!("            {module}: cx.new(|cx| {type_name}::new(window, cx)),\n")
        })
        .collect();

    // Built as a list and joined, rather than concatenated: only the last item
    // carries the comma that closes the argument, and a file that leaves it out
    // is a file `rustfmt` rewrites — which is exactly what a shape must not be.
    let items: String = pages
        .iter()
        .map(|(_, type_name, _)| {
            format!(
                "                                .child(\n\
                 \x20                                   SidebarMenuItem::new(Page::{type_name}.label())\n\
                 \x20                                       .active(self.page == Page::{type_name})\n\
                 \x20                                       .on_click(cx.listener(|this, _, _window, cx| {{\n\
                 \x20                                           this.show(Page::{type_name}, cx)\n\
                 \x20                                       }})),\n\
                 \x20                               )"
            )
        })
        .collect::<Vec<_>>()
        .join("\n");

    let arms: String = pages
        .iter()
        .map(|(module, type_name, _)| {
            format!(
                "                        Page::{type_name} => \
                 self.{module}.clone().into_any_element(),\n"
            )
        })
        .collect();

    let first = pages.first().map(|(_, type_name, _)| *type_name).unwrap_or("Home");

    format!(
        "//! The application's shell: a title bar, a sidebar, and the view it shows.\n\
         //!\n\
         //! Ordinary Rust, written once and yours from here. maxx designs the\n\
         //! views under `src/ui/`; this file is what puts them side by side.\n\
         //!\n\
         //! Adding a page is five lines the compiler asks for one by one: a `Page`\n\
         //! variant, its label, a field, the entity that fills it, and a menu item.\n\
         \n\
         use gpui::{{Context, Entity, Window, div, prelude::*}};\n\
         use gpui_component::StyledExt;\n\
         use gpui_component::TitleBar;\n\
         use gpui_component::h_flex;\n\
         use gpui_component::sidebar::{{Sidebar, SidebarMenu, SidebarMenuItem}};\n\
         use gpui_component::v_flex;\n\
         \n\
         {imports}\
         \n\
         /// The pages the sidebar switches between.\n\
         #[derive(Clone, Copy, PartialEq, Eq)]\n\
         pub enum Page {{\n\
         {variants}\
         }}\n\
         \n\
         impl Page {{\n\
         \x20   /// The name the sidebar and the title bar both show. Written once,\n\
         \x20   /// so the two cannot come to disagree.\n\
         \x20   fn label(self) -> &'static str {{\n\
         \x20       match self {{\n\
         {labels}\
         \x20       }}\n\
         \x20   }}\n\
         }}\n\
         \n\
         pub struct Shell {{\n\
         \x20   page: Page,\n\
         {fields}\
         }}\n\
         \n\
         impl Shell {{\n\
         \x20   pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {{\n\
         \x20       Self {{\n\
         \x20           page: Page::{first},\n\
         {built}\
         \x20       }}\n\
         \x20   }}\n\
         \n\
         \x20   /// Every page is built once, at start-up, and kept afterwards:\n\
         \x20   /// coming back to one has to find it as it was left, not a fresh\n\
         \x20   /// one that forgot what was typed in it.\n\
         \x20   fn show(&mut self, page: Page, cx: &mut Context<Self>) {{\n\
         \x20       self.page = page;\n\
         \x20       cx.notify();\n\
         \x20   }}\n\
         }}\n\
         \n\
         impl Render for Shell {{\n\
         \x20   fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {{\n\
         \x20       v_flex()\n\
         \x20           .size_full()\n\
         \x20           .child(\n\
         \x20               // `main.rs` opens the window with\n\
         \x20               // `TitleBar::title_bar_options()`, which is what makes\n\
         \x20               // the system bar transparent; this is what is drawn in\n\
         \x20               // its place. The two go together — either one without\n\
         \x20               // the other shows at a glance, as a doubled bar or as a\n\
         \x20               // bare strip where the window controls sit.\n\
         \x20               TitleBar::new()\n\
         \x20                   .child(div().font_semibold().child(env!(\"CARGO_PKG_NAME\")))\n\
         \x20                   .child(div().child(self.page.label())),\n\
         \x20           )\n\
         \x20           .child(\n\
         \x20               h_flex()\n\
         \x20                   .flex_1()\n\
         \x20                   .overflow_hidden()\n\
         \x20                   .child(\n\
         \x20                       Sidebar::left().child(\n\
         \x20                           SidebarMenu::new()\n\
         {items},\n\
         \x20                       ),\n\
         \x20                   )\n\
         \x20                   .child(v_flex().flex_1().size_full().child(match self.page {{\n\
         {arms}\
         \x20                   }})),\n\
         \x20           )\n\
         \x20   }}\n\
         }}\n"
    )
}

/// The settings screen: what the application remembers, shown and changed.
///
/// Hand-written like the shell, and for the same reason: what it draws is
/// dictated by the fields of `settings::Settings`, which is the developer's to
/// grow. maxx writes a first one that works, against the module it copied.
pub fn settings_screen_rs() -> String {
    r#"//! The settings screen: what the application remembers, shown and changed.
//!
//! Written against `src/settings.rs`, which maxx copied in with it. Add a field
//! there, add a row here — the file is yours from here.

use gpui::{Context, Window, prelude::*};
use gpui_component::label::Label;
use gpui_component::switch::Switch;
use gpui_component::{h_flex, v_flex};

use crate::settings;

pub struct SettingsScreen {
    settings: settings::Settings,
}

impl SettingsScreen {
    pub fn new(_window: &mut Window, _cx: &mut Context<Self>) -> Self {
        Self { settings: settings::load() }
    }

    /// Written on every change rather than behind a “Save” button: the file is
    /// the truth, and a setting changed but not saved is a setting the user
    /// will find back where it was.
    fn set_dark_theme(&mut self, dark: bool, cx: &mut Context<Self>) {
        self.settings.dark_theme = dark;
        if let Err(error) = settings::save(&self.settings) {
            eprintln!("the settings could not be written: {error}");
        }
        cx.notify();
    }
}

impl Render for SettingsScreen {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .gap_4()
            .p_4()
            .child(Label::new("Settings"))
            .child(
                h_flex()
                    .gap_2()
                    .items_center()
                    .child(Label::new("Dark theme"))
                    .child(
                        Switch::new("dark-theme")
                            .checked(self.settings.dark_theme)
                            .on_click(cx.listener(|this, dark: &bool, _window, cx| {
                                this.set_dark_theme(*dark, cx)
                            })),
                    ),
            )
            // Where the file is, because a settings screen that hides its file
            // is a settings screen nobody can back up or edit by hand.
            .child(Label::new(settings::displayable_path()))
    }
}
"#
    .to_string()
}

/// The boxes maxx can write into a handler, by name.
///
/// Per entry: the name the interface uses, the `use` lines the body needs, and
/// the body itself — written against `window` and `cx`, the two parameters
/// every handler stub already carries.
///
/// A table of plain tuples rather than a struct, for the reason the top of this
/// file gives: `build.rs` includes it verbatim, and a type declared in maxx
/// would not be there.
pub const BOXES: &[(&str, &[&str], &str)] = &[
    (
        "dialog",
        &["use gpui::div;", "use gpui_component::WindowExt;"],
        "window.open_dialog(cx, |dialog, _window, _cx| {\n\
         \x20           dialog\n\
         \x20               .title(\"A dialog\")\n\
         \x20               .child(div().p_4().child(\"Written by maxx; the body is yours.\"))\n\
         \x20       });",
    ),
    (
        "sheet",
        &["use gpui::div;", "use gpui_component::WindowExt;"],
        "window.open_sheet(cx, |sheet, _window, _cx| {\n\
         \x20           sheet\n\
         \x20               .title(\"A sheet\")\n\
         \x20               .child(div().p_4().child(\"Written by maxx; the body is yours.\"))\n\
         \x20       });",
    ),
    (
        "notification",
        &["use gpui_component::WindowExt;", "use gpui_component::notification::Notification;"],
        "window.push_notification(Notification::info(\"Written by maxx.\"), cx);",
    ),
];

/// The sub-tree templates the palette drops in one gesture.
///
/// Per entry: the identifier the palette uses, the `use` lines the expression
/// needs, and the expression itself —
/// ordinary Rust, which `parser::parse_expr` reads into a tree exactly as it
/// reads the clipboard. No new machinery: a template is a piece of Rust in a
/// table, the way a component is. Written in `codegen`'s own spelling, which a
/// test holds to: what a drop puts in the file is then this text, character for
/// character, rather than a reflowed cousin of it. The label it wears is not
/// here but in
/// `registry`, with the interface's other strings: this file is included
/// verbatim by `build.rs` and holds only what a project gets.
///
/// All three are stateless on purpose. A template carrying `&self.field` would
/// name a field the view may not have, and the paste path rebinds those only
/// against fields that already exist — a form with real inputs is a template
/// that has to declare state first, which is a different feature.
pub const SUBTREES: &[(&str, &[&str], &str)] = &[
    (
        "card",
        &["use gpui_component::label::Label;", "use gpui_component::v_flex;"],
        "v_flex()\n\
         \x20   .gap_2()\n\
         \x20   .p_4()\n\
         \x20   .border_1()\n\
         \x20   .rounded_md()\n\
         \x20   .child(Label::new(\"Title\"))\n\
         \x20   .child(Label::new(\"Some content\"))",
    ),
    (
        "toolbar",
        &["use gpui_component::button::{Button, ButtonVariants};", "use gpui_component::h_flex;"],
        "h_flex()\n\
         \x20   .gap_2()\n\
         \x20   .items_center()\n\
         \x20   .child(Button::new(\"save\").label(\"Save\").primary())\n\
         \x20   .child(Button::new(\"cancel\").label(\"Cancel\"))",
    ),
    (
        "section",
        &[
            "use gpui_component::group_box::GroupBox;",
            "use gpui_component::label::Label;",
            "use gpui_component::v_flex;",
        ],
        "GroupBox::new()\n\
         \x20   .title(\"Section\")\n\
         \x20   .child(v_flex().gap_2().child(Label::new(\"First\")).child(Label::new(\"Second\")))",
    ),
];
