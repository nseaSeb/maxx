//! Rust source to model.
//!
//! `syn` throws comments away, so the whole file is never handed to it. The
//! managed region is located by scanning for the markers, only that substring
//! is parsed, and writing back splices the generated text into the same byte
//! range. Everything outside the markers — imports, `struct`, `impl`, handlers,
//! comments, formatting — survives by construction.

use syn::spanned::Spanned as _;
use syn::{Expr, Lit};

use crate::model::{Arg, Call, Node};

/// Opening marker of the region `maxx` owns.
pub const BEGIN: &str = "// maxx:begin";
/// Closing marker of the region `maxx` owns.
pub const END: &str = "// maxx:end";

/// The offset of the `}` closing the `{` at `open`.
///
/// Braces inside comments, strings and char literals do not count. Counting
/// them ends a block early, and the callers of this function splice code at the
/// offset it returns — a `}` in a doc comment was enough to write a method stub
/// into the middle of that comment.
pub fn matching_brace(source: &str, open: usize) -> Option<usize> {
    let bytes = source.as_bytes();
    let mut depth = 0usize;
    let mut index = open;

    while index < bytes.len() {
        match bytes[index] {
            b'/' if bytes.get(index + 1) == Some(&b'/') => {
                index = source[index..].find('\n').map_or(bytes.len(), |offset| index + offset + 1);
                continue;
            }
            b'/' if bytes.get(index + 1) == Some(&b'*') => {
                index = source[index + 2..]
                    .find("*/")
                    .map_or(bytes.len(), |offset| index + 2 + offset + 2);
                continue;
            }
            b'r' => {
                if let Some(end) = raw_string_end(source, index) {
                    index = end;
                    continue;
                }
                index += 1;
            }
            b'"' => {
                index = quoted_end(source, index);
                continue;
            }
            b'\'' => match char_literal_end(source, index) {
                Some(end) => {
                    index = end;
                    continue;
                }
                // A lifetime: nothing to skip.
                None => index += 1,
            },
            b'{' => {
                depth += 1;
                index += 1;
            }
            b'}' => {
                depth -= 1;
                if depth == 0 {
                    return Some(index);
                }
                index += 1;
            }
            _ => index += 1,
        }
    }
    None
}

/// The offset just past a `"…"` literal starting at `start`.
fn quoted_end(source: &str, start: usize) -> usize {
    let bytes = source.as_bytes();
    let mut index = start + 1;
    while index < bytes.len() {
        match bytes[index] {
            b'\\' => index += 2,
            b'"' => return index + 1,
            _ => index += 1,
        }
    }
    bytes.len()
}

/// The offset just past a `'x'` literal starting at `start`, or `None` when the
/// quote opens a lifetime.
///
/// Telling them apart matters: treating `'a` as a literal makes the scan pair
/// it with the next quote and skip everything in between.
fn char_literal_end(source: &str, start: usize) -> Option<usize> {
    let bytes = source.as_bytes();
    if bytes.get(start + 1) == Some(&b'\\') {
        // An escape: read to the closing quote, which is close by.
        let mut index = start + 2;
        while index < bytes.len() && index < start + 12 {
            if bytes[index] == b'\'' {
                return Some(index + 1);
            }
            index += 1;
        }
        return None;
    }
    let mut chars = source[start + 1..].char_indices();
    let (_, first) = chars.next()?;
    let after = start + 1 + first.len_utf8();
    (bytes.get(after) == Some(&b'\'')).then_some(after + 1)
}

/// The offset just past a `r"…"` / `r#"…"#` literal starting at `start`.
fn raw_string_end(source: &str, start: usize) -> Option<usize> {
    let bytes = source.as_bytes();
    let mut index = start + 1;
    let mut hashes = 0;
    while bytes.get(index) == Some(&b'#') {
        hashes += 1;
        index += 1;
    }
    if bytes.get(index) != Some(&b'"') {
        return None;
    }
    let terminator = format!("\"{}", "#".repeat(hashes));
    source[index + 1..].find(&terminator).map(|offset| index + 1 + offset + terminator.len())
}

