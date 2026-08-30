//! The round-trip contract, stated once and checked on trees nobody wrote.
//!
//! `tests/round_trip.rs` holds examples: a shape someone thought of, written
//! down. This holds the invariant those examples are instances of —
//!
//! ```text
//! tree → codegen::render → parser::parse → the same tree
//! ```
//!
//! — and hands it trees drawn at random. That is the half an example file
//! cannot cover: the interesting failures of a parser live in combinations
//! nobody thinks to write, and a generator reaches them in a few seconds.
//!
//! The generator draws from realistic vocabularies rather than from arbitrary
//! text. A random byte string is not a Rust expression, and a failure on one
//! would say nothing about the round trip; what matters is that the *shapes*
//! combine freely — a comment above a call that carries a verbatim argument
//! inside a child of an opaque node, and so on.

use maxx::codegen::{render, render_for_splice};
use maxx::model::{Arg, Base, CHILD_SLOT, Call, Node};
use maxx::parser;
use proptest::prelude::*;

/// Wraps an expression in the smallest file carrying a managed region.
fn file_with(expr: &str) -> String {
    let mut out = String::from(
        "use gpui::*;\n\n\
         impl Render for Home {\n\
         \x20   fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {\n\
         \x20       // maxx:begin\n",
    );
    for line in expr.lines() {
        out.push_str("        ");
        out.push_str(line);
        out.push('\n');
    }
    out.push_str(
        "        // maxx:end\n\
         \x20   }\n\
         }\n",
    );
    out
}

/// The constructors a generated base is drawn from.
///
/// Each is paired with the number of arguments it takes, because a constructor
/// written with the wrong arity is a Rust error and not a round-trip question.
const BASES: &[(&str, usize)] = &[
    ("div", 0),
    ("v_flex", 0),
    ("h_flex", 0),
    ("Label::new", 1),
    ("Button::new", 1),
    ("Icon::new", 1),
];

/// Method names a generated call is drawn from.
const CALLS: &[&str] = &[
    "gap_2",
    "p_4",
    "flex_1",
    "w_full",
    "items_center",
    "justify_between",
    "text_sm",
    "rounded_md",
];

/// Method names taking one argument, with the kind of argument they take.
const CALLS_WITH_ARG: &[&str] = &["label", "id", "text_color", "on_click", "w", "h"];

/// Expressions that are neither literals nor children: what `Arg::Verbatim`
/// holds in a real project.
const VERBATIM: &[&str] = &[
    "&self.name",
    "cx.listener(Self::on_submit)",
    "IconName::Check",
    "px(4.)",
    "theme::accent(cx)",
    "PathBuf::from(\"assets/images/logo.png\")",
];

/// Numeric literals, in the spellings a developer actually writes.
const NUMBERS: &[&str] = &["0", "1", "2", "16", "2.0", "16.", "0.5"];

/// Expressions an opaque node stands for: things maxx shows but never rewrites.
///
/// `Self::header(cx)` is deliberately *not* here, and the distinction is the
/// parser's, not a matter of taste: a plain call expression is a base maxx
/// recognises, whatever it calls, so it comes back as `Base::Known` rather than
/// opaque. What stays opaque is what has no builder shape at all.
const OPAQUE: &[&str] = &[
    "self.render_row(cx)",
    "if self.busy { spinner() } else { div() }",
    "match self.tab { Tab::One => one(), Tab::Two => two() }",
];

/// The text of a comment line, without its `//`.
///
/// No newline, and nothing that would close the comment early; the accents and
/// the punctuation are there because a comment is prose and prose has them.
fn comment_text() -> impl Strategy<Value = String> {
    prop::sample::select(vec![
        "what this line is for".to_string(),
        "TODO: décider si la marge reste".to_string(),
        "the width is the handle's, not ours".to_string(),
        "\u{a0}leading nbsp, and a tab\there".to_string(),
        "émoji 🎛 and a quote \" inside".to_string(),
    ])
}

/// A block of zero to two comment lines, as `codegen` expects them: source
/// text, `//` included.
fn comments() -> impl Strategy<Value = Vec<String>> {
    prop::collection::vec(comment_text().prop_map(|text| format!("// {text}")), 0..3)
}

/// The content of a string literal.
///
/// The escaping is `codegen`'s business and the decoding is the parser's; this
/// is where the two are held to each other, so the quote, the backslash and the
/// newline all have to appear.
fn string_content() -> impl Strategy<Value = String> {
    prop::sample::select(vec![
        String::new(),
        "Name".to_string(),
        "a \"quoted\" word".to_string(),
        "back\\slash".to_string(),
        "two\nlines".to_string(),
        "tab\there".to_string(),
        "accentué — em dash".to_string(),
        "🎛".to_string(),
    ])
}

