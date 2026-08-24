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
pub(crate) fn matching_brace(source: &str, open: usize) -> Option<usize> {
    let mut depth = 0usize;
    for (offset, character) in source[open..].char_indices() {
        match character {
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    return Some(open + offset);
                }
            }
            _ => {}
        }
    }
    None
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
    let open = source[offset..]
        .find('{')
        .map(|index| offset + index)
        .ok_or(Error::NoRender)?;
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
    let indent = if indent.trim().is_empty() { indent } else { "        " };
    let line_end = source[end..]
        .find('\n')
        .map_or(source.len(), |index| end + index + 1);

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
    /// Indentation of the `BEGIN` marker, reused for the generated block.
    pub indent: usize,
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
    let indent = source[line_start..begin].len();

    Ok(Region {
        start,
        end: end.max(start),
        indent,
    })
}

/// Parses the managed region of `source` into a view tree.
pub fn parse(source: &str) -> Result<(Node, Region), Error> {
    let region = locate(source)?;
    // Dedented first: `splice` re-indents every line on the way out, so an
    // opaque expression kept with its file indentation would gain a level of
    // indentation on every save.
    let inner = dedent(&source[region.start..region.end], region.indent);
    let inner = inner.trim();
    let expr: Expr = syn::parse_str(inner).map_err(Error::Syntax)?;
    // Spans are relative to the string handed to `parse_str`, so the node's
    // verbatim slices index into that same string.
    Ok((node_from_expr(&expr, inner), region))
}

/// Removes up to `indent` leading spaces from every line.
fn dedent(source: &str, indent: usize) -> String {
    let mut out = String::with_capacity(source.len());
    for line in source.lines() {
        let strip = line
            .chars()
            .take(indent)
            .take_while(|character| *character == ' ')
            .count();
        out.push_str(&line[strip..]);
        out.push('\n');
    }
    out
}

/// Replaces the managed region of `source` with `block`, indenting it to sit
/// where the markers are.
pub fn splice(source: &str, block: &str) -> Result<String, Error> {
    let region = locate(source)?;
    let indent = " ".repeat(region.indent);

    let mut rendered = String::with_capacity(block.len() + 16);
    for line in block.lines() {
        if line.is_empty() {
            rendered.push('\n');
        } else {
            rendered.push_str(&indent);
            rendered.push_str(line);
            rendered.push('\n');
        }
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
            node.children.push(node_from_expr(child, source));
        } else {
            node.calls.push(Call {
                name,
                args: method
                    .args
                    .iter()
                    .map(|arg| arg_from_expr(arg, source))
                    .collect(),
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