/// The byte ranges of the string literals in `source`.
///
/// Used to leave the inside of a multi-line literal alone when re-indenting.
fn string_ranges(source: &str) -> Vec<std::ops::Range<usize>> {
    let bytes = source.as_bytes();
    let mut ranges = Vec::new();
    let mut index = 0;
    while index < bytes.len() {
        match bytes[index] {
            b'/' if bytes.get(index + 1) == Some(&b'/') => {
                index = source[index..].find('\n').map_or(bytes.len(), |offset| index + offset + 1);
            }
            b'/' if bytes.get(index + 1) == Some(&b'*') => {
                index = source[index + 2..]
                    .find("*/")
                    .map_or(bytes.len(), |offset| index + 2 + offset + 2);
            }
            b'r' => match raw_string_end(source, index) {
                Some(end) => {
                    ranges.push(index..end);
                    index = end;
                }
                None => index += 1,
            },
            b'"' => {
                let end = quoted_end(source, index);
                ranges.push(index..end);
                index = end;
            }
            // A `'"'` would otherwise open a range running to the next quote.
            b'\'' => match char_literal_end(source, index) {
                Some(end) => index = end,
                None => index += 1,
            },
            _ => index += 1,
        }
    }
    ranges
}

/// Wraps the expression a hand-written `render` returns in maxx's markers, so
/// the view can be opened in the designer.
///
/// Nothing else in the file is touched. Statements before the final expression
/// are left alone — only the expression itself becomes the managed region.
pub fn adopt(source: &str) -> Result<String, Error> {
    if locate(source).is_ok() {
        return Err(Error::AlreadyManaged);
    }

    let offset = source.find("fn render(").ok_or(Error::NoRender)?;
    let open = source[offset..].find('{').map(|index| offset + index).ok_or(Error::NoRender)?;
    // The first `{` after `fn render(` closes the argument list's type
    // parameters in no case we generate, but a return type like `impl
    // IntoElement` has none either, so this is the body.
    let close = matching_brace(source, open).ok_or(Error::NoRender)?;

    let block: syn::Block = syn::parse_str(&source[open..=close]).map_err(Error::Syntax)?;
    let last = block.stmts.last().ok_or(Error::NoTrailingExpression)?;
    let syn::Stmt::Expr(expression, None) = last else {
        return Err(Error::NoTrailingExpression);
    };

    // Spans are relative to the string handed to `parse_str`, which started at
    // `open`.
    let range = expression.span().byte_range();
    let start = open + range.start;
    let end = open + range.end;
    if start >= source.len() || end > source.len() {
        return Err(Error::NoTrailingExpression);
    }

    let line_start = source[..start].rfind('\n').map_or(0, |index| index + 1);
    let indent = &source[line_start..start];
    // Widening the markers to whole lines would drag whatever shares the line
    // into the managed region, and the file would no longer open at all.
    if !indent.trim().is_empty() {
        return Err(Error::NoTrailingExpression);
    }
    let line_end = source[end..].find('\n').map_or(source.len(), |index| end + index + 1);

    let mut out = String::with_capacity(source.len() + 48);
    out.push_str(&source[..line_start]);
    out.push_str(indent);
    out.push_str(BEGIN);
    out.push('\n');
    out.push_str(&source[line_start..line_end]);
    out.push_str(indent);
    out.push_str(END);
    out.push('\n');
    out.push_str(&source[line_end..]);
    Ok(out)
}

/// Why a file could not be read as a `maxx` view.
#[derive(Debug)]
pub enum Error {
    /// The file already carries markers.
    AlreadyManaged,
    /// No `fn render` to adopt.
    NoRender,
    /// The body of `render` does not end on an expression.
    NoTrailingExpression,
    /// The file carries no managed region.
    NoMarkers,
    /// The markers are present but in the wrong order.
    MarkersOutOfOrder,
    /// The managed region is not a single Rust expression.
    Syntax(syn::Error),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::AlreadyManaged => write!(f, "cette vue est déjà gérée par maxx"),
            Error::NoRender => write!(f, "aucun « fn render » dans ce fichier"),
            Error::NoTrailingExpression => write!(
                f,
                "le corps de « render » ne se termine pas par une expression — \
                 maxx ne saurait pas où poser ses marqueurs"
            ),
            Error::NoMarkers => write!(f, "aucune zone « {BEGIN} » dans ce fichier"),
            Error::MarkersOutOfOrder => write!(f, "« {END} » apparaît avant « {BEGIN} »"),
            Error::Syntax(error) => write!(f, "la zone gérée n'est pas une expression : {error}"),
        }
    }
}