fn arg() -> impl Strategy<Value = Arg> {
    prop_oneof![
        string_content().prop_map(Arg::Str),
        prop::sample::select(NUMBERS).prop_map(|n| Arg::Num(n.to_string())),
        any::<bool>().prop_map(Arg::Bool),
        prop::sample::select(VERBATIM).prop_map(|v| Arg::Verbatim(v.to_string())),
    ]
}

fn call() -> impl Strategy<Value = Call> {
    let bare = (prop::sample::select(CALLS), comments()).prop_map(|(name, comments)| Call {
        name: name.to_string(),
        args: Vec::new(),
        comments,
    });
    let with_arg = (prop::sample::select(CALLS_WITH_ARG), arg(), comments()).prop_map(
        |(name, arg, comments)| Call { name: name.to_string(), args: vec![arg], comments },
    );
    prop_oneof![bare, with_arg]
}

/// A leaf: a base, its arguments, its calls, and the comments around them.
fn leaf() -> impl Strategy<Value = Node> {
    let known =
        (prop::sample::select(BASES), arg(), prop::collection::vec(call(), 0..4), comments())
            .prop_map(|((path, arity), argument, calls, trailing)| Node {
                base: Base::Known {
                    path: path.to_string(),
                    args: if arity == 0 { Vec::new() } else { vec![argument] },
                },
                calls,
                children: Vec::new(),
                comments: Vec::new(),
                trailing,
            });
    // With its own trailing comments: an opaque node ending on a `//` line is
    // precisely the shape the comma fix is about, and a generator that never
    // draws it would have let that fix rot untested.
    let opaque = (prop::sample::select(OPAQUE), comments()).prop_map(|(source, trailing)| {
        let mut node = Node::opaque(source.to_string());
        node.trailing = trailing;
        node
    });
    prop_oneof![4 => known, 1 => opaque]
}

/// A tree: leaves, then up to three levels of containers over them.
///
/// A child carries the comments written above it, because those are emitted by
/// whoever places the node — the parent. The root's own comments belong to the
/// splice, not to `render`, so they are left empty here and covered by
/// `tests/round_trip.rs`.
///
/// Every child is placed twice, and that is the model's own shape rather than a
/// quirk of the generator: the subtree goes in `children`, and a `CHILD_SLOT`
/// call goes in `calls` to say *where among the other calls* it sits. A chain
/// can interleave them — `.child(a).when(..).child(b)` — so the slot is the
/// only thing that keeps the order the developer wrote.
fn tree() -> impl Strategy<Value = Node> {
    leaf().prop_recursive(3, 24, 3, |inner| {
        (
            prop::sample::select(BASES),
            prop::collection::vec((inner, comments()), 0..4),
            prop::collection::vec(call(), 0..3),
            comments(),
            any::<bool>(),
        )
            .prop_map(|((path, arity), children, calls, trailing, children_first)| {
                let (children, child_slots): (Vec<Node>, Vec<Call>) = children
                    .into_iter()
                    .map(|(mut child, comments)| {
                        child.comments = comments;
                        (child, Call::bare(CHILD_SLOT))
                    })
                    .unzip();
                // Before or after the other calls, so that both orders are
                // drawn; the interleavings between them are reached by the
                // recursion.
                let calls = if children_first {
                    child_slots.into_iter().chain(calls).collect()
                } else {
                    calls.into_iter().chain(child_slots).collect()
                };
                Node {
                    base: Base::Known {
                        path: path.to_string(),
                        args: if arity == 0 { Vec::new() } else { vec![Arg::Str("x".into())] },
                    },
                    calls,
                    children,
                    comments: Vec::new(),
                    trailing,
                }
            })
    })
}

