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
    // Sorted, because `rustfmt` sorts a run of `use` lines: written in page
    // order, a shell whose first page is not the first alphabetically comes back
    // reordered on the developer's first save.
    let mut import_lines: Vec<String> = pages
        .iter()
        .map(|(module, type_name, _)| format!("use crate::ui::{module}::{type_name};\n"))
        .collect();
    import_lines.sort();
    let imports: String = import_lines.concat();

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

/// A page a shape writes whole: a view, with the fields its tree binds.
///
/// The pages of a shape are views and not screens like the one above: what sits
/// between the markers stays maxx's, so the shape opens in the designer and
/// keeps being drawn there. What is written around the region is what maxx
/// never touches again — the struct, `new`, and the methods the tree names.
///
/// The fields are declared right here, which is the second of the two ways a
/// binding gets one: `view::render_source` writes them at a save, for a tree
/// drawn in the designer, and a file written straight to disk never goes
/// through a save. `fields` is, per entry: the name, the type, and the
/// expression `new` builds it with.
pub fn page_rs(
    type_name: &str,
    doc: &str,
    gpui_items: &[&str],
    imports: &[&str],
    fields: &[(&str, &str, &str)],
    methods: &str,
    body: &str,
) -> String {
    // One `use gpui::{…}` and not two: `rustfmt` sorts the items of a group and
    // the groups themselves, so a second line would come back reordered on the
    // developer's first save — and a shape a formatter rewrites is a shape that
    // shows up in a diff nobody wrote.
    let mut items: Vec<&str> = vec!["Context", "Window"];
    items.extend_from_slice(gpui_items);
    items.sort_unstable();
    items.dedup();
    let gpui = items.join(", ");

    let declared: String =
        fields.iter().map(|(name, ty, _)| format!("    {name}: {ty},\n")).collect();
    let initialized: String = fields
        .iter()
        .map(|(name, _, initial)| format!("            {name}: {initial},\n"))
        .collect();
    // An empty struct is written `{}` on one line, and a short literal on one
    // line too: that is what `rustfmt` does with them — `struct_lit_width` is
    // eighteen characters of body — and a shape a formatter rewrites is a shape
    // that shows up in the developer's first diff.
    let struct_body =
        if fields.is_empty() { "{}".to_string() } else { format!("{{\n{declared}}}") };
    let inline: String = fields
        .iter()
        .map(|(name, _, initial)| format!("{name}: {initial}"))
        .collect::<Vec<_>>()
        .join(", ");
    let literal = if fields.is_empty() {
        "Self {}".to_string()
    } else if inline.len() <= 18 {
        format!("Self {{ {inline} }}")
    } else {
        format!("Self {{\n{initialized}        }}")
    };

    // The parameters are named after what is used, not after what is passed: an
    // unused `window` is a warning in a project built with the usual lints, and
    // maxx must not be the one who wrote it.
    let initials: String =
        fields.iter().map(|(_, _, initial)| *initial).collect::<Vec<_>>().join(" ");
    let new_window = if initials.contains("window") { "window" } else { "_window" };
    let new_cx = if initials.contains("cx") { "cx" } else { "_cx" };
    let render_cx = if body.contains("cx.") { "cx" } else { "_cx" };

    let indented: String = body
        .lines()
        .map(|line| if line.is_empty() { "\n".into() } else { format!("        {line}\n") })
        .collect();
    let imports: String = imports.iter().map(|line| format!("{line}\n")).collect();
    let methods = if methods.is_empty() { String::new() } else { format!("\n{methods}") };

    format!(
        "//! {doc}\n\
         //!\n\
         //! A view maxx draws: the tree between the markers is maxx's, and\n\
         //! everything around it is yours.\n\
         \n\
         use gpui::{{{gpui}, prelude::*}};\n\
         {imports}\
         \n\
         pub struct {type_name} {struct_body}\n\
         \n\
         impl {type_name} {{\n\
         \x20   pub fn new({new_window}: &mut Window, {new_cx}: &mut Context<Self>) -> Self {{\n\
         \x20       {literal}\n\
         \x20   }}\n\
         {methods}\
         }}\n\
         \n\
         impl Render for {type_name} {{\n\
         \x20   fn render(&mut self, _window: &mut Window, {render_cx}: &mut Context<Self>) -> impl IntoElement {{\n\
         \x20       // maxx:begin\n\
         {indented}\
         \x20       // maxx:end\n\
         \x20   }}\n\
         }}\n"
    )
}