/// The managed region of a file: the byte range between the markers, and the
/// indentation the block sits at.
#[derive(Debug, Clone)]
pub struct Region {
    /// Offset of the first byte after the line carrying [`BEGIN`].
    pub start: usize,
    /// Offset of the first byte of the line carrying [`END`].
    pub end: usize,
    /// Indentation of the `BEGIN` marker, reused verbatim for the generated
    /// block — tabs stay tabs.
    pub indent: String,
    /// The line ending the file uses, so a CRLF file stays CRLF.
    pub newline: &'static str,
}

impl Region {
    /// Width of the indentation, for the line-length budget.
    pub fn width(&self) -> usize {
        self.indent.chars().map(|c| if c == '\t' { 4 } else { 1 }).sum()
    }
}

/// Whether the managed region holds a comment.
///
/// `syn` throws comments away, so saving would delete it. The caller refuses to
/// write rather than lose it quietly.
pub fn region_has_comment(source: &str) -> bool {
    let Ok(region) = locate(source) else {
        return false;
    };
    let inner = &source[region.start..region.end];
    let literals = string_ranges(inner);
    let bytes = inner.as_bytes();
    let mut index = 0;
    while index + 1 < bytes.len() {
        if bytes[index] == b'/' && matches!(bytes[index + 1], b'/' | b'*') {
            let in_literal = literals.iter().any(|range| range.contains(&index));
            if !in_literal {
                return true;
            }
        }
        index += 1;
    }
    false
}

/// Locates the managed region of `source`.
pub fn locate(source: &str) -> Result<Region, Error> {
    let begin = source.find(BEGIN).ok_or(Error::NoMarkers)?;
    let end_marker = source.find(END).ok_or(Error::NoMarkers)?;
    if end_marker < begin {
        return Err(Error::MarkersOutOfOrder);
    }

    // The region starts after the end of the line holding the opening marker.
    let start = match source[begin..].find('\n') {
        Some(offset) => begin + offset + 1,
        None => source.len(),
    };
    // ...and stops at the start of the line holding the closing marker.
    let end = source[..end_marker].rfind('\n').map_or(0, |offset| offset + 1);

    let line_start = source[..begin].rfind('\n').map_or(0, |offset| offset + 1);
    let indent = source[line_start..begin].to_string();
    let newline = if source.contains("\r\n") { "\r\n" } else { "\n" };

    Ok(Region { start, end: end.max(start), indent, newline })
}

/// Parses the managed region of `source` into a view tree.
pub fn parse(source: &str) -> Result<(Node, Region), Error> {
    let region = locate(source)?;
    // Dedented first: `splice` re-indents every line on the way out, so an
    // opaque expression kept with its file indentation would gain a level of
    // indentation on every save.
    let inner = dedent(&source[region.start..region.end], &region.indent);
    let inner = inner.trim();
    let expr: Expr = syn::parse_str(inner).map_err(Error::Syntax)?;
    // Spans are relative to the string handed to `parse_str`, so the node's
    // verbatim slices index into that same string.
    Ok((node_from_expr(&expr, inner), region))
}

/// Reads a lone builder expression, outside any file.
///
/// What the clipboard carries is Rust source, not a private format: a subtree
/// copied here pastes into Zed, and an expression written by hand there pastes
/// back. So the same reader that handles a managed region handles the
/// clipboard, and neither can drift from the other.
pub fn parse_expr(source: &str) -> Result<Node, Error> {
    let source = source.trim();
    let expr: Expr = syn::parse_str(source).map_err(Error::Syntax)?;
    Ok(node_from_expr(&expr, source))
}