proptest! {
    // A thousand trees rather than the default 256: the whole file still runs
    // in under a second, and the shapes this reaches for — a comment on the
    // last line of a child of a child — are drawn a few times in a hundred.
    #![proptest_config(ProptestConfig { cases: 1000, ..ProptestConfig::default() })]

    /// What the designer writes, the parser reads back as the same tree.
    ///
    /// The whole promise of maxx in one line: the `.rs` file is the truth, so
    /// the model has to survive the trip through it without moving.
    #[test]
    fn a_rendered_tree_parses_back_to_itself(node in tree()) {
        let source = render(&node, 0);
        let file = file_with(&source);
        let (parsed, _) = parser::parse(&file)
            .map_err(|error| TestCaseError::fail(format!("{error}\n--- rendered ---\n{source}")))?;
        prop_assert_eq!(&parsed, &node, "\n--- rendered ---\n{}", source);
    }

    /// The same, through the function a save actually calls.
    ///
    /// `render` is what the tests reach for; `view::save` writes through
    /// `render_for_splice`, which differs by the column budget it is given. The
    /// two are near enough that one standing in for the other is tempting, and
    /// exactly near enough that a divergence would go unnoticed.
    #[test]
    fn the_shape_a_save_writes_parses_back_to_itself(node in tree(), offset in 0usize..24) {
        let source = render_for_splice(&node, offset);
        let file = file_with(&source);
        let (parsed, _) = parser::parse(&file)
            .map_err(|error| TestCaseError::fail(format!("{error}\n--- rendered ---\n{source}")))?;
        prop_assert_eq!(&parsed, &node, "\n--- rendered ---\n{}", source);
    }

    /// And rendering it a second time writes the same text.
    ///
    /// The stronger half: a tree that parses back to itself could still be
    /// rendered differently the second time — an indent that grows, a chain
    /// that folds — and every save would then show a diff nobody asked for.
    #[test]
    fn rendering_twice_writes_the_same_text(node in tree()) {
        let once = render(&node, 0);
        let file = file_with(&once);
        let (parsed, _) = parser::parse(&file)
            .map_err(|error| TestCaseError::fail(format!("{error}\n--- rendered ---\n{once}")))?;
        prop_assert_eq!(render(&parsed, 0), once);
    }
}

/// The import pass is a fixed point, and it only ever adds.
///
/// The invariant this pass promises — "written once and only once" — and the
/// one that kept breaking without a test failing. Sixteen defects were found in
/// it by hand and by review, of which four were a mark that stacked: an
/// indented one, one under an attribute, one with a blank line beneath it, one
/// whose sentence had drifted. Every one of them passed the example tests,
/// which asserted that the mark *appeared* and never that a second pass changed
/// nothing.
///
/// So it is stated as a property instead, over the shapes that broke it and the
/// ones nobody has tried yet:
///
/// - running it twice is running it once;
/// - a file it has nothing to say about comes back byte for byte;
/// - it never removes one of the file's own lines.
fn a_file() -> impl Strategy<Value = String> {
    let line = prop::sample::select(vec![
        "use a::b::C;".to_string(),
        "use a::b::{C, D};".to_string(),
        "use a::b::D;".to_string(),
        "  use a::b::C;".to_string(),
        "#[allow(unused_imports)]".to_string(),
        "/// what this is for".to_string(),
        "// maxx: C is imported twice — one of these two lines has to go.".to_string(),
        "  // maxx: C is imported twice — one of these two lines has to go.".to_string(),
        "".to_string(),
        "mod tests {".to_string(),
        "}".to_string(),
        "fn main() {}".to_string(),
        "const T: &str = \"use a::b::C;\";".to_string(),
    ]);
    (prop::collection::vec(line, 0..9), any::<bool>(), any::<bool>()).prop_map(
        |(lines, crlf, final_newline)| {
            let ending = if crlf { "\r\n" } else { "\n" };
            let mut out = lines.join(ending);
            if final_newline && !out.is_empty() {
                out.push_str(ending);
            }
            out
        },
    )
}

proptest! {
    #![proptest_config(ProptestConfig { cases: 2000, ..ProptestConfig::default() })]

    /// Saying it twice is saying it once.
    #[test]
    fn the_import_pass_is_a_fixed_point(source in a_file()) {
        let once = maxx::view::flag_duplicate_imports_for_test(source);
        let twice = maxx::view::flag_duplicate_imports_for_test(once.clone());
        prop_assert_eq!(&twice, &once, "\n--- once ---\n{:?}", once);
    }

    /// And it never takes away a line the file already had.
    ///
    /// Its own marks aside — those it may take back, and only those.
    #[test]
    fn the_import_pass_only_ever_adds(source in a_file()) {
        let out = maxx::view::flag_duplicate_imports_for_test(source.clone());
        for line in source.lines().filter(|line| !line.trim().starts_with("// maxx: ")) {
            prop_assert!(
                out.lines().any(|kept| kept == line),
                "line gone: {:?}\n--- out ---\n{:?}",
                line,
                out
            );
        }
    }
}
