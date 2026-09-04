//! The catalogue itself: every component maxx can drop, and the values its
//! properties accept.

use super::{CLICK, Common, Init, Kind, Prop, Spec, StateSpec, TOGGLED, Target};

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

/// Which bars a scrollbar draws.
const SCROLLBAR_AXES: &[&str] =
    &["ScrollbarAxis::Vertical", "ScrollbarAxis::Horizontal", "ScrollbarAxis::Both"];

/// The direction a slider runs in.
const SLIDER_AXES: &[&str] = &["horizontal", "vertical"];

/// The sizes an avatar comes in, as `Sizable` spells them.
///
/// Three and not four: `Sizable` names `xsmall`, `small` and `large`, and the
/// medium it starts at is what an empty choice already says — the same rule
/// every other family here follows.
const AVATAR_SIZES: &[&str] = &["xsmall", "small", "large"];

/// The variants of a tab bar, as `TabBar::with_variant` takes them.
///
/// The method rather than the four shorthands — `pill()`, `outline()` — for the
/// reason the tag's variants are written that way: a shorthand is a call of its
/// own, so changing the variant would leave the previous one behind.
const TAB_VARIANTS: &[&str] = &[
    "TabVariant::Tab",
    "TabVariant::Outline",
    "TabVariant::Pill",
    "TabVariant::Segmented",
    "TabVariant::Underline",
];

// The icons offered in the inspector, all eighty-six of them: `ICONS`.
//
// Generated rather than typed — `IconName` has no `FromStr`, no `Display` and
// no way to enumerate itself, so `build.rs` reads the enum out of the crate's
// own sources and writes both this list and the match `designer::canvas` draws
// from. Kept by hand, the two drifted: twenty-two icons were offered out of
// eighty-six, and the sixty-four missing ones were not missing on purpose.
//
// A plain comment and not a doc one: rustdoc has nothing to attach to a macro
// invocation, and says so as a warning.
include!(concat!(env!("OUT_DIR"), "/icons.rs"));

const ROUNDED: &[&str] =
    &["rounded_none", "rounded_sm", "rounded_md", "rounded_lg", "rounded_full"];

/// The margin ramp, on the same rungs as [`PADDINGS`].
///
/// The same numbers on purpose: gpui's scale runs 0, 0p5, 1, 1p5 … 96, and
/// offering all of it turns a choice into a search. Three families and not
/// seven — `m`, `mx`, `my` — because the four single sides double the rows of
/// the box heading to say what two of them already say, and a margin on one
/// side alone is the rarer half of an already rare property.
const MARGINS: &[&str] = &["m_0", "m_1", "m_2", "m_3", "m_4", "m_6", "m_8"];

/// The horizontal margin, on the same rungs.
const MARGINS_X: &[&str] = &["mx_0", "mx_1", "mx_2", "mx_3", "mx_4", "mx_6", "mx_8"];

/// The vertical margin, on the same rungs.
const MARGINS_Y: &[&str] = &["my_0", "my_1", "my_2", "my_3", "my_4", "my_6", "my_8"];

/// The border widths gpui draws on all four sides.
///
/// `border_3` exists too and is left out: a rung between two that are already a
/// hair apart, which is a row of the inspector spent on nothing.
const BORDERS: &[&str] = &["border_0", "border_1", "border_2", "border_4"];

/// The four shadows gpui names, and no more: `shadow` itself takes a vector of
/// `BoxShadow`, which is a value nobody types into a field.
const SHADOWS: &[&str] = &["shadow_sm", "shadow_md", "shadow_lg", "shadow_xl"];

/// How a flex container spreads its children along its own axis.
const JUSTIFIES: &[&str] =
    &["justify_start", "justify_center", "justify_end", "justify_between", "justify_around"];