/// How the table below holds a page: a function, so the text is built only when
/// a project asks for it.
pub type PageSource = fn() -> String;

/// The pages the shapes bring with them, by the module each is written to.
///
/// One table read from both sides: `create_project` writes these files into a
/// project, and `build.rs` writes the same text into `OUT_DIR` where
/// `examples/shapes.rs` compiles it. A page that compiles nowhere else is a
/// project that stops building on a line maxx wrote.
///
/// Nothing here reaches for a module of the project — no `crate::settings`, no
/// `crate::theme` — so a page is compiled on `gpui` and `gpui-component` alone,
/// and a shape can hand it out without dragging a module behind it.
pub const SHAPE_PAGES: &[(&str, PageSource)] = &[
    ("items", items_rs),
    ("form", form_rs),
    ("dashboard", dashboard_rs),
    ("wizard", wizard_rs),
    ("home", utility_rs),
    ("editor", editor_rs),
];

/// A list on the left, the detail of what is selected on the right.
fn items_rs() -> String {
    page_rs(
        "Items",
        "A list, and the detail of the row that is selected.",
        &[],
        &[
            "use gpui_component::button::{Button, ButtonVariants};",
            "use gpui_component::label::Label;",
            "use gpui_component::{Selectable, h_flex, v_flex};",
        ],
        &[("selected", "usize", "0")],
        r#"    /// The rows the list shows, written here rather than fetched: where the
    /// data comes from is yours to decide, and a shape that pretended to know
    /// would be in the way.
    const ITEMS: &[(&str, &str)] = &[
        ("First item", "What this one is about."),
        ("Second item", "And what that one is about."),
        ("Third item", "The panel on the right follows this."),
    ];

    /// The selection, which is the only thing the two panels share.
    fn select(&mut self, index: usize, cx: &mut Context<Self>) {
        self.selected = index;
        cx.notify();
    }

    fn title(&self) -> &'static str {
        Self::ITEMS[self.selected].0
    }

    fn detail(&self) -> &'static str {
        Self::ITEMS[self.selected].1
    }
"#,
        r#"h_flex()
    .id("items")
    .size_full()
    .child(
        v_flex()
            .w_64()
            .h_full()
            .gap_1()
            .p_2()
            .child(
                Button::new("item-0")
                    .label("First item")
                    .ghost()
                    .selected(self.selected == 0)
                    .on_click(cx.listener(|this, _, _window, cx| this.select(0, cx))),
            )
            .child(
                Button::new("item-1")
                    .label("Second item")
                    .ghost()
                    .selected(self.selected == 1)
                    .on_click(cx.listener(|this, _, _window, cx| this.select(1, cx))),
            )
            .child(
                Button::new("item-2")
                    .label("Third item")
                    .ghost()
                    .selected(self.selected == 2)
                    .on_click(cx.listener(|this, _, _window, cx| this.select(2, cx))),
            ),
    )
    .child(
        v_flex()
            .flex_1()
            .gap_2()
            .p_4()
            .child(Label::new(self.title()).text_xl())
            .child(Label::new(self.detail())),
    )"#,
    )
}

