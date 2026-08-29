//! The component catalogue: what maxx knows how to draw, and how it writes it.

use crate::model::{Arg, Base, Node};

mod catalogue;
mod ids;
mod props;
mod scrollbar;
mod state;

pub use ids::{unique_element_id, unique_element_ids};
pub use props::{covers, editable, props, read, tooltip_text, validate, write};
pub use scrollbar::{is_scrollbar_wrapper, scrollbar_assembly, unwrap_scrollbar};
pub use state::{
    handler_name, handlers, read_binding, rebind_state_fields, suggested_handler,
    unique_input_field, write_binding,
};

pub use catalogue::{CATALOGUE, COMMON, INTERACTIVE, TEXT_COMMON};
use props::path_arg;

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
    /// The visible scrollbar: `relative()`, `track_scroll(&self.…)`, and the
    /// overlay that carries the bar.
    ///
    /// One switch and five things happen, which is the opposite of how every
    /// other property works — and it is the point. A visible bar in
    /// `gpui-component` is not a call: it is a handle shared between the box
    /// that scrolls and a bar drawn over it, inside a positioned parent. Asking
    /// the developer to assemble that by hand from three catalogue entries and
    /// a field name typed twice is asking them to know what maxx knows.
    ///
    /// What is written stays ordinary Rust: two nodes of the tree, editable and
    /// deletable like any other.
    Scrollbar,
    /// A tooltip, written as the closure gpui takes:
    /// `.tooltip(|window, cx| Tooltip::new("…").build(window, cx))`.
    ///
    /// Its own target for the same reason as [`Target::Scrollable`]: it lives
    /// on a *stateful* element, so gpui offers it only after `id`. Verified
    /// against the compiler — `v_flex().tooltip(…)` does not exist, and neither
    /// does `Label::new("x").tooltip(…)`: a component that is not an element of
    /// its own would have to be wrapped in a `div`, which is one node drawn as
    /// two, and that decision belongs with the containers holding two contents.
    Tooltip,
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
    /// Everything, plus what only a gpui element can take — a tooltip.
    Element,
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
    /// Whether the palette offers it.
    ///
    /// An entry is two things at once: what the palette drops, and what the
    /// reader recognises. The scrollbar is only the second — dropped on its
    /// own it draws nothing, because it needs a handle and a positioned parent
    /// that the property on the box writes for it.
    pub palette: bool,
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

/// The label each sub-tree template wears in the palette.
///
/// Beside the catalogue rather than beside the templates: the expressions live
/// in `scaffold::templates`, which `build.rs` includes verbatim to compile them
/// and which therefore holds nothing of the interface.
pub const SUBTREE_LABELS: &[(&str, &str)] =
    &[("card", "template.card"), ("toolbar", "template.toolbar"), ("section", "template.section")];

/// The catalogue entry with this identifier.
pub fn by_id(id: &str) -> Option<&'static Spec> {
    CATALOGUE.iter().find(|spec| spec.id == id)
}

/// The catalogue entry whose constructor is this path.
///
/// What a name has to be checked against before it is offered as something
/// else: two components answering to `Badge::new` is two `use` lines for one
/// name, and a canvas drawing the wrong one.
pub fn by_path(path: &str) -> Option<&'static Spec> {
    CATALOGUE.iter().find(|spec| spec.base == path)
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
                // Not a prefix, this one: the argument is a closure, and the
                // type it builds sits in the middle of it.
                if source.contains("Tooltip::new(") {
                    let line = "use gpui_component::tooltip::Tooltip;";
                    if !lines.contains(&line) {
                        lines.push(line);
                    }
                }
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
