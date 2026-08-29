//! State: the fields a component holds in the view's struct, the handlers it
//! calls, and what a copy has to be renamed to.

use crate::model::{Arg, Base, Node};

use super::ids::{element_ids, numbered};
use super::props::is_identifier;
use super::scrollbar::is_scrollbar_wrapper;
use super::{CLICK, HandlerSpec, Kind, Prop, Target, of};

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

    // A field is not always bound where it is declared: a scrolling box holds
    // its handle in `track_scroll(&self.…)`, and the bar that watches it holds
    // the same one as a constructor argument. Renaming only the second leaves a
    // copy whose box scrolls in step with the original — it compiles, and it is
    // wrong only once it runs, which is the worst kind.
    //
    // Repaired where the pairing is made, wrapper by wrapper, and not by
    // rewriting every `&self.…` of the subtree: two assemblies bound to the
    // same field would then trade handles with each other.
    subtree.walk_mut(&mut |node| {
        if !is_scrollbar_wrapper(node) {
            return;
        }
        let Some(handle) = node.children.get(1).and_then(|overlay| {
            overlay.children.first().and_then(|bar| match &bar.base {
                Base::Known { args, .. } => args.first().map(|arg| arg.to_source()),
                Base::Opaque(_) => None,
            })
        }) else {
            return;
        };
        if let Some(box_node) = node.children.first_mut() {
            box_node.set_call("track_scroll", Arg::Verbatim(handle));
        }
    });

    // Two siblings answering to the same element id share gpui's state for it:
    // a duplicated scrolling box would scroll where its twin scrolls.
    let mut ids = element_ids(root);
    subtree.walk_mut(&mut |node| {
        // Where the id sits differs: a `div` carries it in `.id(…)`, a button
        // and a checkbox in the constructor. `element_ids` reads both, so the
        // renaming has to reach both or the collision it found stays.
        let call_id =
            node.call("id").and_then(|call| call.args.first()).and_then(|arg| arg.as_str());
        let base_slot = of(node).and_then(|spec| {
            spec.props.iter().find_map(|prop| match (prop.target, prop.label) {
                (Target::BaseArg(index), "prop.id") => Some(index),
                _ => None,
            })
        });
        let base_id = base_slot.and_then(|index| match &node.base {
            Base::Known { args, .. } => args.get(index).and_then(|arg| arg.as_str()),
            Base::Opaque(_) => None,
        });

        let Some(current) = call_id.or(base_id).map(str::to_string) else {
            return;
        };
        if !ids.contains(&current) {
            ids.push(current);
            return;
        }
        // Named after the one it copies — `save_2` beside `save` — rather than
        // handed a `scroll_2` that says nothing about what it is.
        let fresh = numbered(&current, &ids);
        ids.push(fresh.clone());
        match (call_id.is_some(), base_slot) {
            (true, _) => node.set_call("id", Arg::Str(fresh)),
            (false, Some(index)) => {
                if let Base::Known { args, .. } = &mut node.base
                    && let Some(slot) = args.get_mut(index)
                {
                    *slot = Arg::Str(fresh);
                }
            }
            _ => {}
        }
    });
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