/// Fields that are really typed into, and a Save button that reads them.
fn form_rs() -> String {
    page_rs(
        "Form",
        "A form: fields bound to state, and a button that reads them.",
        &["ClickEvent", "Entity", "SharedString"],
        &[
            "use gpui_component::button::{Button, ButtonVariants};",
            "use gpui_component::input::{Input, InputState};",
            "use gpui_component::label::Label;",
            "use gpui_component::{h_flex, v_flex};",
        ],
        &[
            (
                "name",
                "Entity<InputState>",
                "cx.new(|cx| InputState::new(window, cx).placeholder(\"Ada Lovelace\"))",
            ),
            (
                "email",
                "Entity<InputState>",
                "cx.new(|cx| InputState::new(window, cx).placeholder(\"ada@example.com\"))",
            ),
            ("saved", "SharedString", "SharedString::default()"),
        ],
        r#"    /// Written by maxx; what becomes of the values is yours.
    ///
    /// The two fields are read through `cx` rather than mirrored into the
    /// struct on every keystroke: the input state is the truth, and a copy
    /// kept beside it is a copy that goes stale.
    fn on_save(&mut self, _event: &ClickEvent, _window: &mut Window, cx: &mut Context<Self>) {
        let name = self.name.read(cx).value();
        let email = self.email.read(cx).value();
        self.saved = format!("Saved {name} <{email}>").into();
        cx.notify();
    }
"#,
        r#"v_flex()
    .id("form")
    .size_full()
    .gap_4()
    .p_4()
    .child(Label::new("New contact").text_xl())
    .child(
        v_flex()
            .gap_1()
            .child(Label::new("Name"))
            .child(Input::new(&self.name)),
    )
    .child(
        v_flex()
            .gap_1()
            .child(Label::new("Email"))
            .child(Input::new(&self.email)),
    )
    .child(
        h_flex().gap_2().justify_end().child(
            Button::new("save")
                .label("Save")
                .primary()
                .on_click(cx.listener(Self::on_save)),
        ),
    )
    .child(Label::new(self.saved.clone()).text_xs())"#,
    )
}

/// A header, a grid of cards, and the numbers they carry.
fn dashboard_rs() -> String {
    page_rs(
        "Dashboard",
        "A dashboard: a header, and a grid of cards carrying numbers.",
        &[],
        &[
            "use gpui_component::group_box::GroupBox;",
            "use gpui_component::label::Label;",
            "use gpui_component::{h_flex, v_flex};",
        ],
        &[],
        "",
        r#"v_flex()
    .id("dashboard")
    .size_full()
    .gap_4()
    .p_4()
    .child(
        h_flex()
            .items_center()
            .justify_between()
            .child(Label::new("Overview").text_xl())
            .child(Label::new("Last 30 days").text_xs()),
    )
    .child(
        h_flex()
            .gap_4()
            .child(
                GroupBox::new()
                    .title("Revenue")
                    .flex_1()
                    .child(Label::new("12,480").text_xl()),
            )
            .child(
                GroupBox::new()
                    .title("Orders")
                    .flex_1()
                    .child(Label::new("318").text_xl()),
            )
            .child(
                GroupBox::new()
                    .title("Customers")
                    .flex_1()
                    .child(Label::new("1,204").text_xl()),
            ),
    )
    .child(
        h_flex()
            .gap_4()
            .child(
                GroupBox::new()
                    .title("Refunds")
                    .flex_1()
                    .child(Label::new("12").text_xl()),
            )
            .child(
                GroupBox::new()
                    .title("Open tickets")
                    .flex_1()
                    .child(Label::new("7").text_xl()),
            )
            .child(
                GroupBox::new()
                    .title("Uptime")
                    .flex_1()
                    .child(Label::new("99.9%").text_xl()),
            ),
    )"#,
    )
}

