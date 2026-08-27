//! The component catalogue.
//!
//! One table, extended once per component. Each entry says how the component is
//! written in Rust, which `use` it needs, and which properties the inspector may
//! edit. The canvas renderer in [`crate::canvas`] reads the same table, so the
//! preview and the generated code cannot drift apart.

use crate::model::{Arg, Base, Node};

/// How a property maps onto the builder chain.
#[derive(Clone, Copy, Debug)]
pub enum Target {
    /// An argument of the constructor: `Label::new(<here>)`.
    BaseArg(usize),
    /// A method taking one argument: `.label(<here>)`.
    Method(&'static str),
    /// A method with no argument, present or absent: `.flex_1()`.
    Flag(&'static str),
    /// A family of no-argument methods of which at most one applies, e.g.
    /// `gap_1` … `gap_8`. Setting one removes the others.
    Family(&'static [&'static str]),
    /// A method taking one variant of an enumeration: `.object_fit(
    /// ObjectFit::Cover)`.
    ///
    /// The variants are written out in full, path included, because that is
    /// what goes into the file — and because the inspector shows them as they
    /// are written, the way it already shows `gap_2` rather than "medium".
    Variant(&'static str, &'static [&'static str]),
    /// One variant of an enumeration, as an argument of the constructor:
    /// `Icon::new(IconName::Search)`.
    ///
    /// Apart from [`Target::Variant`], which puts one in a method: an argument
    /// of the base is not a call that can be added and removed — it is there or
    /// the component does not compile — so the empty choice does not exist
    /// here.
    VariantArg(usize, &'static [&'static str]),
    /// Scrolling on the named axis: `overflow_y_scroll` or its horizontal
    /// counterpart.
    ///
    /// Its own target and not a [`Target::Flag`] because it takes two calls,
    /// not one: gpui keeps a scroll offset between frames only for an element
    /// that has an `id`, so the flag alone would clip the content and never
    /// scroll it. The id has to be unique among siblings, which no single node
    /// can know — that is why the workspace writes this one, not [`write`].
    Scrollable(&'static str),
}

/// What kind of editor the inspector shows.
#[derive(Clone, Copy, Debug)]
pub enum Kind {
    /// Free text, written as a string literal.
    Text,
    /// A checkbox.
    Bool,
    /// One of the family's methods, or none.
    Choice,
    /// Free text written as `&self.<value>`.
    Field,
    /// The name of a method of the view, written as `cx.listener(Self::<value>)`
    /// and backed by a stub inserted next to the view's other methods.
    Handler,
    /// A length in pixels, written as `px(<value>.)`.
    Number,
    /// A bare number, written as a float literal: `.value(50.)`.
    ///
    /// Distinct from [`Kind::Number`], which is a length and wears `px(…)`.
    /// Writing a ratio as a length would not compile in the generated project.
    Ratio,
    /// A whole number, written bare: `.count(3)`.
    ///
    /// Distinct from [`Kind::Ratio`], which writes `3.`: the counts of a badge
    /// are `usize`, and a float literal there does not compile.
    Count,
    /// A colour, written as `rgb(0x<value>)`.
    Color,
    /// A file of the project, written as `"<value>"`, relative to the root.
    ///
    /// A bare string, because that is the spelling gpui looks up in the
    /// application's `AssetSource` — and the assets module is what declares
    /// one. A `PathBuf` never consults it: it is read from the disk relative to
    /// the directory the process started in, which is the project root under
    /// `cargo run` and anything at all for a binary someone double-clicked.
    ///
    /// Older projects hold `PathBuf::from("…")`, which still reads back, and
    /// which [`write`] keeps rather than converts: the two spellings do not
    /// mean the same thing, and changing one under the developer is not
    /// maxx's to do.
    Path,
}

/// Which shared style properties a component accepts.
///
/// `COMMON` was posed on everything, so an image offered a text size and a font
/// weight — five rows that do nothing, drowning the two that matter. And it is
/// not only noise: a component that does not implement `Styled` at all would be
/// handed calls that do not compile.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Common {
    /// Everything: size, box and text.
    All,
    /// Size and box, for a component that draws no text of its own.
    Box,
    /// None, for one that is not `Styled`.
    None,
}

/// One editable property.
#[derive(Clone, Copy, Debug)]
pub struct Prop {
    /// Translation key of the label shown in the inspector.
    ///
    /// A key and not the text: it is stable across languages, so it can also
    /// serve as the element's identity — which is what the inspector's field
    /// ids are built from.
    pub label: &'static str,
    /// Where the value lives in the chain.
    pub target: Target,
    /// Which editor to show.
    pub kind: Kind,
}

/// One entry of the catalogue.
#[derive(Clone, Copy, Debug)]
pub struct Spec {
    /// Stable identifier, used by the palette and the drag payloads.
    pub id: &'static str,
    /// Translation key of the name shown in the palette and the tree.
    pub label: &'static str,
    /// Constructor path emitted in the generated code.
    pub base: &'static str,
    /// The `use` lines the generated file always needs for this component.
    pub imports: &'static [&'static str],
    /// The `use` lines it needs only once a given call is written.
    ///
    /// A variant or a `disabled` comes from a trait, and a trait has to be in
    /// scope: with the type alone, the call is a method the generated project
    /// does not have, and it says so only when the developer builds it. But
    /// imported on sight of the component, the trait is unused on the button
    /// that has no variant — a warning in a project maxx has just written.
    ///
    /// Hence the condition, and hence its being held per component: `outline`
    /// is a variant of a button and a flag of a tag, so a table of call names
    /// alone would import the button's trait into a file that only has tags.
    pub extra_imports: &'static [(&'static [&'static str], &'static str)],
    /// Whether this component accepts children.
    pub container: bool,
    /// Constructor arguments used when the component is first dropped.
    pub default_args: &'static [&'static str],
    /// No-argument calls set when the component is first dropped.
    ///
    /// What the component needs to behave well on the very first frame, and
    /// which the developer is free to remove afterwards: an image that fits the
    /// column it lands in rather than pushing everything aside. A default in
    /// the table, not a special case in the code.
    pub default_calls: &'static [&'static str],
    /// Properties the inspector exposes.
    pub props: &'static [Prop],
    /// Which shared style properties it accepts.
    pub common: Common,
    /// The shape of the method its action property calls, when it has one.
    pub handler: Option<HandlerSpec>,
    /// The field this component needs on the view, when it needs one.
    ///
    /// A text input and a dropdown are not values but entities the view owns:
    /// they cannot be written as an expression in the region, only bound to a
    /// field that `new` builds. This is what tells `view::save` which field to
    /// declare, and with what.
    pub state: Option<StateSpec>,
}

/// The shape of the method a component's action calls.
///
/// `Button::on_click` hands a `&ClickEvent`, `Switch::on_click` hands a `&bool`
/// — the state it has just moved to. Writing one stub for both would leave a
/// project that does not compile, and the error would point at generated code.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct HandlerSpec {
    /// The second parameter of the method, declaration included.
    pub argument: &'static str,
    /// The `use` lines that parameter needs.
    pub imports: &'static [&'static str],
}

/// What a `Button` hands its handler.
const CLICK: HandlerSpec =
    HandlerSpec { argument: "_event: &ClickEvent", imports: &["use gpui::ClickEvent;"] };

