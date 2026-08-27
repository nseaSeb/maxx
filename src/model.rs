//! The view model.
//!
//! A widget is not modelled as a fixed property struct but as **a base
//! expression plus an ordered list of method calls** — exactly the shape of the
//! GPUI code it round-trips with. That is what keeps hand edits safe: a call
//! `maxx` does not understand still survives as data and is re-emitted
//! verbatim, and an expression it cannot parse at all degrades to
//! [`Base::Opaque`] instead of being overwritten.

use std::fmt::Write as _;

/// An argument of a base expression or of a method call.
#[derive(Clone, Debug, PartialEq)]
pub enum Arg {
    /// A string literal, decoded. Re-emitted with escaping.
    Str(String),
    /// A numeric literal, kept exactly as written (`2`, `2.0`, `16.`).
    Num(String),
    /// A boolean literal.
    Bool(bool),
    /// Any other expression, kept as source text.
    Verbatim(String),
}

impl Arg {
    /// The source text for this argument.
    pub fn to_source(&self) -> String {
        match self {
            Arg::Str(value) => format!("\"{}\"", escape(value)),
            Arg::Num(literal) => literal.clone(),
            Arg::Bool(value) => value.to_string(),
            Arg::Verbatim(source) => source.clone(),
        }
    }

    /// The string content, when this argument is a string literal.
    pub fn as_str(&self) -> Option<&str> {
        match self {
            Arg::Str(value) => Some(value),
            _ => None,
        }
    }
}

/// Marks where a child sits among the calls.
///
/// A chain can interleave children with other calls — `.child(a).when(..).child(b)` —
/// and lifting every child to the end would move it in the rendered tree. The
/// slot keeps the original order; the name cannot collide with a Rust method.
pub const CHILD_SLOT: &str = "\u{0}child";

/// One `.method(args)` in a builder chain.
#[derive(Clone, Debug, PartialEq)]
pub struct Call {
    /// Method name, without the dot.
    pub name: String,
    /// Arguments, in order.
    pub args: Vec<Arg>,
}

impl Call {
    /// A call with no arguments, e.g. `.flex_1()`.
    pub fn bare(name: impl Into<String>) -> Self {
        Self { name: name.into(), args: Vec::new() }
    }

    /// A call with a single argument.
    pub fn with(name: impl Into<String>, arg: Arg) -> Self {
        Self { name: name.into(), args: vec![arg] }
    }
}

/// The head of a builder chain.
#[derive(Clone, Debug, PartialEq)]
pub enum Base {
    /// A recognized constructor: `v_flex()`, `Button::new("ok")`.
    Known {
        /// Path of the function, without the call parentheses.
        path: String,
        /// Constructor arguments.
        args: Vec<Arg>,
    },
    /// An expression `maxx` could not interpret. Kept as source text and
    /// re-emitted unchanged; the designer shows it but refuses to edit it.
    Opaque(String),
}

impl Base {
    /// The constructor path, for a recognized base.
    pub fn path(&self) -> Option<&str> {
        match self {
            Base::Known { path, .. } => Some(path),
            Base::Opaque(_) => None,
        }
    }
}

/// A node of the view tree.
#[derive(Clone, Debug, PartialEq)]
pub struct Node {
    /// Head of the builder chain.
    pub base: Base,
    /// Method calls applied to the base, in source order, minus the `.child()`
    /// calls which are lifted into [`Node::children`].
    pub calls: Vec<Call>,
    /// Children, in order, from the `.child(..)` calls.
    pub children: Vec<Node>,
}

/// Position of a node in the tree: the child index at each level, from the root.
pub type Path = Vec<usize>;

impl Node {
    /// A node with the given constructor and no arguments.
    pub fn known(path: impl Into<String>) -> Self {
        Self {
            base: Base::Known { path: path.into(), args: Vec::new() },
            calls: Vec::new(),
            children: Vec::new(),
        }
    }

    /// A node that stands for an expression `maxx` did not interpret.
    pub fn opaque(source: impl Into<String>) -> Self {
        Self { base: Base::Opaque(source.into()), calls: Vec::new(), children: Vec::new() }
    }

    /// Whether this node is an unparsed Rust expression.
    pub fn is_opaque(&self) -> bool {
        matches!(self.base, Base::Opaque(_))
    }

    /// The first call with this name.
    pub fn call(&self, name: &str) -> Option<&Call> {
        self.calls.iter().find(|call| call.name == name)
    }

    /// Sets a single-argument call, replacing the existing one if present and
    /// appending otherwise. Keeps the position of an existing call so that
    /// editing a property does not reshuffle the generated chain.
    pub fn set_call(&mut self, name: &str, arg: Arg) {
        match self.calls.iter_mut().find(|call| call.name == name) {
            Some(call) => call.args = vec![arg],
            None => self.calls.push(Call::with(name, arg)),
        }
    }

    /// Removes the first call with this name, if any.
    pub fn remove_call(&mut self, name: &str) {
        if let Some(index) = self.calls.iter().position(|call| call.name == name) {
            self.calls.remove(index);
        }
    }

    /// Adds a no-argument call if it is not already present.
    pub fn set_flag(&mut self, name: &str, on: bool) {
        let existing = self.calls.iter().position(|call| call.name == name);
        match (existing, on) {
            (None, true) => self.calls.push(Call::bare(name)),
            (Some(index), false) => {
                self.calls.remove(index);
            }
            _ => {}
        }
    }

    /// Positions of the child slots inside `calls`.
    fn slots(&self) -> Vec<usize> {
        self.calls
            .iter()
            .enumerate()
            .filter(|(_, call)| call.name == CHILD_SLOT)
            .map(|(index, _)| index)
            .collect()
    }