/// Properties every component accepts.
///
/// Every widget of `gpui-component` implements `Styled`, so these are safe to
/// emit on any of them — checked component by component before adding them
/// here, because a style method on a type that does not implement `Styled`
/// would only fail when the generated project is compiled.
pub const COMMON: &[Prop] = &[
    Prop { label: "prop.width", target: Target::Method("w"), kind: Kind::Number },
    Prop { label: "prop.height", target: Target::Method("h"), kind: Kind::Number },
    Prop { label: "prop.min_width", target: Target::Method("min_w"), kind: Kind::Number },
    Prop { label: "prop.max_width", target: Target::Method("max_w"), kind: Kind::Number },
    Prop { label: "prop.min_height", target: Target::Method("min_h"), kind: Kind::Number },
    Prop { label: "prop.max_height", target: Target::Method("max_h"), kind: Kind::Number },
    Prop { label: "prop.margin", target: Target::Family(MARGINS), kind: Kind::Choice },
    Prop { label: "prop.margin_x", target: Target::Family(MARGINS_X), kind: Kind::Choice },
    Prop { label: "prop.margin_y", target: Target::Family(MARGINS_Y), kind: Kind::Choice },
    Prop { label: "prop.background", target: Target::Method("bg"), kind: Kind::Color },
    Prop { label: "prop.rounded", target: Target::Family(ROUNDED), kind: Kind::Choice },
    Prop { label: "prop.border", target: Target::Family(BORDERS), kind: Kind::Choice },
    Prop { label: "prop.border_color", target: Target::Method("border_color"), kind: Kind::Color },
    Prop { label: "prop.shadow", target: Target::Family(SHADOWS), kind: Kind::Choice },
    Prop { label: "prop.opacity", target: Target::Method("opacity"), kind: Kind::Ratio },
    Prop { label: "prop.clip", target: Target::Flag("overflow_hidden"), kind: Kind::Bool },
    Prop { label: "prop.cursor", target: Target::Flag("cursor_pointer"), kind: Kind::Bool },
];

/// The shared properties of a component that is a gpui element of its own.
///
/// Not in [`COMMON`], which every entry of the catalogue accepts: a tooltip
/// needs `StatefulInteractiveElement`, which a `div` has once it carries an
/// `id` and which no `gpui-component` widget has at all. Posed on everything,
/// it would be a call that does not compile — the defect this table exists to
/// prevent.
pub const INTERACTIVE: &[Prop] =
    &[Prop { label: "prop.tooltip", target: Target::Tooltip, kind: Kind::Text }];

/// What changes while the pointer is over the element.
///
/// Six and not twenty-five: "repeats the style properties" is the shape, not
/// the count — a hover that moves a margin or changes a font weight makes the
/// layout jump under the cursor, which is a thing to do on purpose and by hand.
/// What is here is what a hover is actually for: the colours, the depth, the
/// corner, the fade.
///
/// Each one wraps the very target its ordinary twin uses, so the two cannot
/// come to disagree about how a background is spelt.
///
/// On [`Common::Element`] alone, beside [`INTERACTIVE`]: `hover` is a method of
/// `InteractiveElement`, which a gpui `div` has and no `gpui-component` widget
/// does. A button already carries its own hover; a label written with one would
/// not compile.
pub const HOVER: &[Prop] = &[
    Prop {
        label: "prop.hover_background",
        target: Target::Hover(&Target::Method("bg")),
        kind: Kind::Color,
    },
    Prop {
        label: "prop.hover_text_color",
        target: Target::Hover(&Target::Method("text_color")),
        kind: Kind::Color,
    },
    Prop {
        label: "prop.hover_border_color",
        target: Target::Hover(&Target::Method("border_color")),
        kind: Kind::Color,
    },
    Prop {
        label: "prop.hover_opacity",
        target: Target::Hover(&Target::Method("opacity")),
        kind: Kind::Ratio,
    },
    Prop {
        label: "prop.hover_rounded",
        target: Target::Hover(&Target::Family(ROUNDED)),
        kind: Kind::Choice,
    },
    Prop {
        label: "prop.hover_shadow",
        target: Target::Hover(&Target::Family(SHADOWS)),
        kind: Kind::Choice,
    },
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
    Prop { label: "prop.line_height", target: Target::Method("line_height"), kind: Kind::Number },
    Prop { label: "prop.italic", target: Target::Flag("italic"), kind: Kind::Bool },
    Prop { label: "prop.underline", target: Target::Flag("underline"), kind: Kind::Bool },
    Prop { label: "prop.truncate", target: Target::Flag("truncate"), kind: Kind::Bool },
    Prop { label: "prop.nowrap", target: Target::Flag("whitespace_nowrap"), kind: Kind::Bool },
];