/// What a switch, a checkbox or a radio hands its handler: the new state.
const TOGGLED: HandlerSpec = HandlerSpec { argument: "_on: &bool", imports: &[] };

/// The field a stateful component binds to.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct StateSpec {
    /// The declared type, verbatim.
    pub ty: &'static str,
    /// The `use` lines the field itself needs, beyond the component's own.
    pub imports: &'static [&'static str],
    /// The expression `new` initializes it with.
    pub initializer: &'static str,
}

const GAPS: &[&str] = &["gap_0", "gap_1", "gap_2", "gap_3", "gap_4", "gap_6", "gap_8"];
const PADDINGS: &[&str] = &["p_0", "p_1", "p_2", "p_3", "p_4", "p_6", "p_8"];
const ALIGNS: &[&str] = &["items_start", "items_center", "items_end"];
const VARIANTS: &[&str] = &["primary", "danger", "outline", "ghost", "link"];
const TEXT_SIZES: &[&str] = &["text_xs", "text_sm", "text_base", "text_lg", "text_xl", "text_2xl"];
/// gpui has no `font_medium()`: the weight goes through `font_weight`, which
/// takes a variant. Written as a family of no-argument methods — the shape
/// every other choice has — it produced a project that would not compile, on a
/// line maxx wrote itself, and only when the developer built it.
const WEIGHTS: &[&str] =
    &["FontWeight::NORMAL", "FontWeight::MEDIUM", "FontWeight::SEMIBOLD", "FontWeight::BOLD"];
/// How an image's box relates to what holds it: as wide as the picture, never
/// wider than the container, or exactly the container.
const IMAGE_SIZES: &[&str] = &["max_w_full", "w_full"];

/// gpui's own list, in the order one reaches for them. `Contain` is what gpui
/// does when nothing is said, so it is written only when chosen — like every
/// other value the inspector leaves at its default.
const OBJECT_FITS: &[&str] = &[
    "ObjectFit::Contain",
    "ObjectFit::Cover",
    "ObjectFit::Fill",
    "ObjectFit::ScaleDown",
    "ObjectFit::None",
];

/// The variants of a tag, as `Tag::with_variant` takes them.
///
/// The method and not the constructors — `Tag::primary()` is one too — because
/// changing the variant would otherwise change the node's base, and the tree
/// would lose the calls hanging off it.
const TAG_VARIANTS: &[&str] = &[
    "TagVariant::Primary",
    "TagVariant::Secondary",
    "TagVariant::Danger",
    "TagVariant::Success",
    "TagVariant::Warning",
    "TagVariant::Info",
];

/// The direction a slider runs in.
const SLIDER_AXES: &[&str] = &["horizontal", "vertical"];

/// The icons offered in the inspector.
///
/// A choice out of the eighty-eight `IconName` carries: a list that long is not
/// a list one picks from, and every name here is drawn on the canvas by
/// `designer::icon_named`, which is what `tests/catalogue.rs` holds the two
/// sides to.
const ICONS: &[&str] = &[
    "IconName::Check",
    "IconName::Close",
    "IconName::Search",
    "IconName::Settings",
    "IconName::Plus",
    "IconName::Minus",
    "IconName::Info",
    "IconName::TriangleAlert",
    "IconName::CircleCheck",
    "IconName::CircleX",
    "IconName::Star",
    "IconName::Heart",
    "IconName::Bell",
    "IconName::Calendar",
    "IconName::File",
    "IconName::Folder",
    "IconName::Globe",
    "IconName::User",
    "IconName::Copy",
    "IconName::Delete",
    "IconName::Eye",
    "IconName::ArrowRight",
];

const ROUNDED: &[&str] =
    &["rounded_none", "rounded_sm", "rounded_md", "rounded_lg", "rounded_full"];

/// Properties every component accepts.
///
/// Every widget of `gpui-component` implements `Styled`, so these are safe to
/// emit on any of them — checked component by component before adding them
/// here, because a style method on a type that does not implement `Styled`
/// would only fail when the generated project is compiled.
pub const COMMON: &[Prop] = &[
    Prop { label: "prop.width", target: Target::Method("w"), kind: Kind::Number },
    Prop { label: "prop.height", target: Target::Method("h"), kind: Kind::Number },
    Prop { label: "prop.background", target: Target::Method("bg"), kind: Kind::Color },
    Prop { label: "prop.rounded", target: Target::Family(ROUNDED), kind: Kind::Choice },
];

/// The shared properties that only make sense where there is text.
///
/// Kept apart rather than filtered by name: the list is the answer to "does
/// this component draw text of its own", and that is a question about the
/// component, not about the property's spelling.
pub const TEXT_COMMON: &[Prop] = &[
    Prop { label: "prop.text_color", target: Target::Method("text_color"), kind: Kind::Color },
    Prop { label: "prop.text_size", target: Target::Family(TEXT_SIZES), kind: Kind::Choice },
    Prop {
        label: "prop.weight",
        target: Target::Variant("font_weight", WEIGHTS),
        kind: Kind::Choice,
    },
];