/// Steps, a bar that says where you are, and the two buttons that move.
fn wizard_rs() -> String {
    page_rs(
        "Wizard",
        "A wizard: steps, an indicator, and the two buttons that move between them.",
        &["ClickEvent"],
        &[
            "use gpui_component::button::{Button, ButtonVariants};",
            "use gpui_component::label::Label;",
            "use gpui_component::progress::Progress;",
            "use gpui_component::{Disableable, h_flex, v_flex};",
        ],
        &[("step", "usize", "0")],
        r#"    /// The steps, in order. Adding one is a line here and nothing else.
    const STEPS: &[(&str, &str)] = &[
        ("Welcome", "What this assistant is about to do."),
        ("Details", "What it needs from the person using it."),
        ("Done", "What happened, and what to do next."),
    ];

    fn back(&mut self, _event: &ClickEvent, _window: &mut Window, cx: &mut Context<Self>) {
        self.step = self.step.saturating_sub(1);
        cx.notify();
    }

    fn forward(&mut self, _event: &ClickEvent, _window: &mut Window, cx: &mut Context<Self>) {
        self.step = (self.step + 1).min(Self::STEPS.len() - 1);
        cx.notify();
    }

    fn title(&self) -> &'static str {
        Self::STEPS[self.step].0
    }

    fn hint(&self) -> &'static str {
        Self::STEPS[self.step].1
    }

    fn counter(&self) -> String {
        format!("Step {} of {}", self.step + 1, Self::STEPS.len())
    }

    /// How far along, as the bar wants it: a percentage.
    fn progress(&self) -> f32 {
        (self.step + 1) as f32 * 100. / Self::STEPS.len() as f32
    }
"#,
        r#"v_flex()
    .id("wizard")
    .size_full()
    .gap_4()
    .p_4()
    .child(Label::new(self.counter()).text_xs())
    .child(Progress::new().value(self.progress()))
    .child(Label::new(self.title()).text_xl())
    .child(Label::new(self.hint()))
    .child(
        h_flex()
            .gap_2()
            .justify_end()
            .child(
                Button::new("back")
                    .label("Previous")
                    .disabled(self.step == 0)
                    .on_click(cx.listener(Self::back)),
            )
            .child(
                Button::new("next")
                    .label("Next")
                    .primary()
                    .disabled(self.step + 1 == Self::STEPS.len())
                    .on_click(cx.listener(Self::forward)),
            ),
    )"#,
    )
}

/// One window, one job. The whole application, and it is called `home`.
fn utility_rs() -> String {
    page_rs(
        "Home",
        "A tool in one window: something to type in, something that acts on it.",
        &["ClickEvent", "Entity", "SharedString"],
        &[
            "use gpui_component::button::{Button, ButtonVariants};",
            "use gpui_component::input::{Input, InputState};",
            "use gpui_component::label::Label;",
            "use gpui_component::{h_flex, v_flex};",
        ],
        &[
            (
                "entry",
                "Entity<InputState>",
                "cx.new(|cx| InputState::new(window, cx).placeholder(\"Type something\"))",
            ),
            ("answer", "SharedString", "SharedString::default()"),
        ],
        r#"    /// What the tool does, which is the one thing to replace.
    fn run(&mut self, _event: &ClickEvent, _window: &mut Window, cx: &mut Context<Self>) {
        let value = self.entry.read(cx).value();
        self.answer = format!("{} characters", value.chars().count()).into();
        cx.notify();
    }
"#,
        r#"v_flex()
    .id("home")
    .size_full()
    .gap_3()
    .p_4()
    .child(Label::new("Count the characters").text_lg())
    .child(Input::new(&self.entry))
    .child(
        h_flex().gap_2().justify_end().child(
            Button::new("run")
                .label("Run")
                .primary()
                .on_click(cx.listener(Self::run)),
        ),
    )
    .child(Label::new(self.answer.clone()).text_xs())"#,
    )
}

