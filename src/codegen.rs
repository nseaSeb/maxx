//! Model to Rust source.
//!
//! The output is hand-formatted rather than run through `prettyplease`: the
//! designer rewrites these files constantly, so the layout has to be stable and
//! diff-friendly — one builder call per line, indentation by depth.

use crate::model::{Base, CHILD_SLOT, Node, write_args};

/// Width beyond which a chain is broken across lines.
const WIDTH: usize = 76;

/// One indentation level.
const INDENT: &str = "    ";

/// Renders `node` as a Rust expression whose continuation lines are indented
/// `depth` levels.
///
/// The result never carries leading indentation on its first line — the caller
/// places it — and never a trailing newline.
pub fn render(node: &Node, depth: usize) -> String {
    with_own_comments(node, render_with(node, depth, 0), &INDENT.repeat(depth))
}

/// Renders `node` for a block that a later step will indent by `offset`
/// columns. Continuation lines start at column zero, and the line-width budget
/// accounts for the indentation that will be added.
pub fn render_for_splice(node: &Node, offset: usize) -> String {
    with_own_comments(node, render_with(node, 0, offset), "")
}

/// Puts the root's own comments above the chain.
///
/// Every other node has a parent to write them — the line above its `.child(`.
/// The root has none, so the two entry points do it, and only they: doing it
/// inside `render_with` would write a child's comments twice.
fn with_own_comments(node: &Node, rendered: String, indent: &str) -> String {
    if node.comments.is_empty() {
        return rendered;
    }
    let mut out = String::new();
    for comment in &node.comments {
        for (index, line) in comment.lines().enumerate() {
            if index == 0 {
                out.push_str(line.trim_start());
            } else {
                out.push_str(line);
            }
            out.push('\n');
            out.push_str(indent);
        }
    }
    out.push_str(&rendered);
    out
}

fn render_with(node: &Node, depth: usize, offset: usize) -> String {
    // A comment has nowhere to go on a single line: a chain carrying one is
    // broken across lines however short it is.
    if !node.has_comments() {
        let inline = render_inline(node);
        if !inline.contains('\n') && offset + depth * INDENT.len() + inline.len() <= WIDTH {
            return inline;
        }
    }
    render_block(node, depth, offset)
}

/// Renders the whole chain on a single line.
fn render_inline(node: &Node) -> String {
    let mut out = String::new();
    write_head(&mut out, node);
    let mut children = node.children.iter();
    for call in &node.calls {
        if call.name == CHILD_SLOT {
            if let Some(child) = children.next() {
                out.push_str(".child(");
                out.push_str(&render_inline(child));
                out.push(')');
            }
            continue;
        }
        out.push('.');
        out.push_str(&call.name);
        write_args(&mut out, &call.args);
    }
    // Children added without a slot — built programmatically — go last.
    for child in children {
        out.push_str(".child(");
        out.push_str(&render_inline(child));
        out.push(')');
    }
    out
}

/// Renders the chain across several lines, one call per line.
fn render_block(node: &Node, depth: usize, offset: usize) -> String {
    let inner = INDENT.repeat(depth + 1);
    let mut out = String::new();
    write_head(&mut out, node);

    let mut children = node.children.iter();
    for call in &node.calls {
        if call.name == CHILD_SLOT {
            if let Some(child) = children.next() {
                write_child(&mut out, child, depth, offset, &inner);
            }
            continue;
        }
        write_comments(&mut out, &call.comments, &inner);
        out.push('\n');
        out.push_str(&inner);
        out.push('.');
        out.push_str(&call.name);
        write_args(&mut out, &call.args);
    }

    // Children added without a slot — built programmatically — go last.
    for child in children {
        write_child(&mut out, child, depth, offset, &inner);
    }

    write_comments(&mut out, &node.trailing, &inner);
    out
}

/// Writes comment lines, each on its own line, at `indent`.
///
/// A block comment keeps its shape: every line after the first is written at
/// the same column, which is where it was read from — the region is dedented on
/// the way in and indented again on the way out.
fn write_comments(out: &mut String, comments: &[String], indent: &str) {
    for comment in comments {
        for (index, line) in comment.lines().enumerate() {
            out.push('\n');
            if index == 0 {
                out.push_str(indent);
                out.push_str(line.trim_start());
            } else {
                // The lines of a block comment keep the column they were
                // written in: aligning them on the chain would take the shape
                // out of an ASCII drawing or a lined-up table.
                out.push_str(line);
            }
        }
    }
}

/// Writes one `.child(..)`, on its own line when the child does not fit.
fn write_child(out: &mut String, child: &Node, depth: usize, offset: usize, inner: &str) {
    // The child's own comments sit above the `.child(` that holds it: it is the
    // parent that knows the column they belong in.
    write_comments(out, &child.comments, inner);
    let rendered = render_with(child, depth + 1, offset);
    out.push('\n');
    out.push_str(inner);
    if rendered.contains('\n') {
        // Break the argument onto its own line so the nested chain keeps its
        // own indentation column.
        out.push_str(".child(\n");
        out.push_str(&INDENT.repeat(depth + 2));
        out.push_str(&render_with(child, depth + 2, offset));
        out.push_str(",\n");
        out.push_str(inner);
        out.push(')');
    } else {
        out.push_str(".child(");
        out.push_str(&rendered);
        out.push(')');
    }
}

/// Writes the head of the chain: the constructor, or the verbatim source of an
/// expression `maxx` did not interpret.
fn write_head(out: &mut String, node: &Node) {
    match &node.base {
        Base::Known { path, args } => {
            out.push_str(path);
            write_args(out, args);
        }
        // Emitted byte for byte: an opaque node must come back out of the file
        // exactly as the human wrote it.
        Base::Opaque(source) => out.push_str(source),
    }
}