/// The catalogue. Adding a component means adding an entry here and a branch in
/// [`crate::canvas::render_node`].
pub const CATALOGUE: &[Spec] = &[
    Spec {
        id: "column",
        label: "component.column",
        base: "v_flex",
        imports: &["use gpui_component::v_flex;"],
        extra_imports: &[],
        container: true,
        default_args: &[],
        default_calls: &[],
        props: &[
            Prop { label: "prop.gap", target: Target::Family(GAPS), kind: Kind::Choice },
            Prop { label: "prop.padding", target: Target::Family(PADDINGS), kind: Kind::Choice },
            Prop { label: "prop.align", target: Target::Family(ALIGNS), kind: Kind::Choice },
            Prop { label: "prop.flex", target: Target::Flag("flex_1"), kind: Kind::Bool },
            Prop {
                label: "prop.scroll",
                target: Target::Scrollable("overflow_y_scroll"),
                kind: Kind::Bool,
            },
        ],
        common: Common::All,
        handler: None,
        state: None,
    },
    Spec {
        id: "row",
        label: "component.row",
        base: "h_flex",
        imports: &["use gpui_component::h_flex;"],
        extra_imports: &[],
        container: true,
        default_args: &[],
        default_calls: &[],
        props: &[
            Prop { label: "prop.gap", target: Target::Family(GAPS), kind: Kind::Choice },
            Prop { label: "prop.padding", target: Target::Family(PADDINGS), kind: Kind::Choice },
            Prop { label: "prop.align", target: Target::Family(ALIGNS), kind: Kind::Choice },
            Prop { label: "prop.flex", target: Target::Flag("flex_1"), kind: Kind::Bool },
            Prop {
                label: "prop.scroll",
                target: Target::Scrollable("overflow_x_scroll"),
                kind: Kind::Bool,
            },
        ],
        common: Common::All,
        handler: None,
        state: None,
    },
    Spec {
        id: "label",
        label: "component.label",
        base: "Label::new",
        imports: &["use gpui_component::label::Label;"],
        extra_imports: &[],
        container: false,
        default_args: &["Label"],
        default_calls: &[],
        props: &[Prop { label: "prop.text", target: Target::BaseArg(0), kind: Kind::Text }],
        common: Common::All,
        handler: None,
        state: None,
    },
    Spec {
        id: "input",
        label: "component.input",
        base: "Input::new",
        imports: &["use gpui_component::input::Input;"],
        extra_imports: &[],
        container: false,
        default_args: &[],
        default_calls: &[],
        props: &[Prop { label: "prop.bound_field", target: Target::BaseArg(0), kind: Kind::Field }],
        common: Common::All,
        handler: None,
        state: Some(StateSpec {
            ty: "Entity<InputState>",
            imports: &["use gpui::Entity;", "use gpui_component::input::InputState;"],
            initializer: "cx.new(|cx| InputState::new(window, cx))",
        }),
    },
    Spec {
        id: "select",
        label: "component.select",
        base: "Select::new",
        imports: &["use gpui_component::select::Select;"],
        extra_imports: &[],
        container: false,
        default_args: &[],
        default_calls: &[],
        props: &[Prop { label: "prop.bound_field", target: Target::BaseArg(0), kind: Kind::Field }],
        // The list's contents live in the initializer, so in the code you edit
        // by hand: maxx puts two entries there so that something shows, and does
        // not pretend to manage where the data comes from.
        common: Common::All,
        handler: None,
        state: Some(StateSpec {
            ty: "Entity<SelectState<SearchableVec<SharedString>>>",
            imports: &[
                "use gpui::Entity;",
                "use gpui::SharedString;",
                "use gpui_component::IndexPath;",
                "use gpui_component::select::{SearchableVec, SelectState};",
            ],
            initializer: "cx.new(|cx| {\n                SelectState::new(\n                    SearchableVec::new(vec![\n                        SharedString::from(\"First\"),\n                        SharedString::from(\"Second\"),\n                    ]),\n                    Some(IndexPath::new(0)),\n                    window,\n                    cx,\n                )\n            })",
        }),
    },
    Spec {
        id: "button",
        label: "component.button",
        base: "Button::new",
        // The variants come from a trait, and a trait has to be in scope: with
        // `Button` alone, `.primary()` is a method the generated project does
        // not have, and it says so only when the developer builds it.
        imports: &["use gpui_component::button::Button;"],
        extra_imports: &[
            (VARIANTS, "use gpui_component::button::ButtonVariants;"),
            (&["disabled"], "use gpui_component::Disableable;"),
        ],
        container: false,
        default_args: &["button"],
        default_calls: &[],
        props: &[
            Prop { label: "prop.id", target: Target::BaseArg(0), kind: Kind::Text },
            Prop { label: "prop.label", target: Target::Method("label"), kind: Kind::Text },
            Prop { label: "prop.variant", target: Target::Family(VARIANTS), kind: Kind::Choice },
            Prop { label: "prop.tooltip", target: Target::Method("tooltip"), kind: Kind::Text },
            Prop { label: "prop.disabled", target: Target::Method("disabled"), kind: Kind::Bool },
            Prop { label: "prop.action", target: Target::Method("on_click"), kind: Kind::Handler },
        ],
        common: Common::All,
        handler: Some(CLICK),
        state: None,
    },
    Spec {
        id: "checkbox",
        label: "component.checkbox",
        base: "Checkbox::new",
        imports: &["use gpui_component::checkbox::Checkbox;"],
        extra_imports: &[],
        container: false,
        default_args: &["checkbox"],
        default_calls: &[],
        props: &[
            Prop { label: "prop.id", target: Target::BaseArg(0), kind: Kind::Text },
            Prop { label: "prop.label", target: Target::Method("label"), kind: Kind::Text },
            Prop { label: "prop.checked", target: Target::Method("checked"), kind: Kind::Bool },
            Prop { label: "prop.action", target: Target::Method("on_click"), kind: Kind::Handler },
        ],
        common: Common::All,
        handler: Some(TOGGLED),
        state: None,
    },
    Spec {
        id: "switch",
        label: "component.switch",
        base: "Switch::new",
        imports: &["use gpui_component::switch::Switch;"],
        extra_imports: &[],
        container: false,
        default_args: &["switch"],
        default_calls: &[],
        props: &[
            Prop { label: "prop.id", target: Target::BaseArg(0), kind: Kind::Text },
            Prop { label: "prop.label", target: Target::Method("label"), kind: Kind::Text },
            Prop { label: "prop.on", target: Target::Method("checked"), kind: Kind::Bool },
            Prop { label: "prop.action", target: Target::Method("on_click"), kind: Kind::Handler },
        ],
        common: Common::All,
        handler: Some(TOGGLED),
        state: None,
    },
    Spec {
        id: "group_box",
        label: "component.group_box",
        base: "GroupBox::new",
        imports: &["use gpui_component::group_box::GroupBox;"],
        extra_imports: &[],
        container: true,
        default_args: &[],
        default_calls: &[],
        props: &[Prop { label: "prop.title", target: Target::Method("title"), kind: Kind::Text }],
        common: Common::All,
        handler: None,
        state: None,
    },
    Spec {
        id: "divider",
        label: "component.divider",
        base: "Divider::horizontal",
        imports: &["use gpui_component::divider::Divider;"],
        extra_imports: &[],
        container: false,
        default_args: &[],
        default_calls: &[],
        props: &[Prop { label: "prop.label", target: Target::Method("label"), kind: Kind::Text }],
        common: Common::All,
        handler: None,
        state: None,
    },
    Spec {
        id: "radio",
        label: "component.radio",
        base: "Radio::new",
        imports: &["use gpui_component::radio::Radio;"],
        extra_imports: &[(&["disabled"], "use gpui_component::Disableable;")],
        container: false,
        default_args: &["radio"],
        default_calls: &[],
        props: &[
            Prop { label: "prop.id", target: Target::BaseArg(0), kind: Kind::Text },
            Prop { label: "prop.label", target: Target::Method("label"), kind: Kind::Text },
            Prop { label: "prop.selected", target: Target::Method("checked"), kind: Kind::Bool },
            Prop { label: "prop.disabled", target: Target::Method("disabled"), kind: Kind::Bool },
            Prop { label: "prop.action", target: Target::Method("on_click"), kind: Kind::Handler },
        ],
        common: Common::All,
        handler: Some(TOGGLED),
        state: None,
    },
    Spec {
        id: "link",
        label: "component.link",
        base: "Link::new",
        imports: &["use gpui_component::link::Link;"],
        extra_imports: &[(&["disabled"], "use gpui_component::Disableable;")],
        // `Link` is a `ParentElement`: its text is a child, not an argument. A
        // label dropped inside it is what writes it.
        container: true,
        default_args: &["link"],
        default_calls: &[],
        props: &[
            Prop { label: "prop.id", target: Target::BaseArg(0), kind: Kind::Text },
            Prop { label: "prop.href", target: Target::Method("href"), kind: Kind::Text },
            Prop { label: "prop.disabled", target: Target::Method("disabled"), kind: Kind::Bool },
        ],
        common: Common::All,
        handler: None,
        state: None,
    },
    Spec {
        id: "alert",
        label: "component.alert",
        base: "Alert::new",
        imports: &["use gpui_component::alert::Alert;"],
        extra_imports: &[],
        container: false,
        default_args: &["alert", "Message"],
        default_calls: &[],
        props: &[
            Prop { label: "prop.id", target: Target::BaseArg(0), kind: Kind::Text },
            Prop { label: "prop.message", target: Target::BaseArg(1), kind: Kind::Text },
            Prop { label: "prop.title", target: Target::Method("title"), kind: Kind::Text },
        ],
        common: Common::All,
        handler: None,
        state: None,
    },
    Spec {
        id: "tag",
        label: "component.tag",
        base: "Tag::new",
        imports: &["use gpui_component::tag::Tag;"],
        extra_imports: &[(&["with_variant"], "use gpui_component::tag::TagVariant;")],
        container: true,
        default_args: &[],
        default_calls: &[],
        props: &[
            Prop {
                label: "prop.variant",
                target: Target::Variant("with_variant", TAG_VARIANTS),
                kind: Kind::Choice,
            },
            Prop { label: "prop.outline", target: Target::Flag("outline"), kind: Kind::Bool },
            Prop {
                label: "prop.rounded_full",
                target: Target::Flag("rounded_full"),
                kind: Kind::Bool,
            },
        ],
        common: Common::All,
        handler: None,
        state: None,
    },
    Spec {
        id: "progress",
        label: "component.progress",
        base: "Progress::new",
        imports: &["use gpui_component::progress::Progress;"],
        extra_imports: &[],
        container: false,
        default_args: &[],
        default_calls: &[],
        props: &[Prop { label: "prop.value", target: Target::Method("value"), kind: Kind::Ratio }],
        common: Common::All,
        handler: None,
        state: None,
    },
    Spec {
        id: "image",
        label: "component.image",
        base: "img",
        imports: &["use gpui::img;"],
        extra_imports: &[],
        container: false,
        default_args: &["assets/images/image.png"],
        // A photograph is two thousand pixels wide, and a view is five hundred:
        // dropped as it is, the first image pushes everything else off the
        // board. Fitting is what one wants nine times out of ten, and the
        // switch is right there for the tenth.
        default_calls: &["max_w_full"],
        props: &[
            Prop { label: "prop.source", target: Target::BaseArg(0), kind: Kind::Path },
            // Two questions, and they are not the same one: how the box relates
            // to what holds it, and how the picture fills the box. The second
            // only has anything to say once the first has bounded something.
            Prop { label: "prop.size", target: Target::Family(IMAGE_SIZES), kind: Kind::Choice },
            Prop {
                label: "prop.fit",
                target: Target::Variant("object_fit", OBJECT_FITS),
                kind: Kind::Choice,
            },
        ],
        // An image draws no text of its own: a font weight on it is a row that
        // does nothing.
        common: Common::Box,
        handler: None,
        state: None,
    },
    Spec {
        id: "slider",
        label: "component.slider",
        base: "Slider::new",
        imports: &["use gpui_component::slider::Slider;"],
        extra_imports: &[],
        container: false,
        default_args: &[],
        default_calls: &[],
        props: &[
            Prop { label: "prop.bound_field", target: Target::BaseArg(0), kind: Kind::Field },
            Prop { label: "prop.axis", target: Target::Family(SLIDER_AXES), kind: Kind::Choice },
            Prop { label: "prop.disabled", target: Target::Method("disabled"), kind: Kind::Bool },
        ],
        // No text of its own, so no text size and no weight — five rows that do
        // nothing would drown the three that matter.
        common: Common::Box,
        handler: None,
        // The bounds live in the initializer, so in the code you edit by hand:
        // maxx writes a nought-to-a-hundred slider so that something moves, and
        // does not pretend to own the range.
        state: Some(StateSpec {
            ty: "Entity<SliderState>",
            imports: &["use gpui::Entity;", "use gpui_component::slider::SliderState;"],
            initializer: "cx.new(|_| SliderState::new().min(0.).max(100.).step(1.).default_value(50.))",
        }),
    },
    Spec {
        id: "color_picker",
        label: "component.color_picker",
        base: "ColorPicker::new",
        imports: &["use gpui_component::color_picker::ColorPicker;"],
        extra_imports: &[],
        container: false,
        default_args: &[],
        default_calls: &[],
        props: &[
            Prop { label: "prop.bound_field", target: Target::BaseArg(0), kind: Kind::Field },
            Prop { label: "prop.label", target: Target::Method("label"), kind: Kind::Text },
        ],
        common: Common::All,
        handler: None,
        state: Some(StateSpec {
            ty: "Entity<ColorPickerState>",
            imports: &["use gpui::Entity;", "use gpui_component::color_picker::ColorPickerState;"],
            initializer: "cx.new(|cx| ColorPickerState::new(window, cx))",
        }),
    },
    Spec {
        id: "skeleton",
        label: "component.skeleton",
        base: "Skeleton::new",
        imports: &["use gpui_component::skeleton::Skeleton;"],
        extra_imports: &[],
        container: false,
        default_args: &[],
        // A placeholder with no height of its own is a placeholder nobody sees.
        default_calls: &["h_4"],
        props: &[Prop {
            label: "prop.secondary",
            target: Target::Flag("secondary"),
            kind: Kind::Bool,
        }],
        common: Common::Box,
        handler: None,
        state: None,
    },
    Spec {
        id: "spinner",
        label: "component.spinner",
        base: "Spinner::new",
        imports: &["use gpui_component::spinner::Spinner;"],
        extra_imports: &[],
        container: false,
        default_args: &[],
        default_calls: &[],
        props: &[],
        // `Spinner` does not implement `Styled`: the shared style calls would
        // not compile on it, and the developer would find out, not maxx.
        common: Common::None,
        handler: None,
        state: None,
    },
    Spec {
        id: "badge",
        label: "component.badge",
        base: "Badge::new",
        imports: &["use gpui_component::badge::Badge;"],
        extra_imports: &[],
        // It wraps what it marks — an icon, a button — rather than standing on
        // its own.
        container: true,
        default_args: &[],
        default_calls: &[],
        props: &[
            Prop { label: "prop.count", target: Target::Method("count"), kind: Kind::Count },
            Prop { label: "prop.max", target: Target::Method("max"), kind: Kind::Count },
        ],
        // `Badge` does not implement `Styled` either. This is the case the flag
        // was made for.
        common: Common::None,
        handler: None,
        state: None,
    },
    Spec {
        id: "icon",
        label: "component.icon",
        base: "Icon::new",
        // Both always: the variant is the constructor's argument, so it is
        // there from the moment the icon is.
        imports: &["use gpui_component::{Icon, IconName};"],
        extra_imports: &[],
        container: false,
        default_args: &["IconName::Check"],
        default_calls: &[],
        props: &[
            Prop { label: "prop.icon", target: Target::VariantArg(0, ICONS), kind: Kind::Choice },
            // An icon's colour is its text colour: it is drawn as an `svg`
            // taking `text_color`, which is why it is here rather than among
            // the shared text properties an icon has no use for.
            Prop {
                label: "prop.text_color",
                target: Target::Method("text_color"),
                kind: Kind::Color,
            },
        ],
        common: Common::Box,
        handler: None,
        state: None,
    },
    Spec {
        id: "spacer",
        label: "component.spacer",
        base: "div",
        imports: &["use gpui::div;"],
        extra_imports: &[],
        container: false,
        default_args: &[],
        default_calls: &[],
        props: &[Prop { label: "prop.flex", target: Target::Flag("flex_1"), kind: Kind::Bool }],
        common: Common::All,
        handler: None,
        state: None,
    },
];