/// Tabs above, text in the middle, a status bar below.
fn editor_rs() -> String {
    page_rs(
        "Editor",
        "An editor: a strip of tabs, a multi-line text area, and a status bar.",
        &["Entity"],
        &[
            "use gpui_component::input::{Input, InputState};",
            "use gpui_component::label::Label;",
            "use gpui_component::tab::TabBar;",
            "use gpui_component::{h_flex, v_flex};",
        ],
        &[
            ("tab", "usize", "0"),
            (
                "content",
                "Entity<InputState>",
                "cx.new(|cx| InputState::new(window, cx).multi_line(true))",
            ),
        ],
        r#"    /// The names the status bar reads back.
    ///
    /// The strip itself carries its labels in the tree, where maxx can change
    /// them: a `Tab` is a type and not an element, so the bar is one node and
    /// its labels are its own arguments. Renaming a tab is therefore two edits
    /// — the label above, and the line here.
    const TABS: &[&str] = &["Draft", "Notes", "Scratch"];

    /// Switching tabs leaves the text where it is: one buffer is what a shape
    /// can honestly offer, and three buffers is a decision about the document
    /// model that belongs to whoever writes the application.
    fn show(&mut self, tab: usize, cx: &mut Context<Self>) {
        self.tab = tab;
        cx.notify();
    }

    fn name(&self) -> &'static str {
        Self::TABS[self.tab]
    }

    fn length(&self, cx: &Context<Self>) -> String {
        let value = self.content.read(cx).value();
        format!("{} characters", value.chars().count())
    }
"#,
        r#"v_flex()
    .id("editor")
    .size_full()
    .child(
        TabBar::new("editor-tabs")
            .selected_index(self.tab)
            .child("Draft")
            .child("Notes")
            .child("Scratch")
            .on_click(
                cx.listener(|this, index: &usize, _window, cx| this.show(*index, cx)),
            ),
    )
    .child(
        v_flex()
            .flex_1()
            .p_2()
            .child(Input::new(&self.content).h_full()),
    )
    .child(
        h_flex()
            .items_center()
            .justify_between()
            .px_2()
            .py_1()
            .child(Label::new(self.name()).text_xs())
            .child(Label::new(self.length(cx)).text_xs()),
    )"#,
    )
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
/// needs, the fields a view holding it declares, and the expression itself —
/// ordinary Rust, which `parser::parse_expr` reads into a tree exactly as it
/// reads the clipboard. No new machinery: a template is a piece of Rust in a
/// table, the way a component is. Written in `codegen`'s own spelling, which a
/// test holds to: what a drop puts in the file is then this text, character for
/// character, rather than a reflowed cousin of it. The label it wears is not
/// here but in
/// `registry`, with the interface's other strings: this file is included
/// verbatim by `build.rs` and holds only what a project gets.
///
/// A template may bind state — `&self.field` — and carry handlers. What a drop
/// owes it is worked out where it already is for a pasted copy:
/// `registry::rebind_state_fields` moves the binding off a name the view
/// already uses, and `view::ensure_state_field` declares whatever binding
/// survives, at the save, along with `ensure_handler` for the methods named.
/// The third column is not that declaration but the build check's: the fields
/// are written fully qualified so that compiling the table owes no `use` line
/// of its own.
pub const SUBTREES: &[(&str, &[&str], &[&str], &str)] = &[
    (
        "card",
        &["use gpui_component::label::Label;", "use gpui_component::v_flex;"],
        &[],
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
        &[],
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
        &[],
        "GroupBox::new()\n\
         \x20   .title(\"Section\")\n\
         \x20   .child(v_flex().gap_2().child(Label::new(\"First\")).child(Label::new(\"Second\")))",
    ),
    // The one that had to wait for a template to be allowed state: a label over
    // a real input, with the line under it that says what to type. Bound to
    // `field`, which is the name a dropped input takes too — so a second drop
    // is renamed rather than mirrored, by the same rule.
    (
        "form_field",
        &[
            "use gpui_component::input::Input;",
            "use gpui_component::label::Label;",
            "use gpui_component::v_flex;",
        ],
        &["field: gpui::Entity<gpui_component::input::InputState>"],
        "v_flex()\n\
         \x20   .gap_1()\n\
         \x20   .child(Label::new(\"Label\"))\n\
         \x20   .child(Input::new(&self.field))\n\
         \x20   .child(Label::new(\"Help text\").text_xs())",
    ),
    (
        "page_header",
        &[
            "use gpui_component::button::{Button, ButtonVariants};",
            "use gpui_component::h_flex;",
            "use gpui_component::label::Label;",
        ],
        &[],
        "h_flex()\n\
         \x20   .gap_2()\n\
         \x20   .items_center()\n\
         \x20   .justify_between()\n\
         \x20   .child(Label::new(\"Page title\").text_xl())\n\
         \x20   .child(h_flex().gap_2().child(Button::new(\"action\").label(\"Action\").primary()))",
    ),
    (
        "status_bar",
        &["use gpui_component::h_flex;", "use gpui_component::label::Label;"],
        &[],
        "h_flex()\n\
         \x20   .gap_2()\n\
         \x20   .items_center()\n\
         \x20   .justify_between()\n\
         \x20   .px_2()\n\
         \x20   .py_1()\n\
         \x20   .border_1()\n\
         \x20   .child(Label::new(\"Ready\").text_xs())\n\
         \x20   .child(Label::new(\"Ln 1, Col 1\").text_xs())",
    ),
    // Made of catalogue entries and not of the `EmptyState` the component
    // library carries: what a template drops has to be a tree maxx can take
    // apart, and a brick is one node whose inside is a file.
    (
        "empty_state",
        &[
            "use gpui_component::button::{Button, ButtonVariants};",
            "use gpui_component::label::Label;",
            "use gpui_component::v_flex;",
            "use gpui_component::{Icon, IconName};",
        ],
        &[],
        "v_flex()\n\
         \x20   .gap_2()\n\
         \x20   .p_8()\n\
         \x20   .items_center()\n\
         \x20   .justify_center()\n\
         \x20   .child(Icon::new(IconName::Folder))\n\
         \x20   .child(Label::new(\"Nothing here yet\"))\n\
         \x20   .child(Button::new(\"create\").label(\"Create one\").primary())",
    ),
    (
        "confirm_row",
        &["use gpui_component::button::{Button, ButtonVariants};", "use gpui_component::h_flex;"],
        &[],
        "h_flex()\n\
         \x20   .gap_2()\n\
         \x20   .justify_end()\n\
         \x20   .child(\n\
         \x20       Button::new(\"cancel\")\n\
         \x20           .label(\"Cancel\")\n\
         \x20           .on_click(cx.listener(Self::on_cancel)),\n\
         \x20   )\n\
         \x20   .child(\n\
         \x20       Button::new(\"ok\")\n\
         \x20           .label(\"OK\")\n\
         \x20           .primary()\n\
         \x20           .on_click(cx.listener(Self::on_ok)),\n\
         \x20   )",
    ),
];