/// Removes the region's own indentation from every line.
///
/// A line that begins inside a multi-line string literal is part of the user's
/// data and is left exactly as it is — `splice` does not indent it on the way
/// out, and stripping it here would eat characters from the string on every
/// save.
pub(crate) fn dedent(source: &str, indent: &str) -> String {
    let literals = string_ranges(source);
    let mut out = String::with_capacity(source.len());
    let mut offset = 0usize;

    for line in source.lines() {
        let inside_literal =
            literals.iter().any(|range| range.start < offset && offset < range.end);
        if inside_literal {
            out.push_str(line);
        } else {
            let stripped = line.strip_prefix(indent).unwrap_or_else(|| {
                // A line indented less than the markers: take what is there.
                line.trim_start_matches([' ', '\t'])
            });
            out.push_str(stripped);
        }
        out.push('\n');
        offset += line.len() + 1;
    }
    out
}

/// Replaces the managed region of `source` with `block`, indenting it to sit
/// where the markers are.
pub fn splice(source: &str, block: &str) -> Result<String, Error> {
    let region = locate(source)?;

    // A line that begins inside a multi-line string literal is part of the
    // user's data: indenting it would silently change what the string says.
    let literals = string_ranges(block);
    let mut offset = 0usize;

    let mut rendered = String::with_capacity(block.len() + 16);
    for line in block.lines() {
        let inside_literal =
            literals.iter().any(|range| range.start < offset && offset < range.end);
        if line.is_empty() {
            rendered.push_str(region.newline);
        } else {
            if !inside_literal {
                rendered.push_str(&region.indent);
            }
            rendered.push_str(line);
            rendered.push_str(region.newline);
        }
        offset += line.len() + 1;
    }

    let mut out = String::with_capacity(source.len() + rendered.len());
    out.push_str(&source[..region.start]);
    out.push_str(&rendered);
    out.push_str(&source[region.end..]);
    Ok(out)
}

/// Turns an expression into a node.
///
/// A chain whose head is not a plain `path(args)` call is kept whole as an
/// opaque node: re-emitting its source text unchanged is always safe, whereas
/// guessing at its structure is not.
fn node_from_expr(expr: &Expr, source: &str) -> Node {
    let mut chain = Vec::new();
    let mut head = expr;
    while let Expr::MethodCall(call) = head {
        chain.push(call);
        head = &call.receiver;
    }
    chain.reverse();

    let Expr::Call(call) = head else {
        return Node::opaque(text(expr, source));
    };
    let Expr::Path(path) = call.func.as_ref() else {
        return Node::opaque(text(expr, source));
    };

    let mut node = Node {
        base: crate::model::Base::Known {
            path: text(&Expr::Path(path.clone()), source),
            args: call.args.iter().map(|arg| arg_from_expr(arg, source)).collect(),
        },
        calls: Vec::new(),
        children: Vec::new(),
    };

    for method in chain {
        let name = method.method.to_string();
        if name == "child" && method.args.len() == 1 {
            let child = method.args.first().expect("length checked");
            node.push_child(node_from_expr(child, source));
        } else {
            node.calls.push(Call {
                name,
                args: method.args.iter().map(|arg| arg_from_expr(arg, source)).collect(),
            });
        }
    }

    node
}

/// Classifies a call argument, keeping anything unrecognized as source text.
fn arg_from_expr(expr: &Expr, source: &str) -> Arg {
    let Expr::Lit(literal) = expr else {
        return Arg::Verbatim(text(expr, source));
    };
    match &literal.lit {
        Lit::Str(value) => Arg::Str(value.value()),
        Lit::Bool(value) => Arg::Bool(value.value()),
        Lit::Int(_) | Lit::Float(_) => Arg::Num(text(expr, source)),
        _ => Arg::Verbatim(text(expr, source)),
    }
}

/// The source text an expression was parsed from.
///
/// Byte ranges come from `proc-macro2`'s `span-locations` feature and are
/// relative to the string given to `syn::parse_str`. If a span ever falls
/// outside `source` the token stream is used instead — less faithful, but it
/// never panics on a slice.
fn text(expr: &Expr, source: &str) -> String {
    let range = expr.span().byte_range();
    match source.get(range) {
        Some(slice) => slice.to_string(),
        None => {
            use quote::ToTokens as _;
            expr.to_token_stream().to_string()
        }
    }
}