/// `120` becomes `px(120.)`, `12.5` becomes `px(12.5)`.
fn pixel_literal(value: &str) -> Option<String> {
    Some(format!("px({})", float_literal(value)?))
}

/// A whole number, as a `usize` literal — or nothing, for anything else.
///
/// What a badge's counts are, and why they cannot go through
/// [`float_literal`]: `.count(3.)` does not compile.
fn whole_literal(value: &str) -> Option<String> {
    let value = value.trim();
    if value.is_empty() || !value.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }
    value.parse::<usize>().ok().map(|number| number.to_string())
}

/// `120` and `12.5` become Rust float literals; `.5`, `inf` and `NaN` are
/// refused.
///
/// `f32::from_str` accepts spellings `rustc` does not, and emitting one leaves
/// the generated project unbuildable.
fn float_literal(value: &str) -> Option<String> {
    let value = value.trim();
    let digits = value.strip_prefix('-').unwrap_or(value);
    if digits.is_empty() || !digits.starts_with(|c: char| c.is_ascii_digit()) {
        return None;
    }
    if !digits.chars().all(|c| c.is_ascii_digit() || c == '.') || digits.matches('.').count() > 1 {
        return None;
    }
    let number: f32 = value.parse().ok()?;
    if !number.is_finite() {
        return None;
    }
    Some(if value.contains('.') { value.to_string() } else { format!("{value}.") })
}