// The components maxx copies into a project, each as one file of `src/components/`.
//
// Real components, not the sub-trees above. The difference is the one that
// matters on the third day of a project: a sub-tree is pasted, so ten cards are
// ten copies and changing the look is ten edits; a component is a type, so ten
// cards are ten calls and changing the look is one file.
//
// They are written against the project's own `theme` module, which is why
// adding them adds that too. A brick that carries its own colours would ignore
// the palette the developer just chose.
//
// The shape is deliberate and it is a contract: `pub fn new(..)` for what the
// component cannot do without, `pub fn x(mut self, ..) -> Self` for what it can.
// It is the shape gpui-component itself uses, the shape a Rust developer
// expects — and the shape maxx will read back to offer these in the catalogue.
pub const COMPONENTS: &[(&str, &str)] = &[
    (
        "card",
        r#"//! A titled box, with anything inside it.

use gpui::prelude::*;
use gpui::{AnyElement, App, FontWeight, IntoElement, RenderOnce, SharedString, Window, div};
use gpui_component::v_flex;

use crate::theme;

/// A box with a title, an optional subtitle, and whatever it is given.
#[derive(IntoElement)]
pub struct Card {
    title: SharedString,
    subtitle: Option<SharedString>,
    children: Vec<AnyElement>,
}

impl Card {
    /// A card with its title. The one thing a card cannot do without.
    pub fn new(title: impl Into<SharedString>) -> Self {
        Self { title: title.into(), subtitle: None, children: Vec::new() }
    }

    /// A line under the title, for what the title does not say.
    pub fn subtitle(mut self, subtitle: impl Into<SharedString>) -> Self {
        self.subtitle = Some(subtitle.into());
        self
    }
}

impl ParentElement for Card {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl RenderOnce for Card {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        v_flex()
            .gap_2()
            .p_4()
            .rounded_md()
            .border_1()
            .border_color(theme::BORDER.get(cx))
            .bg(theme::PANEL.get(cx))
            .child(
                div()
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(theme::TEXT.get(cx))
                    .child(self.title),
            )
            .children(self.subtitle.map(|subtitle| {
                div().text_sm().text_color(theme::TEXT_MUTED.get(cx)).child(subtitle)
            }))
            .children(self.children)
    }
}
"#,
    ),
    (
        "toolbar",
        r#"//! A row of controls, with a gap and an alignment already decided.

use gpui::prelude::*;
use gpui::{AnyElement, App, IntoElement, RenderOnce, Window};
use gpui_component::h_flex;

use crate::theme;

/// A horizontal bar for buttons and the like.
#[derive(IntoElement)]
pub struct Toolbar {
    children: Vec<AnyElement>,
    separated: bool,
}

impl Toolbar {
    /// An empty toolbar.
    pub fn new() -> Self {
        Self { children: Vec::new(), separated: false }
    }

    /// A line under the bar, when it sits above what it acts on.
    pub fn separated(mut self) -> Self {
        self.separated = true;
        self
    }
}

impl Default for Toolbar {
    fn default() -> Self {
        Self::new()
    }
}

impl ParentElement for Toolbar {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl RenderOnce for Toolbar {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        h_flex()
            .gap_2()
            .items_center()
            .px_3()
            .py_2()
            .when(self.separated, |this| {
                this.border_b_1().border_color(theme::BORDER.get(cx))
            })
            .children(self.children)
    }
}
"#,
    ),
    (
        "empty_state",
        r#"//! What a screen shows before it has anything to show.

use gpui::prelude::*;
use gpui::{AnyElement, App, IntoElement, RenderOnce, SharedString, Window, div};
use gpui_component::v_flex;

use crate::theme;

/// The screen every application needs and nobody writes on the first day: a
/// title, a line saying what to do, and room for the button that does it.
#[derive(IntoElement)]
pub struct EmptyState {
    title: SharedString,
    hint: Option<SharedString>,
    children: Vec<AnyElement>,
}

impl EmptyState {
    /// The state with its title.
    pub fn new(title: impl Into<SharedString>) -> Self {
        Self { title: title.into(), hint: None, children: Vec::new() }
    }

    /// What the reader should do about it.
    pub fn hint(mut self, hint: impl Into<SharedString>) -> Self {
        self.hint = Some(hint.into());
        self
    }
}

impl ParentElement for EmptyState {
    fn extend(&mut self, elements: impl IntoIterator<Item = AnyElement>) {
        self.children.extend(elements);
    }
}

impl RenderOnce for EmptyState {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        v_flex()
            .flex_1()
            .items_center()
            .justify_center()
            .gap_2()
            .p_8()
            .child(div().text_lg().text_color(theme::TEXT.get(cx)).child(self.title))
            .children(
                self.hint.map(|hint| {
                    div().text_sm().text_color(theme::TEXT_MUTED.get(cx)).child(hint)
                }),
            )
            .children(self.children)
    }
}
"#,
    ),
];