/// The two shapes a text input's state is built with.
///
/// The switch is in `new`, not in the region: `InputState::multi_line` is a
/// builder of the state the input is bound to, and the element takes no such
/// call. `{}` is the value — `true`, since a switch left off writes the other
/// shape rather than `multi_line(false)`, which would say the same thing in
/// more words.
const MULTI_LINE: Init = Init {
    off: Some("cx.new(|cx| InputState::new(window, cx))"),
    on: "cx.new(|cx| InputState::new(window, cx).multi_line({}))",
};

/// What a dropdown holds, where it is written: the initializer of its state.
///
/// The contents live in the code the developer edits — that is the point of
/// binding to a field rather than to a value — but the two names maxx puts
/// there on the first drop are ones nobody wants to keep, and reaching them
/// meant leaving the workshop. So the field writes back into the very line
/// `ensure_state_field` posted, and only into that one: an initializer that has
/// been changed no longer reads back, and is left as it stands.
///
/// No empty shape, and that is the reason [`Init::off`] is an option at all.
const SELECT_ITEMS: Init = Init {
    off: None,
    on: "cx.new(|cx| {\n                SelectState::new(\n                    SearchableVec::new(vec![\n                        {},\n                    ]),\n                    Some(IndexPath::new(0)),\n                    window,\n                    cx,\n                )\n            })",
};

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
        palette: true,
        default_args: &[],
        default_calls: &[],
        props: &[
            Prop { label: "prop.gap", target: Target::Family(GAPS), kind: Kind::Choice },
            Prop { label: "prop.padding", target: Target::Family(PADDINGS), kind: Kind::Choice },
            Prop { label: "prop.align", target: Target::Family(ALIGNS), kind: Kind::Choice },
            Prop { label: "prop.justify", target: Target::Family(JUSTIFIES), kind: Kind::Choice },
            Prop { label: "prop.flex", target: Target::Flag("flex_1"), kind: Kind::Bool },
            Prop { label: "prop.wrap", target: Target::Flag("flex_wrap"), kind: Kind::Bool },
            Prop {
                label: "prop.scroll",
                target: Target::Scrollable("overflow_y_scroll"),
                kind: Kind::Bool,
            },
            Prop { label: "prop.scrollbar", target: Target::Scrollbar, kind: Kind::Bool },
        ],
        common: Common::Element,
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
        palette: true,
        default_args: &[],
        default_calls: &[],
        props: &[
            Prop { label: "prop.gap", target: Target::Family(GAPS), kind: Kind::Choice },
            Prop { label: "prop.padding", target: Target::Family(PADDINGS), kind: Kind::Choice },
            Prop { label: "prop.align", target: Target::Family(ALIGNS), kind: Kind::Choice },
            Prop { label: "prop.justify", target: Target::Family(JUSTIFIES), kind: Kind::Choice },
            Prop { label: "prop.flex", target: Target::Flag("flex_1"), kind: Kind::Bool },
            Prop { label: "prop.wrap", target: Target::Flag("flex_wrap"), kind: Kind::Bool },
            Prop {
                label: "prop.scroll",
                target: Target::Scrollable("overflow_x_scroll"),
                kind: Kind::Bool,
            },
            Prop { label: "prop.scrollbar", target: Target::Scrollbar, kind: Kind::Bool },
        ],
        common: Common::Element,
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
        palette: true,
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
        palette: true,
        default_args: &[],
        default_calls: &[],
        props: &[
            Prop { label: "prop.bound_field", target: Target::BaseArg(0), kind: Kind::Field },
            // A property and not an entry of its own: `multi_line` is a builder
            // of `InputState`, and there is no `Input::multi_line` to write
            // beside it — one field, one state, one switch.
            Prop {
                label: "prop.multi_line",
                target: Target::Initializer(&MULTI_LINE),
                kind: Kind::Bool,
            },
        ],
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
        palette: true,
        default_args: &[],
        default_calls: &[],
        props: &[
            Prop { label: "prop.bound_field", target: Target::BaseArg(0), kind: Kind::Field },
            // The list's contents live in the initializer, so in the code the
            // developer edits — but the two names maxx put there are maxx's, and
            // this is what lets them be changed without leaving the workshop.
            Prop {
                label: "prop.items",
                target: Target::Initializer(&SELECT_ITEMS),
                kind: Kind::Text,
            },
        ],
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
        palette: true,
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
        palette: true,
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
        palette: true,
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
        palette: true,
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
        palette: true,
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
        palette: true,
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
        palette: true,
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
        palette: true,
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
        palette: true,
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
        palette: true,
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
        palette: true,
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
        id: "svg",
        label: "component.svg",
        base: "svg",
        imports: &["use gpui::svg;"],
        extra_imports: &[],
        container: false,
        palette: true,
        // The path is a call, not an argument: `svg()` takes none, and
        // `instantiate` is what seeds the two calls a drawing needs.
        default_args: &[],
        // A drawing with no size lays out as nothing at all — the same reason
        // the skeleton is given a height.
        default_calls: &["size_4"],
        props: &[
            // The same string an image writes, and read by the same
            // `AssetSource`: `Svg::path` hands its text to the application's
            // assets, exactly as `img("assets/…")` does.
            Prop { label: "prop.source", target: Target::Method("path"), kind: Kind::Path },
            // Not `TEXT_COMMON`, and this is the decision the backlog left
            // open. gpui tints an svg with `style.text.color` — it paints
            // *nothing at all* without one — so the colour has to be here; but
            // a font size, a weight, an underline on a drawing are seven rows
            // that do nothing. So: the box, plus the one text property that
            // means something. The very shape `icon` already has, for the very
            // same reason — an icon *is* an svg gpui tints.
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
        id: "slider",
        label: "component.slider",
        base: "Slider::new",
        imports: &["use gpui_component::slider::Slider;"],
        extra_imports: &[],
        container: false,
        palette: true,
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
        palette: true,
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
        id: "date_picker",
        label: "component.date_picker",
        // `gpui_component::date_picker`, not `time::date_picker`: the crate
        // keeps `time` private and re-exports the two modules from its root, so
        // the longer path does not resolve.
        base: "DatePicker::new",
        imports: &["use gpui_component::date_picker::DatePicker;"],
        extra_imports: &[(&["disabled"], "use gpui_component::Disableable;")],
        container: false,
        palette: true,
        default_args: &[],
        default_calls: &[],
        props: &[
            Prop { label: "prop.bound_field", target: Target::BaseArg(0), kind: Kind::Field },
            Prop {
                label: "prop.placeholder",
                target: Target::Method("placeholder"),
                kind: Kind::Text,
            },
            Prop { label: "prop.disabled", target: Target::Method("disabled"), kind: Kind::Bool },
        ],
        common: Common::All,
        handler: None,
        // The date it starts on lives in the state, so in the code you edit by
        // hand: `DatePickerState::new` opens on nothing chosen, which is what a
        // fresh picker should read as.
        state: Some(StateSpec {
            ty: "Entity<DatePickerState>",
            imports: &["use gpui::Entity;", "use gpui_component::date_picker::DatePickerState;"],
            initializer: "cx.new(|cx| DatePickerState::new(window, cx))",
        }),
    },
    Spec {
        id: "calendar",
        label: "component.calendar",
        base: "Calendar::new",
        imports: &["use gpui_component::calendar::Calendar;"],
        extra_imports: &[],
        container: false,
        palette: true,
        default_args: &[],
        default_calls: &[],
        props: &[
            Prop { label: "prop.bound_field", target: Target::BaseArg(0), kind: Kind::Field },
            Prop {
                label: "prop.months",
                target: Target::Method("number_of_months"),
                kind: Kind::Count,
            },
        ],
        // It sets its own type — a month name, a row of weekdays, thirty-odd
        // numbers — so a font weight here would be a row that does nothing.
        common: Common::Box,
        handler: None,
        state: Some(StateSpec {
            ty: "Entity<CalendarState>",
            imports: &["use gpui::Entity;", "use gpui_component::calendar::CalendarState;"],
            initializer: "cx.new(|cx| CalendarState::new(window, cx))",
        }),
    },
    Spec {
        id: "number_input",
        label: "component.number_input",
        base: "NumberInput::new",
        imports: &["use gpui_component::input::NumberInput;"],
        // `NumberInput` takes its `disabled` from the trait, where the text
        // input has one of its own — hence the line here and none on `input`.
        extra_imports: &[(&["disabled"], "use gpui_component::Disableable;")],
        container: false,
        palette: true,
        default_args: &[],
        default_calls: &[],
        props: &[
            Prop { label: "prop.bound_field", target: Target::BaseArg(0), kind: Kind::Field },
            Prop {
                label: "prop.placeholder",
                target: Target::Method("placeholder"),
                kind: Kind::Text,
            },
            Prop { label: "prop.disabled", target: Target::Method("disabled"), kind: Kind::Bool },
        ],
        common: Common::All,
        handler: None,
        // The very same `InputState` the text field takes: the difference is
        // the element wrapped around it, not the state behind it.
        state: Some(StateSpec {
            ty: "Entity<InputState>",
            imports: &["use gpui::Entity;", "use gpui_component::input::InputState;"],
            initializer: "cx.new(|cx| InputState::new(window, cx))",
        }),
    },
    Spec {
        id: "otp_input",
        label: "component.otp_input",
        base: "OtpInput::new",
        imports: &["use gpui_component::input::OtpInput;"],
        extra_imports: &[(&["disabled"], "use gpui_component::Disableable;")],
        container: false,
        palette: true,
        default_args: &[],
        default_calls: &[],
        props: &[
            Prop { label: "prop.bound_field", target: Target::BaseArg(0), kind: Kind::Field },
            Prop { label: "prop.groups", target: Target::Method("groups"), kind: Kind::Count },
            Prop { label: "prop.disabled", target: Target::Method("disabled"), kind: Kind::Bool },
        ],
        // `OtpInput` does not implement `Styled`: it draws its own boxes, and
        // the shared style calls would not compile on it.
        common: Common::None,
        handler: None,
        // How many characters it asks for is the constructor's argument, so it
        // lives in the initializer: six is what a one-time code usually is, and
        // the number is one word away in the code the developer owns.
        state: Some(StateSpec {
            ty: "Entity<OtpState>",
            imports: &["use gpui::Entity;", "use gpui_component::input::OtpState;"],
            initializer: "cx.new(|cx| OtpState::new(6, window, cx))",
        }),
    },
    Spec {
        id: "skeleton",
        label: "component.skeleton",
        base: "Skeleton::new",
        imports: &["use gpui_component::skeleton::Skeleton;"],
        extra_imports: &[],
        container: false,
        palette: true,
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
        palette: true,
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
        palette: true,
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
        palette: true,
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
        id: "scrollbar",
        label: "component.scrollbar",
        base: "Scrollbar::new",
        imports: &["use gpui_component::scroll::Scrollbar;"],
        // The axis is a variant, so its type is needed only once one is
        // written: a bar left on the default would carry an unused import.
        extra_imports: &[(&["axis"], "use gpui_component::scroll::ScrollbarAxis;")],
        container: false,
        // Written by the box's own property, never dropped: alone, with no
        // handle and no positioned parent, it draws nothing.
        palette: false,
        default_args: &[],
        default_calls: &[],
        props: &[
            Prop { label: "prop.bound_field", target: Target::BaseArg(0), kind: Kind::Field },
            Prop { label: "prop.id", target: Target::Method("id"), kind: Kind::Text },
            Prop {
                label: "prop.axis",
                target: Target::Variant("axis", SCROLLBAR_AXES),
                kind: Kind::Choice,
            },
        ],
        // Not `Styled`: it is an element of its own, which is why it is drawn
        // inside a positioned `div` rather than positioned itself.
        common: Common::None,
        handler: None,
        // A `ScrollHandle` is a plain value, not an entity: the box that
        // scrolls and the bar drawn over it must share the same one, which is
        // what makes the bar follow the content.
        state: Some(StateSpec {
            ty: "ScrollHandle",
            imports: &["use gpui::ScrollHandle;"],
            initializer: "ScrollHandle::new()",
        }),
    },
    Spec {
        id: "avatar",
        label: "component.avatar",
        base: "Avatar::new",
        imports: &["use gpui_component::avatar::Avatar;"],
        // The size comes from `Sizable`, a trait, and a trait has to be in
        // scope: imported on sight of the avatar it would be unused on every
        // one left at its default size.
        extra_imports: &[(AVATAR_SIZES, "use gpui_component::Sizable;")],
        container: false,
        palette: true,
        default_args: &[],
        // Nothing: with neither a picture nor a name, the avatar draws the
        // person icon it falls back to, which is exactly what a fresh one
        // should look like.
        default_calls: &[],
        props: &[
            // The same path an image takes, on a call instead of on the
            // constructor: `Avatar::new()` has no argument. Everything else is
            // shared with the image — the thumbnail, the file dialog, and the
            // copy into `assets/`, which all read the kind and not the target.
            Prop { label: "prop.source", target: Target::Method("src"), kind: Kind::Path },
            // What is drawn when there is no picture: the initials of the name.
            Prop { label: "prop.name", target: Target::Method("name"), kind: Kind::Text },
            Prop { label: "prop.size", target: Target::Family(AVATAR_SIZES), kind: Kind::Choice },
        ],
        // It draws the initials of a name, so the text properties are not
        // empty rows here.
        common: Common::All,
        handler: None,
        state: None,
    },
    Spec {
        id: "breadcrumb",
        label: "component.breadcrumb",
        base: "Breadcrumb::new",
        imports: &["use gpui_component::breadcrumb::Breadcrumb;"],
        extra_imports: &[],
        // Its items are `BreadcrumbItem`, not elements: a label dropped inside
        // would write `.child(Label::new(..))`, which does not compile. They
        // are a property of the crumb instead — one node, a list of names.
        container: false,
        palette: true,
        default_args: &[],
        default_calls: &[],
        props: &[Prop {
            label: "prop.items",
            target: Target::Labels("children"),
            kind: Kind::Text,
        }],
        common: Common::All,
        handler: None,
        state: None,
    },
    Spec {
        id: "kbd",
        label: "component.kbd",
        base: "Kbd::new",
        // Both always: the keystroke is the constructor's argument, so its type
        // is there from the moment the key is.
        imports: &["use gpui::Keystroke;", "use gpui_component::kbd::Kbd;"],
        extra_imports: &[],
        container: false,
        palette: true,
        default_args: &["cmd-k"],
        default_calls: &[],
        props: &[Prop { label: "prop.keystroke", target: Target::Keystroke(0), kind: Kind::Text }],
        common: Common::All,
        handler: None,
        state: None,
    },
    Spec {
        id: "clipboard",
        label: "component.clipboard",
        base: "Clipboard::new",
        imports: &["use gpui_component::clipboard::Clipboard;"],
        extra_imports: &[],
        container: false,
        palette: true,
        default_args: &["clipboard"],
        default_calls: &[],
        props: &[
            Prop { label: "prop.id", target: Target::BaseArg(0), kind: Kind::Text },
            Prop { label: "prop.value", target: Target::Method("value"), kind: Kind::Text },
        ],
        // `Clipboard` is not `Styled`: it is a button it builds itself, and the
        // shared style calls would not compile on it.
        common: Common::None,
        handler: None,
        state: None,
    },
    Spec {
        id: "tab_bar",
        label: "component.tab_bar",
        base: "TabBar::new",
        imports: &["use gpui_component::tab::TabBar;"],
        extra_imports: &[(&["with_variant"], "use gpui_component::tab::TabVariant;")],
        // Same as the breadcrumb, and for the same reason: its children are
        // `Tab`, a type, not elements. `Tab` therefore stays out of the
        // catalogue — an entry only valid under one parent is a notion the tree
        // does not have — and the bar carries its labels itself.
        container: false,
        palette: true,
        default_args: &["tabs"],
        default_calls: &[],
        props: &[
            Prop { label: "prop.id", target: Target::BaseArg(0), kind: Kind::Text },
            Prop { label: "prop.items", target: Target::Labels("children"), kind: Kind::Text },
            Prop {
                label: "prop.selected",
                target: Target::Method("selected_index"),
                kind: Kind::Count,
            },
            Prop {
                label: "prop.variant",
                target: Target::Variant("with_variant", TAB_VARIANTS),
                kind: Kind::Choice,
            },
        ],
        common: Common::All,
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
        palette: true,
        default_args: &[],
        default_calls: &[],
        props: &[Prop { label: "prop.flex", target: Target::Flag("flex_1"), kind: Kind::Bool }],
        common: Common::Element,
        handler: None,
        state: None,
    },
];