/// The translation key of why a value was refused, for the inspector to say so.
///
/// `write` silently ignores what it cannot encode — which is the right
/// behaviour for the file, and the wrong one for the person typing.
pub fn validate(prop: &Prop, value: &str) -> Option<&'static str> {
    let value = value.trim();
    match prop.kind {
        Kind::Number if !value.is_empty() && pixel_literal(value).is_none() => Some("error.length"),
        Kind::Ratio if !value.is_empty() && float_literal(value).is_none() => Some("error.number"),
        Kind::Color => {
            let hex = value.trim_start_matches('#');
            if hex.is_empty() || (hex.len() == 6 && hex.chars().all(|c| c.is_ascii_hexdigit())) {
                None
            } else {
                Some("error.colour")
            }
        }
        Kind::Field | Kind::Handler if !value.is_empty() && !is_identifier(value) => {
            Some("error.identifier")
        }
        // Relative to the project root, or the image stops being found the day
        // the project moves — and it never showed on anybody else's machine.
        Kind::Path if leaves_the_project(value) => Some("error.path_relative"),
        _ => None,
    }
}

/// The state field a text property reads, when it reads one instead of a
/// literal.
pub fn read_binding(node: &Node, prop: &Prop) -> Option<String> {
    if !matches!(prop.kind, Kind::Text) {
        return None;
    }
    let source = match prop.target {
        Target::BaseArg(index) => match &node.base {
            Base::Known { args, .. } => args.get(index)?.to_source(),
            Base::Opaque(_) => return None,
        },
        Target::Method(name) => node.call(name)?.args.first()?.to_source(),
        _ => return None,
    };
    binding_field(&source)
}

/// `self.titre.clone()` and `self.clics.to_string()` both read `titre`/`clics`.
fn binding_field(source: &str) -> Option<String> {
    let inner = source.strip_prefix("self.")?;
    let name = inner.strip_suffix(".clone()").or_else(|| inner.strip_suffix(".to_string()"))?;
    is_identifier(name).then(|| name.to_string())
}

/// Writes a text property as an expression reading the view's state, or back to
/// a literal when `expression` is `None`.
pub fn write_binding(node: &mut Node, prop: &Prop, expression: Option<&str>) {
    let arg = match expression {
        Some(expression) => Arg::Verbatim(expression.to_string()),
        None => Arg::Str(String::new()),
    };
    match prop.target {
        Target::BaseArg(index) => {
            if let Base::Known { args, .. } = &mut node.base {
                if index < args.len() {
                    args[index] = arg;
                } else {
                    args.push(arg);
                }
            }
        }
        Target::Method(name) => node.set_call(name, arg),
        _ => {}
    }
}

/// Every property of a component: its own, then the shared style ones.
pub fn props(spec: &'static Spec) -> Vec<&'static Prop> {
    let shared: &[Prop] = match spec.common {
        Common::All | Common::Box => COMMON,
        Common::None => &[],
    };
    let text: &[Prop] = match spec.common {
        Common::All => TEXT_COMMON,
        _ => &[],
    };
    spec.props.iter().chain(shared).chain(text).collect()
}

/// Whether any property of `spec` owns the call named `name`.
///
/// What is not owned is shown as-is in the inspector's "other calls" section
/// rather than being hidden: the model carries every call, so the panel should
/// too.
pub fn covers(spec: &'static Spec, name: &str) -> bool {
    props(spec).into_iter().any(|prop| match prop.target {
        Target::BaseArg(_) | Target::VariantArg(..) => false,
        Target::Method(method) | Target::Flag(method) => method == name,
        Target::Family(names) => names.contains(&name),
        Target::Variant(method, _) => method == name,
        // The overflow only. The hold and the id are left visible among the
        // other calls: maxx writes them, but it cannot prove it wrote *this*
        // one — a `h_full` may well be the developer's own layout — and hiding
        // a call it might then delete is how a hand-written line disappears
        // without anyone seeing it go.
        Target::Scrollable(method) => method == name,
    })
}

/// The call that holds the axis a scrolling box scrolls on.
///
/// Nothing scrolls inside a box whose size follows its own content: it grows
/// instead, and the window cuts it. The property owns this call as much as the
/// overflow one — which is why it removes it too, and why the inspector shows
/// neither among the calls it does not know.
fn hold_for(axis: &str) -> &'static str {
    if axis == "overflow_x_scroll" { "w_full" } else { "h_full" }
}

/// An element id no node of `root` is already using.
///
/// gpui keeps a scroll offset per element id, and two siblings sharing one is a
/// conflict the framework catches at the worst moment. Walking the whole tree
/// rather than the siblings is the cheap answer, and it survives a node being
/// dragged somewhere else.
pub fn unique_element_id(root: &Node) -> String {
    let mut taken = Vec::new();
    root.walk(&mut |_, node| {
        if let Some(call) = node.call("id")
            && let Some(value) = call.args.first().and_then(|arg| arg.as_str())
        {
            taken.push(value.to_string());
        }
        // A button, a checkbox, a switch carry theirs as a constructor
        // argument: looking only at the `id` call would hand out an id one of
        // them already answers to, which is the collision this exists to
        // prevent.
        if let Some(spec) = of(node) {
            for prop in spec.props {
                if let (Target::BaseArg(index), "prop.id") = (prop.target, prop.label)
                    && let Base::Known { args, .. } = &node.base
                    && let Some(value) = args.get(index).and_then(|arg| arg.as_str())
                {
                    taken.push(value.to_string());
                }
            }
        }
    });
    let mut name = "scroll".to_string();
    let mut index = 2;
    while taken.contains(&name) {
        name = format!("scroll_{index}");
        index += 1;
    }
    name
}

/// The catalogue entry with this identifier.
pub fn by_id(id: &str) -> Option<&'static Spec> {
    CATALOGUE.iter().find(|spec| spec.id == id)
}

