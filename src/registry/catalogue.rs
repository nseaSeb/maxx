//! The catalogue itself: every component maxx can drop, and the values its
//! properties accept.

use super::{CLICK, Common, Kind, Prop, Spec, StateSpec, TOGGLED, Target};

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

/// The icons offered in the inspector.
///
/// A choice out of the eighty-eight `IconName` carries: a list that long is not
/// a list one picks from, and every name here is drawn on the canvas by
/// `designer::canvas::icon_named`, which is what `tests/catalogue.rs` holds the two
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

/// The shared properties of a component that is a gpui element of its own.
///
/// Not in [`COMMON`], which every entry of the catalogue accepts: a tooltip
/// needs `StatefulInteractiveElement`, which a `div` has once it carries an
/// `id` and which no `gpui-component` widget has at all. Posed on everything,
/// it would be a call that does not compile — the defect this table exists to
/// prevent.
pub const INTERACTIVE: &[Prop] =
    &[Prop { label: "prop.tooltip", target: Target::Tooltip, kind: Kind::Text }];

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
        palette: true,
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
            Prop { label: "prop.flex", target: Target::Flag("flex_1"), kind: Kind::Bool },
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
        palette: true,
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
