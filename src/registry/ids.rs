//! Element ids: the names maxx gives to what gpui asks to be named.

use crate::model::{Base, Node};

use super::{Target, of};

/// An element id no node of `root` is already using.
///
/// gpui keeps a scroll offset per element id, and two siblings sharing one is a
/// conflict the framework catches at the worst moment. Walking the whole tree
/// rather than the siblings is the cheap answer, and it survives a node being
/// dragged somewhere else.
pub fn unique_element_id(root: &Node) -> String {
    next_element_id(&element_ids(root))
}

/// The first `scroll…` name that `taken` does not hold.
fn next_element_id(taken: &[String]) -> String {
    numbered("scroll", taken)
}

/// `name`, or `name_2`, `name_3` … — the first that `taken` does not hold.
pub(super) fn numbered(name: &str, taken: &[String]) -> String {
    let mut candidate = name.to_string();
    let mut index = 2;
    while taken.contains(&candidate) {
        candidate = format!("{name}_{index}");
        index += 1;
    }
    candidate
}

/// Two element ids no node of `root` answers to.
///
/// The visible scrollbar needs both at once: the box becomes stateful, and the
/// bar is an element of its own. Asked together because the first is not in the
/// tree yet when the second is chosen.
pub fn unique_element_ids(root: &Node) -> [String; 2] {
    let mut taken = element_ids(root);
    let first = next_element_id(&taken);
    taken.push(first.clone());
    let second = next_element_id(&taken);
    [first, second]
}

/// Every element id the tree answers to.
pub(super) fn element_ids(root: &Node) -> Vec<String> {
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
    taken
}