    /// Appends a child and the slot that records where it goes.
    pub fn push_child(&mut self, child: Node) {
        self.calls.push(Call::bare(CHILD_SLOT));
        self.children.push(child);
    }

    /// Borrows the node at `path`, or `None` if the path leaves the tree.
    pub fn at(&self, path: &[usize]) -> Option<&Node> {
        match path.split_first() {
            None => Some(self),
            Some((index, rest)) => self.children.get(*index)?.at(rest),
        }
    }

    /// Mutably borrows the node at `path`.
    pub fn at_mut(&mut self, path: &[usize]) -> Option<&mut Node> {
        match path.split_first() {
            None => Some(self),
            Some((index, rest)) => self.children.get_mut(*index)?.at_mut(rest),
        }
    }

    /// Inserts `node` as the child at `path`, shifting later siblings right.
    ///
    /// Returns `false` if the parent path does not resolve or the index is past
    /// the end of its children.
    pub fn insert(&mut self, path: &[usize], node: Node) -> bool {
        let Some((index, parent_path)) = path.split_last() else {
            return false;
        };
        let Some(parent) = self.at_mut(parent_path) else {
            return false;
        };
        if *index > parent.children.len() {
            return false;
        }
        let slots = parent.slots();
        let at = slots.get(*index).copied().unwrap_or(parent.calls.len());
        parent.calls.insert(at, Call::bare(CHILD_SLOT));
        parent.children.insert(*index, node);
        true
    }

    /// Removes and returns the node at `path`. The root itself cannot be
    /// removed.
    pub fn remove(&mut self, path: &[usize]) -> Option<Node> {
        let (index, parent_path) = path.split_last()?;
        let parent = self.at_mut(parent_path)?;
        if *index >= parent.children.len() {
            return None;
        }
        if let Some(at) = parent.slots().get(*index).copied() {
            parent.calls.remove(at);
        }
        Some(parent.children.remove(*index))
    }

    /// Moves the node at `from` so that it becomes the child at `to`, and
    /// returns where it ended up.
    ///
    /// Refuses to move a node into its own descendants, which would detach the
    /// subtree from the tree. The destination is adjusted for the removal, which
    /// shifts every later sibling of the source left.
    pub fn move_node(&mut self, from: &[usize], to: &[usize]) -> Option<Path> {
        if to.len() > from.len() && to.starts_with(from) {
            return None;
        }
        let node = self.remove(from)?;

        // Removing the node shifts every later sibling left, so any component
        // of the destination that indexes into the source's parent — not just
        // the last one — has to be adjusted.
        let mut destination = to.to_vec();
        let (from_index, from_parent) = from.split_last().expect("remove checked the path");
        let depth = from_parent.len();
        if destination.len() > depth
            && destination[..depth] == *from_parent
            && destination[depth] > *from_index
        {
            destination[depth] -= 1;
        }

        if self.insert(&destination, node.clone()) {
            Some(destination)
        } else {
            // Put it back where it came from rather than losing it.
            self.insert(from, node);
            None
        }
    }

    /// Walks the tree depth-first, yielding each node with its path.
    pub fn walk(&self, visit: &mut impl FnMut(&[usize], &Node)) {
        fn go(node: &Node, path: &mut Vec<usize>, visit: &mut impl FnMut(&[usize], &Node)) {
            visit(path, node);
            for (index, child) in node.children.iter().enumerate() {
                path.push(index);
                go(child, path, visit);
                path.pop();
            }
        }
        go(self, &mut Vec::new(), visit);
    }

    /// Number of nodes in this subtree, including itself.
    pub fn count(&self) -> usize {
        1 + self.children.iter().map(Node::count).sum::<usize>()
    }
}

/// Escapes a string for a Rust string literal.
pub(crate) fn escape(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    for character in value.chars() {
        match character {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\t' => out.push_str("\\t"),
            '\r' => out.push_str("\\r"),
            _ => out.push(character),
        }
    }
    out
}

/// Reads a Rust string literal's content back, escapes included.
///
/// The counterpart of [`escape`]: what maxx wrote as `He said \"hi\"` has to come
/// back to the inspector as `He said "hi"`, or the field shows source instead
/// of text.
pub(crate) fn unescape(literal: &str) -> String {
    let mut out = String::with_capacity(literal.len());
    let mut chars = literal.chars();
    while let Some(character) = chars.next() {
        if character != '\\' {
            out.push(character);
            continue;
        }
        match chars.next() {
            Some('n') => out.push('\n'),
            Some('t') => out.push('\t'),
            Some('r') => out.push('\r'),
            // `\"` and `\\`, and anything else is kept as it was written.
            Some(other) => out.push(other),
            None => out.push('\\'),
        }
    }
    out
}

/// The content of the string literal starting at `source`, and what follows it.
///
/// Splitting on the first quote is not enough: a literal may carry an escaped
/// one, and cutting there gives half a text.
pub(crate) fn read_literal(source: &str) -> Option<(String, &str)> {
    let mut rest = source;
    let mut literal = String::new();
    loop {
        let character = rest.chars().next()?;
        rest = &rest[character.len_utf8()..];
        match character {
            '"' => return Some((unescape(&literal), rest)),
            '\\' => {
                let escaped = rest.chars().next()?;
                literal.push('\\');
                literal.push(escaped);
                rest = &rest[escaped.len_utf8()..];
            }
            _ => literal.push(character),
        }
    }
}

/// Renders a base or call argument list as `(a, b)`, or `()` when empty.
pub(crate) fn write_args(out: &mut String, args: &[Arg]) {
    out.push('(');
    for (index, arg) in args.iter().enumerate() {
        if index > 0 {
            out.push_str(", ");
        }
        let _ = write!(out, "{}", arg.to_source());
    }
    out.push(')');
}