/// The catalogue entry a node was built from, matched on its constructor path.
pub fn of(node: &Node) -> Option<&'static Spec> {
    let path = node.base.path()?;
    CATALOGUE.iter().find(|spec| spec.base == path)
}

/// Builds a fresh node for the component `id`.
pub fn instantiate(id: &str) -> Option<Node> {
    let spec = by_id(id)?;
    let args = if spec.state.is_some() {
        // A stateful component — a text input, a dropdown, a slider, a colour
        // picker — is not a value but an entity the view owns: it takes a
        // reference to a field, `view::save` declares it, and the caller gives
        // it a name no sibling is using.
        vec![Arg::Verbatim("&self.field".into())]
    } else {
        // Per argument, and not from the first property: a component whose
        // first argument is a path and whose second is a string — the shape
        // `alert` already has — would have had the second written as a path
        // too, and the generated project would not compile.
        spec.default_args
            .iter()
            .enumerate()
            .map(|(index, value)| {
                let target = spec.props.iter().find_map(|prop| match prop.target {
                    Target::BaseArg(at) if at == index => Some((prop.target, prop.kind)),
                    Target::VariantArg(at, _) if at == index => Some((prop.target, prop.kind)),
                    _ => None,
                });
                match target {
                    // The encoder is the one `write` uses, so a fresh node and
                    // an edited one hold the same shape.
                    Some((_, Kind::Path)) => path_arg(None, value),
                    // A variant is a path, not a string: `Icon::new("IconName::Check")`
                    // does not compile.
                    Some((Target::VariantArg(..), _)) => Arg::Verbatim((*value).into()),
                    _ => Arg::Str((*value).into()),
                }
            })
            .collect()
    };

    let mut node = Node::known(spec.base);
    if let Base::Known { args: slot, .. } = &mut node.base {
        *slot = args;
    }
    match spec.id {
        "button" => node.set_call("label", Arg::Str("Button".into())),
        "checkbox" => node.set_call("label", Arg::Str("Checkbox".into())),
        "switch" => node.set_call("label", Arg::Str("Switch".into())),
        "radio" => node.set_call("label", Arg::Str("Radio".into())),
        "group_box" => node.set_call("title", Arg::Str("Group".into())),
        "alert" => node.set_call("title", Arg::Str("Note".into())),
        // A spacer with no `flex_1` takes no room and cannot be found again.
        "spacer" => node.set_flag("flex_1", true),
        _ => {}
    }
    for call in spec.default_calls {
        node.set_flag(call, true);
    }
    Some(node)
}

/// Whether a valid Rust identifier.
fn is_identifier(value: &str) -> bool {
    let mut chars = value.chars();
    matches!(chars.next(), Some(first) if first == '_' || first.is_alphabetic())
        && chars.all(|character| character == '_' || character.is_alphanumeric())
}

/// Whether the inspector may edit this property of this node.
///
/// A base argument that is not a string literal is a hand-written expression —
/// `Button::new(cx.entity_id())`. Overwriting it with a string literal on the
/// first keystroke would silently change what the code means, so it is shown
/// but not edited.
pub fn editable(node: &Node, prop: &Prop) -> bool {
    match (prop.target, prop.kind) {
        (Target::Method(_), Kind::Number | Kind::Color | Kind::Ratio) => !node.is_opaque(),
        (Target::Method(name), Kind::Text) => match node.call(name).and_then(|c| c.args.first()) {
            // A literal, or nothing yet: free to type in.
            None | Some(Arg::Str(_)) => !node.is_opaque(),
            // Anything else is an expression someone wrote; the binding button
            // handles the shapes maxx knows, and the rest is left alone.
            Some(_) => false,
        },
        (Target::BaseArg(index), Kind::Path) => match &node.base {
            Base::Known { args, .. } => match args.get(index) {
                // Nothing yet, maxx's own string, or the older `PathBuf::from`
                // spelling — anything else is an expression someone wrote,
                // `img(self.avatar.clone())` among them.
                None | Some(Arg::Str(_)) => true,
                Some(Arg::Verbatim(source)) => path_value(source).is_some(),
                Some(_) => false,
            },
            Base::Opaque(_) => false,
        },
        (Target::BaseArg(index), Kind::Text) => match &node.base {
            Base::Known { args, .. } => match args.get(index) {
                None | Some(Arg::Str(_)) => true,
                Some(_) => false,
            },
            Base::Opaque(_) => false,
        },
        (Target::Method(name), Kind::Handler) => match node.call(name) {
            Some(call) => {
                call.args.first().is_some_and(|arg| handler_name(&arg.to_source()).is_some())
            }
            None => true,
        },
        _ => !node.is_opaque(),
    }
}

/// `px(240.)` reads back as `240`.
fn number_value(source: &str) -> Option<String> {
    let inner = source.strip_prefix("px(")?.strip_suffix(')')?;
    Some(inner.trim_end_matches('.').to_string())
}

/// `PathBuf::from("assets/logo.png")` reads back as `assets/logo.png`.
///
/// `None` for anything else, which is what tells [`editable`] that the argument
/// is a hand-written expression the inspector must not overwrite.
///
/// Only older projects hold this shape — maxx writes a bare string now — but
/// they hold it for good, so it has to keep reading back.
fn path_value(source: &str) -> Option<String> {
    let inner = source.strip_prefix("PathBuf::from(\"")?.strip_suffix("\")")?;
    // The exact inverse of [`crate::model::escape`], sequence for sequence.
    // Undoing `\\` alone left `\"` behind, and the next write escaped its
    // backslash again: the argument grew by one on every keystroke. Taking the
    // character after a backslash literally was no better — `\t` came back as
    // the letter `t`, and a file whose name holds a tab stopped loading with
    // nothing said.
    let mut out = String::with_capacity(inner.len());
    let mut chars = inner.chars();
    while let Some(character) = chars.next() {
        match character {
            '\\' => match chars.next()? {
                'n' => out.push('\n'),
                't' => out.push('\t'),
                'r' => out.push('\r'),
                escaped => out.push(escaped),
            },
            _ => out.push(character),
        }
    }
    Some(out)
}

/// The argument a path is written as, in the form the node already had.
///
/// A string for anything maxx writes today, because that is what reaches the
/// `AssetSource`; `PathBuf::from("…")` for a node that already held one, since
/// the two spellings do not mean the same thing at runtime and flipping an
/// existing one would change what the project does — and leave the file with a
/// `use std::path::PathBuf;` nothing uses, which [`imports`] adds and never
/// prunes.
fn path_arg(existing: Option<&Arg>, value: &str) -> Arg {
    match existing {
        Some(Arg::Verbatim(source)) if path_value(source).is_some() => {
            Arg::Verbatim(format!("PathBuf::from(\"{}\")", crate::model::escape(value)))
        }
        _ => Arg::Str(value.to_string()),
    }
}

/// Whether this path would only resolve on the machine it was typed on.
///
/// Absolute, root-relative, or climbing out of the project with `..`: all three
/// draw here and nowhere else, which is the whole reason the property refuses
/// anything but a path relative to the root.
fn leaves_the_project(value: &str) -> bool {
    let value = value.replace('\\', "/");
    value.starts_with('/')
        || matches!(value.as_bytes(), [drive, b':', ..] if drive.is_ascii_alphabetic())
        || value.split('/').any(|part| part == "..")
}

