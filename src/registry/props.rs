//! Properties: what the inspector may show, what it reads back, and what it is
//! allowed to write.

use crate::model::{Arg, Base, Node};

use super::catalogue::{COMMON, INTERACTIVE, TEXT_COMMON};
use super::scrollbar::hold_for;
use super::state::handler_name;
use super::{Common, Kind, Prop, Spec, Target};

/// Every property of a component: its own, then the shared style ones.
pub fn props(spec: &'static Spec) -> Vec<&'static Prop> {
    let shared: &[Prop] = match spec.common {
        Common::All | Common::Box | Common::Element => COMMON,
        Common::None => &[],
    };
    let text: &[Prop] = match spec.common {
        Common::All | Common::Element => TEXT_COMMON,
        _ => &[],
    };
    let element: &[Prop] = match spec.common {
        Common::Element => INTERACTIVE,
        _ => &[],
    };
    spec.props.iter().chain(shared).chain(text).chain(element).collect()
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
        Target::Tooltip => name == "tooltip",
        // The handle only. `relative` is left visible among the other calls:
        // maxx writes it, but it cannot prove it wrote *this* one, and hiding
        // a call it might then delete is how a hand-written line disappears.
        Target::Scrollbar => name == "track_scroll",
    })
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
        Target::Scrollbar => Some(node.call("track_scroll").is_some().to_string()),
        Target::Tooltip => {
            let source = node.call("tooltip")?.args.first()?.to_source();
            Some(tooltip_text(&source).unwrap_or(source))
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
        // Nothing here: this one is not a call on a node but a shape around it,
        // so it is written by `Workspace::toggle_scrollbar`, which has the tree
        // — the parent to wrap into, an element id no sibling uses, a field no
        // other component is bound to. `read` below answers from the node all
        // the same, because the handle it carries is the honest evidence.
        Target::Scrollbar => {}
        Target::Tooltip => {
            if value.trim().is_empty() {
                node.remove_call("tooltip");
                return;
            }
            // The id first, and for the same reason the scroll needs it: the
            // call lives on a stateful element, and written before `id` the
            // chain does not compile. The workspace hands out an id no sibling
            // is using; this is the fallback for a node written without one.
            if node.call("id").is_none() {
                node.set_call("id", Arg::Str("tip".into()));
            }
            node.set_call(
                "tooltip",
                Arg::Verbatim(format!(
                    "|window, cx| Tooltip::new(\"{}\").build(window, cx)",
                    crate::model::escape(value)
                )),
            );
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

/// The text of a tooltip closure, when it is one maxx wrote.
///
/// Anything else — a closure building something of the developer's own — is
/// left as it is written, and shown that way in the inspector rather than
/// half-read.
pub fn tooltip_text(source: &str) -> Option<String> {
    let (_, rest) = source.split_once("Tooltip::new(\"")?;
    // Read as a literal and not split on the first quote: a tooltip saying
    // `He said "hi"` carries an escaped one, and cutting there gives half a
    // text — which the inspector would then write back, truncated.
    let (text, after) = crate::model::read_literal(rest)?;
    // And it has to be the closure maxx writes, not something of the
    // developer's that merely holds a `Tooltip::new`.
    after.starts_with(").build(window, cx)").then_some(text)
}

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
pub(super) fn path_arg(existing: Option<&Arg>, value: &str) -> Arg {
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

/// Whether a valid Rust identifier.
pub(super) fn is_identifier(value: &str) -> bool {
    let mut chars = value.chars();
    matches!(chars.next(), Some(first) if first == '_' || first.is_alphabetic())
        && chars.all(|character| character == '_' || character.is_alphanumeric())
}