/// `rgb(0x1e2127)` reads back as `1e2127`.
fn color_value(source: &str) -> Option<String> {
    let inner = source.strip_prefix("rgb(0x")?.strip_suffix(')')?;
    Some(inner.to_string())
}

/// The method name inside `cx.listener(Self::<name>)`, if that is the shape.
///
/// Anything else — a closure written by hand, a call to something else — is
/// left alone: the inspector shows it and refuses to rewrite it.
pub fn handler_name(source: &str) -> Option<String> {
    let inner = source.strip_prefix("cx.listener(Self::")?.strip_suffix(')')?;
    is_identifier(inner).then(|| inner.to_string())
}

/// Every handler method the tree refers to, in tree order.
pub fn handlers(root: &Node) -> Vec<(String, HandlerSpec)> {
    let mut names: Vec<(String, HandlerSpec)> = Vec::new();
    root.walk(&mut |_, node| {
        // The shape comes from the component the call sits on, not from the
        // call's name: `on_click` means a `&ClickEvent` on a button and a
        // `&bool` on a switch.
        let shape = of(node).and_then(|spec| spec.handler).unwrap_or(CLICK);
        for call in &node.calls {
            if let Some(arg) = call.args.first()
                && let Some(name) = handler_name(&arg.to_source())
                && !names.iter().any(|(known, _)| *known == name)
            {
                names.push((name, shape));
            }
        }
    });
    names
}

/// A handler name derived from a node, e.g. the button `valider` gives
/// `on_valider`.
pub fn suggested_handler(node: &Node) -> String {
    let base = match &node.base {
        Base::Known { args, .. } => {
            args.first().and_then(|arg| arg.as_str()).unwrap_or("action").to_string()
        }
        Base::Opaque(_) => "action".to_string(),
    };
    let cleaned: String =
        base.chars()
            .map(|character| {
                if character.is_ascii_alphanumeric() { character.to_ascii_lowercase() } else { '_' }
            })
            .collect();
    format!("on_{}", if cleaned.is_empty() { "action" } else { &cleaned })
}

/// A field name not already bound by another input in the tree.
/// Renames the bindings of `subtree` that would collide with `root`'s.
///
/// Two text inputs sharing `&self.field` compile and then mirror each other at
/// runtime — the same defect `insert_component` avoids when it drops a fresh
/// one. A copy carries the original's binding, so a duplicate always collides
/// and is always renamed; `view::save` then declares the new fields.
///
/// A binding that collides with nothing is left exactly as it is. That is what
/// an `Input::new(&self.search)` written by hand in Zed and pasted here keeps:
/// renaming it would contradict the promise that what is written there comes
/// back, and would declare a second field for the one it already has.
pub fn rebind_state_fields(subtree: &mut Node, root: &Node) {
    // Grown as we go: a name handed out here is not in `root`, and two inputs
    // of the same subtree must not be given the same one either.
    let mut taken = state_fields(root);

    fn walk(node: &mut Node, taken: &mut Vec<String>) {
        if of(node).is_some_and(|spec| spec.state.is_some())
            && let Base::Known { args, .. } = &mut node.base
        {
            let current = args
                .first()
                .map(|arg| arg.to_source())
                .and_then(|source| source.strip_prefix("&self.").map(str::to_string));
            match current {
                Some(name) if !taken.contains(&name) => taken.push(name),
                _ => {
                    let name = next_field(taken);
                    taken.push(name.clone());
                    let arg = Arg::Verbatim(format!("&self.{name}"));
                    match args.first_mut() {
                        Some(slot) => *slot = arg,
                        None => args.push(arg),
                    }
                }
            }
        }
        for child in &mut node.children {
            walk(child, taken);
        }
    }
    walk(subtree, &mut taken);
}

/// The names of the view fields every state-backed node of `root` binds to.
fn state_fields(root: &Node) -> Vec<String> {
    let mut used = Vec::new();
    root.walk(&mut |_, node| {
        if of(node).is_none_or(|spec| spec.state.is_none()) {
            return;
        }
        if let Base::Known { args, .. } = &node.base
            && let Some(name) = args
                .first()
                .map(|arg| arg.to_source())
                .and_then(|source| source.strip_prefix("&self.").map(str::to_string))
        {
            used.push(name);
        }
    });
    used
}

/// The first `field`, `field_2`, … that `taken` does not hold.
fn next_field(taken: &[String]) -> String {
    let mut index = 1;
    loop {
        let candidate = if index == 1 { "field".to_string() } else { format!("field_{index}") };
        if !taken.contains(&candidate) {
            return candidate;
        }
        index += 1;
    }
}

pub fn unique_input_field(root: &Node) -> String {
    next_field(&state_fields(root))
}

/// Whether the tree draws a file of the project as an asset.
///
/// The string spelling, and not the `PathBuf` one: a path is read from the disk
/// and needs nothing, an asset is looked up in a source the project has to
/// declare. This is what tells the workspace the assets module is now owed.
pub fn uses_an_asset(root: &Node) -> bool {
    let mut found = false;
    root.walk(&mut |_, node| {
        let Some(spec) = of(node) else {
            return;
        };
        for prop in spec.props {
            if let (Target::BaseArg(index), Kind::Path) = (prop.target, prop.kind)
                && let Base::Known { args, .. } = &node.base
                && matches!(args.get(index), Some(Arg::Str(_)))
            {
                found = true;
            }
        }
    });
    found
}

/// Reads the current value of a property, as text for the inspector.
pub fn read(node: &Node, prop: &Prop) -> Option<String> {
    match prop.target {
        Target::BaseArg(index) => match &node.base {
            Base::Known { args, .. } => args.get(index).map(|arg| match (prop.kind, arg) {
                (Kind::Field, Arg::Verbatim(source)) => {
                    source.trim_start_matches("&self.").to_string()
                }
                (Kind::Path, Arg::Verbatim(source)) => {
                    path_value(source).unwrap_or_else(|| source.clone())
                }
                (_, Arg::Str(value)) => value.clone(),
                (_, other) => other.to_source(),
            }),
            Base::Opaque(_) => None,
        },
        Target::Method(name)
            if matches!(prop.kind, Kind::Number | Kind::Color | Kind::Ratio | Kind::Count) =>
        {
            let source = node.call(name)?.args.first()?.to_source();
            match prop.kind {
                Kind::Number => number_value(&source),
                Kind::Ratio => Some(source.trim_end_matches('.').to_string()),
                Kind::Count => Some(source),
                _ => color_value(&source),
            }
        }
        Target::Variant(name, _) => {
            node.call(name).and_then(|call| call.args.first()).map(|arg| arg.to_source())
        }
        Target::VariantArg(index, _) => match &node.base {
            Base::Known { args, .. } => args.get(index).map(|arg| arg.to_source()),
            Base::Opaque(_) => None,
        },
        Target::Method(name) if matches!(prop.kind, Kind::Handler) => node
            .call(name)
            .and_then(|call| call.args.first())
            .map(|arg| arg.to_source())
            .map(|source| handler_name(&source).unwrap_or(source)),
        Target::Method(name) => match node.call(name) {
            Some(call) => call
                .args
                .first()
                .map(|arg| arg.as_str().map(str::to_string).unwrap_or(arg.to_source())),
            None if matches!(prop.kind, Kind::Bool) => Some("false".into()),
            None => None,
        },
        Target::Flag(name) | Target::Scrollable(name) => {
            Some(node.call(name).is_some().to_string())
        }
        Target::Family(names) => {
            names.iter().find(|name| node.call(name).is_some()).map(|name| (*name).to_string())
        }
    }
}

/// Writes a property. `value` is the raw text from the inspector; for a family,
/// it is the chosen method name, or empty to clear the choice.
pub fn write(node: &mut Node, prop: &Prop, value: &str) {
    match prop.target {
        Target::BaseArg(index) => {
            // An empty or malformed field name would be written straight into
            // the source as `&self.` and into the struct as `pub : Entity<..>`.
            if matches!(prop.kind, Kind::Field) && !is_identifier(value) {
                return;
            }
            // A path replaces maxx's own writing and nothing else: exempting
            // the guard below for the whole kind would let a keystroke
            // overwrite `img(self.avatar.clone())`.
            if matches!(prop.kind, Kind::Path)
                && (!editable(node, prop) || leaves_the_project(value))
            {
                return;
            }
            let Base::Known { args, .. } = &mut node.base else {
                return;
            };
            if !matches!(args.get(index), None | Some(Arg::Str(_)))
                && !matches!(prop.kind, Kind::Field | Kind::Path)
            {
                return;
            }
            let arg = match prop.kind {
                Kind::Field => Arg::Verbatim(format!("&self.{value}")),
                Kind::Path => path_arg(args.get(index), value),
                _ => Arg::Str(value.to_string()),
            };
            if index < args.len() {
                args[index] = arg;
            } else {
                args.push(arg);
            }
        }
        Target::Method(name) if matches!(prop.kind, Kind::Number) => {
            if value.trim().is_empty() {
                node.remove_call(name);
            } else if let Some(literal) = pixel_literal(value) {
                node.set_call(name, Arg::Verbatim(literal));
            }
        }
        Target::Method(name) if matches!(prop.kind, Kind::Ratio) => {
            if value.trim().is_empty() {
                node.remove_call(name);
            } else if let Some(literal) = float_literal(value) {
                node.set_call(name, Arg::Verbatim(literal));
            }
        }
        Target::Method(name) if matches!(prop.kind, Kind::Count) => {
            if value.trim().is_empty() {
                node.remove_call(name);
            } else if let Some(literal) = whole_literal(value) {
                node.set_call(name, Arg::Verbatim(literal));
            }
        }
        Target::Method(name) if matches!(prop.kind, Kind::Color) => {
            let hex = value.trim().trim_start_matches('#');
            if hex.is_empty() {
                node.remove_call(name);
            } else if hex.len() == 6 && hex.chars().all(|c| c.is_ascii_hexdigit()) {
                node.set_call(name, Arg::Verbatim(format!("rgb(0x{hex})")));
            }
        }
        Target::Method(name) if matches!(prop.kind, Kind::Handler) => {
            if value.is_empty() {
                node.remove_call(name);
            } else if is_identifier(value) {
                node.set_call(name, Arg::Verbatim(format!("cx.listener(Self::{value})")));
            }
        }
        Target::Method(name) => {
            let arg = match prop.kind {
                Kind::Bool => Arg::Bool(value == "true"),
                _ => Arg::Str(value.to_string()),
            };
            node.set_call(name, arg);
        }
        Target::Variant(name, values) => {
            if value.is_empty() {
                node.remove_call(name);
            } else if values.contains(&value) {
                node.set_call(name, Arg::Verbatim(value.to_string()));
            }
        }
        // No empty case: the argument is what the constructor takes, and a
        // component without it does not compile. A value the table does not
        // know is refused rather than written.
        Target::VariantArg(index, values) => {
            if !values.contains(&value) {
                return;
            }
            let Base::Known { args, .. } = &mut node.base else {
                return;
            };
            let arg = Arg::Verbatim(value.to_string());
            if index < args.len() {
                args[index] = arg;
            } else {
                args.push(arg);
            }
        }
        Target::Flag(name) => node.set_flag(name, value == "true"),
        Target::Scrollable(name) => {
            let hold = hold_for(name);
            let size = if name == "overflow_x_scroll" { "w" } else { "h" };
            if value == "true" {
                // Before the overflow, and that is not a matter of taste:
                // `overflow_y_scroll` lives on a *stateful* element, so gpui
                // only offers it once `id` has been called. Written the other
                // way round, the chain does not compile — and only when the
                // developer builds their project.
                //
                // The id is also where gpui keeps the scroll offset: without
                // one, the content is clipped and never moves. The workspace
                // assigns one no sibling is using before it gets here; this is
                // the fallback for a node written without it.
                if node.call("id").is_none() {
                    node.set_call("id", Arg::Str("scroll".into()));
                }
            }
            node.set_flag(name, value == "true");
            if value == "true" {
                // And nothing scrolls inside a box whose size follows its own
                // content: it grows instead, and the window cuts it. The axis
                // that scrolls is the one that has to be held — unless a size
                // was set by hand, which says what to hold it to already.
                if node.call(size).is_none() && node.call("size_full").is_none() {
                    node.set_flag(hold, true);
                }
            }
            // Turning it off leaves both behind, and that is the lesser evil:
            // maxx cannot tell its own `h_full` from one written by hand, and
            // deleting a layout call nobody asked it to touch is worse than
            // leaving one that shows in the inspector, under its own name, for
            // whoever wants it gone.
        }
        Target::Family(names) => {
            for name in names {
                node.set_flag(name, false);
            }
            if !value.is_empty() {
                node.set_flag(value, true);
            }
        }
    }
}

/// Every `use` line the tree needs, in a stable order.
pub fn imports(root: &Node) -> Vec<&'static str> {
    let mut lines: Vec<&'static str> = Vec::new();
    root.walk(&mut |_, node| {
        if let Some(spec) = of(node) {
            for line in spec.imports {
                if !lines.contains(line) {
                    lines.push(line);
                }
            }
            for (calls, line) in spec.extra_imports {
                if calls.iter().any(|call| node.call(call).is_some()) && !lines.contains(line) {
                    lines.push(line);
                }
            }
        }
        // A path argument brings `PathBuf` with it, and it sits on the base
        // rather than on a call: an image is written `img(PathBuf::from(..))`.
        if let Base::Known { args, .. } = &node.base
            && args.iter().any(|arg| arg.to_source().starts_with("PathBuf::from("))
        {
            let line = "use std::path::PathBuf;";
            if !lines.contains(&line) {
                lines.push(line);
            }
        }
        // The style properties emit `px(..)` and `rgb(..)`, which are functions
        // of `gpui`, not methods of the component.
        for call in &node.calls {
            for arg in &call.args {
                let source = arg.to_source();
                for (needle, line) in [
                    ("px(", "use gpui::px;"),
                    ("rgb(0x", "use gpui::rgb;"),
                    ("ObjectFit::", "use gpui::ObjectFit;"),
                    ("FontWeight::", "use gpui::FontWeight;"),
                ] {
                    if source.starts_with(needle) && !lines.contains(&line) {
                        lines.push(line);
                    }
                }
            }
        }
    });
    lines.sort_unstable();
    lines
}
