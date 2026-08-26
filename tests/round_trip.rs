//! The round-trip contract: what the designer writes, it must be able to read
//! back, and what a human writes by hand must survive being read and rewritten.

use maxx::codegen::render;
use maxx::model::{Arg, Base, Node};
use maxx::parser;

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

/// Parses the region, re-renders it, and returns the rendered text.
fn reparse(expr: &str) -> String {
    let file = file_with(expr);
    let (node, _) = parser::parse(&file).expect("the region should parse");
    render(&node, 0)
}

#[test]
fn simple_chain_survives() {
    let source = "v_flex().gap_2().child(Label::new(\"Nom\"))";
    assert_eq!(reparse(source), source);
}

#[test]
fn nested_children_survive() {
    let source = "\
v_flex()
    .gap_3()
    .p_4()
    .child(Label::new(\"Nom\"))
    .child(
        h_flex()
            .gap_2()
            .child(div().flex_1())
            .child(Button::new(\"valider\").label(\"Valider\")),
    )";
    assert_eq!(reparse(source), source);
}

#[test]
fn unknown_method_is_kept() {
    // `.shadow_lg()` and `.bg(rgb(0x1e2127))` are not in the registry; the
    // model must still carry them and emit them unchanged.
    let source = "div().shadow_lg().bg(rgb(0x1e2127))";
    let file = file_with(source);
    let (node, _) = parser::parse(&file).expect("the region should parse");

    assert_eq!(node.calls.len(), 2);
    assert_eq!(node.calls[0].name, "shadow_lg");
    assert_eq!(node.calls[1].args[0], Arg::Verbatim("rgb(0x1e2127)".into()));
    assert_eq!(render(&node, 0), source);
}

#[test]
fn unparsable_expression_degrades_to_opaque() {
    let source = "if self.busy { spinner() } else { Label::new(\"ready\") }";
    let file = file_with(source);
    let (node, _) = parser::parse(&file).expect("the region should parse");

    assert!(node.is_opaque(), "an `if` is not a builder chain");
    assert_eq!(render(&node, 0), source, "it must come back out unchanged");
}

#[test]
fn arguments_keep_their_kind() {
    let source = "Button::new(\"ok\").label(\"Valider\").disabled(true).w(px(120.))";
    let file = file_with(source);
    let (node, _) = parser::parse(&file).expect("the region should parse");

    match &node.base {
        Base::Known { path, args } => {
            assert_eq!(path, "Button::new");
            assert_eq!(args[0], Arg::Str("ok".into()));
        }
        Base::Opaque(_) => panic!("Button::new is a recognized constructor"),
    }
    assert_eq!(node.call("label").unwrap().args[0], Arg::Str("Valider".into()));
    assert_eq!(node.call("disabled").unwrap().args[0], Arg::Bool(true));
    assert_eq!(node.call("w").unwrap().args[0], Arg::Verbatim("px(120.)".into()));
}

#[test]
fn rendering_is_idempotent() {
    let mut node = Node::known("v_flex");
    node.calls.push(maxx::model::Call::bare("gap_2"));
    for label in ["Nom", "Prénom", "Adresse électronique du contact"] {
        let mut child = Node::known("Label::new");
        if let Base::Known { args, .. } = &mut child.base {
            args.push(Arg::Str(label.into()));
        }
        node.push_child(child);
    }

    let once = render(&node, 0);
    let twice = render(&node, 0);
    assert_eq!(once, twice, "rendering the same model must be stable");

    let file = file_with(&once);
    let (reparsed, _) = parser::parse(&file).expect("the region should parse");
    assert_eq!(reparsed, node, "the model must survive a trip through source");
    assert_eq!(render(&reparsed, 0), once);
}

#[test]
fn splicing_preserves_everything_outside_the_markers() {
    let file = "\
use gpui::*;

/// A hand-written comment, which has to survive.
impl Render for Home {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        // maxx:begin
        v_flex()
        // maxx:end
    }

    /// A hand-added method.
    fn validate(&mut self) {}
}
";
    let spliced = parser::splice(file, "h_flex().gap_2()").expect("markers are present");

    assert!(spliced.contains("A hand-written comment"));
    assert!(spliced.contains("A hand-added method"));
    assert!(spliced.contains("        h_flex().gap_2()\n"));
    assert!(!spliced.contains("v_flex()"));

    // And the spliced file is still readable.
    let (node, _) = parser::parse(&spliced).expect("the region should parse");
    assert_eq!(node.base.path(), Some("h_flex"));
}

#[test]
fn a_file_without_markers_is_refused_not_rewritten() {
    let file = "fn main() {}\n";
    assert!(matches!(parser::parse(file), Err(parser::Error::NoMarkers)));
    assert!(matches!(parser::splice(file, "v_flex()"), Err(parser::Error::NoMarkers)));
}

#[test]
fn a_multiline_opaque_expression_does_not_drift() {
    // The bug this guards: the opaque slice kept its file indentation, and
    // `splice` added the region indent again on every save.
    let expr = "if self.busy {\n    spinner()\n} else {\n    Label::new(\"ready\")\n}";
    let mut file = file_with(expr);

    for _ in 0..3 {
        let (node, region) = parser::parse(&file).expect("the region should parse");
        assert!(node.is_opaque());
        let block = maxx::codegen::render_for_splice(&node, region.width());
        file = parser::splice(&file, &block).expect("markers are present");
    }

    let saved = file
        .lines()
        .find(|line| line.contains("spinner()"))
        .expect("the expression is still there");
    assert_eq!(saved, "            spinner()", "the indentation must not grow on every save");
}

#[test]
fn moving_a_node_into_a_later_sibling_works() {
    let mut root = Node::known("v_flex");
    root.push_child(Node::known("Label::new"));
    root.push_child(Node::known("h_flex"));

    assert_eq!(root.move_node(&[0], &[1, 0]), Some(vec![0, 0]));
    assert_eq!(root.children.len(), 1);
    assert_eq!(root.children[0].base.path(), Some("h_flex"));
    assert_eq!(root.children[0].children.len(), 1);
    assert_eq!(root.children[0].children[0].base.path(), Some("Label::new"));
}

#[test]
fn a_hand_written_argument_is_not_overwritten_by_the_inspector() {
    let source = "Button::new(cx.entity_id()).label(\"Valider\")";
    let file = file_with(source);
    let (mut node, _) = parser::parse(&file).expect("the region should parse");

    let spec = maxx::registry::of(&node).expect("Button is in the catalogue");
    let id_prop =
        spec.props.iter().find(|prop| prop.label == "prop.id").expect("Button has an id property");

    assert!(!maxx::registry::editable(&node, id_prop), "a hand-written argument is not editable");
    maxx::registry::write(&mut node, id_prop, "ok");
    assert_eq!(maxx::codegen::render(&node, 0), source, "the original expression must be intact");
}

#[test]
fn an_invalid_field_name_is_refused() {
    let mut node = maxx::registry::instantiate("input").expect("input is in the catalogue");
    let spec = maxx::registry::of(&node).expect("Input is in the catalogue");
    let prop = &spec.props[0];

    for refused in ["", "my field", "2field", "champ-x"] {
        maxx::registry::write(&mut node, prop, refused);
        assert_eq!(
            maxx::codegen::render(&node, 0),
            "Input::new(&self.field)",
            "`{refused}` must not be written into the source"
        );
    }

    maxx::registry::write(&mut node, prop, "address");
    assert_eq!(maxx::codegen::render(&node, 0), "Input::new(&self.address)");
}

#[test]
fn a_node_cannot_be_dropped_into_itself() {
    let mut root = Node::known("v_flex");
    let mut column = Node::known("v_flex");
    column.push_child(Node::known("Label::new"));
    root.push_child(column);

    // Into its own child.
    assert_eq!(root.move_node(&[0], &[0, 0]), None);
    // The tree is untouched.
    assert_eq!(root.children.len(), 1);
    assert_eq!(root.children[0].children.len(), 1);
}

#[test]
fn dropping_before_an_earlier_sibling_keeps_the_order() {
    let mut root = Node::known("v_flex");
    for label in ["a", "b", "c"] {
        let mut child = Node::known("Label::new");
        if let Base::Known { args, .. } = &mut child.base {
            args.push(Arg::Str(label.into()));
        }
        root.push_child(child);
    }

    // Move the third child to the front.
    assert_eq!(root.move_node(&[2], &[0]), Some(vec![0]));
    let labels: Vec<String> = root
        .children
        .iter()
        .map(|child| match &child.base {
            Base::Known { args, .. } => args[0].as_str().unwrap().to_string(),
            Base::Opaque(_) => String::new(),
        })
        .collect();
    assert_eq!(labels, ["c", "a", "b"]);
}

#[test]
fn an_invalid_value_is_explained_not_swallowed() {
    let node = maxx::registry::instantiate("button").unwrap();
    let spec = maxx::registry::of(&node).unwrap();
    let width =
        maxx::registry::props(spec).into_iter().find(|prop| prop.label == "prop.width").unwrap();
    let colour = maxx::registry::props(spec)
        .into_iter()
        .find(|prop| prop.label == "prop.background")
        .unwrap();

    assert!(maxx::registry::validate(width, "120").is_none());
    assert!(maxx::registry::validate(width, "").is_none());
    assert!(maxx::registry::validate(width, "large").is_some());

    assert!(maxx::registry::validate(colour, "#1e2127").is_none());
    assert!(maxx::registry::validate(colour, "1e2127").is_none());
    assert!(maxx::registry::validate(colour, "").is_none());
    assert!(maxx::registry::validate(colour, "rouge").is_some());
    assert!(maxx::registry::validate(colour, "1e21").is_some());
}

#[test]
fn the_selection_survives_an_undo() {
    // Deleting the second child then undoing must not send the selection back
    // to the root when the node it pointed at is there again.
    let mut root = Node::known("v_flex");
    root.push_child(Node::known("Label::new"));
    root.push_child(Node::known("Label::new"));

    let before = root.clone();
    root.remove(&[1]).unwrap();
    assert!(root.at(&[1]).is_none(), "the node has indeed gone");

    root = before;
    assert!(root.at(&[1]).is_some(), "and it is back after the undo, so the selection holds");
}

#[test]
fn interleaved_children_keep_their_place() {
    // Lifting every child to the end of the chain moved a header below a list.
    let source = "v_flex().child(header()).children(self.rows()).child(footer())";
    assert_eq!(reparse(source), source);

    let conditional = "v_flex().child(a()).when(self.large, |d| d.child(b())).child(c())";
    assert_eq!(reparse(conditional), conditional);
}

#[test]
fn a_brace_in_a_comment_does_not_end_a_block() {
    let source = "impl Home {\n    /// Closes the panel } and resets everything.\n    pub fn r(&mut self) {}\n}\n";
    let open = source.find('{').unwrap();
    let close = maxx::parser::matching_brace(source, open).unwrap();
    assert_eq!(&source[close..], "}\n", "the block closes on the right brace");

    let with_string = "fn f() { let s = \"} not a brace\"; }\n";
    let open = with_string.find('{').unwrap();
    let close = maxx::parser::matching_brace(with_string, open).unwrap();
    assert_eq!(&with_string[close..], "}\n");
}

#[test]
fn a_multiline_string_is_not_reindented() {
    let file = file_with("v_flex()");
    let block = "div().child(\n    \"line one\nline two\",\n)";
    let spliced = parser::splice(&file, block).expect("markers are present");
    assert!(
        spliced.contains("\nline two\","),
        "the spaces must not get into the string:\n{spliced}"
    );
}

#[test]
fn a_length_must_be_a_rust_literal() {
    let mut node = maxx::registry::instantiate("button").unwrap();
    let spec = maxx::registry::of(&node).unwrap();
    let width =
        maxx::registry::props(spec).into_iter().find(|prop| prop.label == "prop.width").unwrap();

    for refused in [".5", "inf", "NaN", "-inf", "12px", "1.2.3"] {
        maxx::registry::write(&mut node, width, refused);
        assert!(maxx::registry::validate(width, refused).is_some(), "`{refused}` must be reported");
        assert!(
            !maxx::codegen::render(&node, 0).contains("px("),
            "`{refused}` must not reach the file"
        );
    }

    maxx::registry::write(&mut node, width, "120");
    assert!(maxx::codegen::render(&node, 0).contains(".w(px(120.))"));
    maxx::registry::write(&mut node, width, "12.5");
    assert!(maxx::codegen::render(&node, 0).contains(".w(px(12.5))"));
}

#[test]
fn a_hand_written_method_argument_is_protected() {
    let source = "Button::new(\"ok\").label(t!(\"valider\"))";
    let file = file_with(source);
    let (node, _) = parser::parse(&file).expect("the region should parse");
    let spec = maxx::registry::of(&node).unwrap();
    let label = spec.props.iter().find(|p| p.label == "prop.label").unwrap();

    assert!(
        !maxx::registry::editable(&node, label),
        "a hand-written expression is not edited as free text"
    );
}

#[test]
fn a_multiline_string_survives_a_full_save_cycle() {
    // `splice` stopped indenting inside literals, but `dedent` still stripped
    // them on the way back in, so the fix was one-sided and the string drifted
    // a little on every save.
    let mut file = String::from(
        "impl Render for Home {\n\
         \x20   fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {\n\
         \x20       // maxx:begin\n\
         \x20       div().child(\n\
         \"line one\n\
         \x20 line two\",\n\
         \x20       )\n\
         \x20       // maxx:end\n\
         \x20   }\n\
         }\n",
    );

    let mut seen = Vec::new();
    for _ in 0..3 {
        let (node, region) = parser::parse(&file).expect("the region should parse");
        let block = maxx::codegen::render_for_splice(&node, region.width());
        file = parser::splice(&file, &block).expect("markers are present");
        seen.push(file.clone());
    }

    assert_eq!(seen[0], seen[1], "the file must not move any more");
    assert_eq!(seen[1], seen[2]);
    assert!(
        seen[0].contains("\n  line two\""),
        "the string's indentation must stay intact:\n{}",
        seen[0]
    );
}

#[test]
fn a_lifetime_is_not_a_char_literal() {
    let source = "impl Foo {\n    fn f<'a>() { let c = 'x'; }\n    fn g() {}\n}\n";
    let open = source.find('{').unwrap();
    let close = maxx::parser::matching_brace(source, open).unwrap();
    assert_eq!(&source[close..], "}\n", "the block must close on the impl's brace");
}

/// Every catalogue entry writes, reads back, and lands on itself again.
///
/// The catalogue is a table, so adding a line to it costs nothing — and that
/// is exactly why a wrong line goes unnoticed: an `import` that does not match
/// the `base`, or a property whose target does not exist on the component,
/// only shows when the generated project is compiled.
#[test]
fn every_catalogue_entry_writes_and_reads_back() {
    for spec in maxx::registry::CATALOGUE {
        let node = maxx::registry::instantiate(spec.id)
            .unwrap_or_else(|| panic!("{} must instantiate", spec.id));
        assert_eq!(
            node.base.path(),
            Some(spec.base),
            "{}: the base written is not the table's",
            spec.id
        );
        assert_eq!(
            maxx::registry::of(&node).map(|found| found.id),
            Some(spec.id),
            "{}: the node written is not found in the catalogue",
            spec.id
        );
        assert!(
            spec.import.starts_with("use ") && spec.import.ends_with(';'),
            "{}: the import is not a complete `use` line",
            spec.id
        );
    }
}

/// A bare number is written without `px`, unlike a length.
///
/// `Progress::value` takes an `f32`: handing it `px(50.)` would not compile
/// in the generated project, and the error would only appear then.
#[test]
fn a_plain_number_is_written_without_px() {
    let mut node = maxx::registry::instantiate("progress").unwrap();
    let spec = maxx::registry::of(&node).unwrap();
    let value =
        maxx::registry::props(spec).into_iter().find(|prop| prop.label == "prop.value").unwrap();

    assert!(maxx::registry::validate(value, "50").is_none());
    assert!(maxx::registry::validate(value, "12.5").is_none());
    assert!(maxx::registry::validate(value, "").is_none());
    assert!(maxx::registry::validate(value, "beaucoup").is_some());

    maxx::registry::write(&mut node, value, "50");
    assert_eq!(node.call("value").unwrap().args[0].to_source(), "50.");
    assert_eq!(maxx::registry::read(&node, value).as_deref(), Some("50"));

    maxx::registry::write(&mut node, value, "12.5");
    assert_eq!(node.call("value").unwrap().args[0].to_source(), "12.5");

    // Emptied, the call disappears rather than being written as zero.
    maxx::registry::write(&mut node, value, "");
    assert!(node.call("value").is_none());
}

/// A copied subtree reads back from the text the clipboard carries.
///
/// The clipboard carries no format of maxx's own but Rust: what is copied
/// here pastes into Zed, and what is written there pastes here. Both ends of
/// the trip therefore go through `codegen::render` and `parser::parse_expr`,
/// and this test is what keeps them agreeing.
#[test]
fn a_subtree_survives_the_clipboard() {
    let mut column = maxx::registry::instantiate("column").unwrap();
    let mut button = maxx::registry::instantiate("button").unwrap();
    let spec = maxx::registry::of(&button).unwrap();
    let label = spec.props.iter().find(|prop| prop.label == "prop.label").unwrap();
    maxx::registry::write(&mut button, label, "Envoyer");
    column.push_child(button);
    column.push_child(maxx::registry::instantiate("input").unwrap());

    let source = maxx::codegen::render(&column, 0);
    let back = maxx::parser::parse_expr(&source).expect("the copied text must read back");
    assert_eq!(back, column, "the trip through text must change nothing");
}

/// What is not a gpui expression does not become an opaque node.
#[test]
fn clipboard_prose_is_not_an_expression() {
    let node = maxx::parser::parse_expr("hello, this is not Rust");
    assert!(node.is_err(), "arbitrary text must be refused, not adopted");
}

/// A copied state field is bound to a new field, not to the original's.
///
/// Two `Input`s on `&self.field` compile and copy each other at run time:
/// the defect only shows when the project is run.
#[test]
fn a_copied_input_gets_a_field_of_its_own() {
    let mut root = maxx::registry::instantiate("column").unwrap();
    root.push_child(maxx::registry::instantiate("input").unwrap());

    let mut copy = root.children[0].clone();
    maxx::registry::rebind_state_fields(&mut copy, &root);
    root.push_child(copy);

    let first = maxx::registry::read(&root.children[0], binding()).unwrap();
    let second = maxx::registry::read(&root.children[1], binding()).unwrap();
    assert_eq!(first, "field");
    assert_eq!(second, "field_2", "the second field cannot be the first");

    // And a subtree carrying two of them tells those apart too.
    let mut pair = maxx::registry::instantiate("column").unwrap();
    pair.push_child(maxx::registry::instantiate("input").unwrap());
    pair.push_child(maxx::registry::instantiate("input").unwrap());
    maxx::registry::rebind_state_fields(&mut pair, &root);
    let names: Vec<_> =
        pair.children.iter().map(|child| maxx::registry::read(child, binding()).unwrap()).collect();
    assert_eq!(names, vec!["field_3", "field_4"]);
}

/// The text input's "Bound field" property.
fn binding() -> &'static maxx::registry::Prop {
    let spec = maxx::registry::by_id("input").unwrap();
    spec.props.iter().find(|prop| prop.label == "prop.bound_field").unwrap()
}

/// The catalogue search answers the label, the identifier, and the accents.
///
/// On the pure function rather than on `matches_query`: that one translates
/// the label, so its result depends on the language, and the language is a
/// global the tests of one binary share in parallel.
#[test]
fn the_palette_search_forgives_the_accents() {
    let matches = maxx::designer::label_matches;

    // An empty search hides nothing.
    assert!(matches("Étiquette", "label", ""));

    // The label, whatever the case and the accents: nobody types
    // "Étiquette" with its accent in a search box.
    assert!(matches("Étiquette", "label", "Étiquette"));
    assert!(matches("Étiquette", "label", "etiquette"));
    assert!(matches("Séparateur", "divider", "separateur"));

    // The identifier too: it is what someone who has read the generated code types.
    assert!(matches("Étiquette", "label", "label"));
    assert!(!matches("Étiquette", "label", "bouton"));

    // And the English label answers the English word.
    assert!(matches("Progress bar", "progress", "progress"));
}

/// A hand-written binding that collides with nothing is kept as it is.
///
/// It is the counterpart of the clipboard's promise: what is written in Zed
/// comes back here. Renaming `&self.search` to `&self.field` would declare a
/// second field for the one it already has.
#[test]
fn a_binding_that_collides_with_nothing_is_kept() {
    let mut root = maxx::registry::instantiate("column").unwrap();
    root.push_child(maxx::registry::instantiate("input").unwrap());

    let mut pasted = maxx::parser::parse_expr("Input::new(&self.search)").expect("must read back");
    maxx::registry::rebind_state_fields(&mut pasted, &root);
    assert_eq!(maxx::registry::read(&pasted, binding()).as_deref(), Some("search"));

    // And the one that collides is renamed.
    let mut collides = maxx::parser::parse_expr("Input::new(&self.field)").expect("must read back");
    maxx::registry::rebind_state_fields(&mut collides, &root);
    assert_eq!(maxx::registry::read(&collides, binding()).as_deref(), Some("field_2"));
}

/// An image path is an expression, and it has to survive as one.
///
/// `Kind::Path` is the only property written as neither a literal nor a
/// `&self.` binding: it goes out as `PathBuf::from("…")`, comes back through
/// `syn`, and the inspector has to recognise its own writing on the way in —
/// otherwise the field shows the expression and the next keystroke overwrites
/// the whole argument.
#[test]
fn an_image_path_is_written_read_back_and_still_editable() {
    let mut node = maxx::registry::instantiate("image").expect("image is in the catalogue");
    let spec = maxx::registry::of(&node).expect("img is in the catalogue");
    let source = &spec.props[0];

    let rendered = maxx::codegen::render(&node, 0);
    assert_eq!(rendered, "img(PathBuf::from(\"assets/images/image.png\"))");

    let back = maxx::parser::parse_expr(&rendered).expect("the expression must read back");
    assert_eq!(maxx::registry::read(&back, source).as_deref(), Some("assets/images/image.png"));
    assert!(maxx::registry::editable(&back, source), "the inspector must own this argument");

    maxx::registry::write(&mut node, source, "assets/logo.png");
    assert_eq!(maxx::codegen::render(&node, 0), "img(PathBuf::from(\"assets/logo.png\"))");

    // The import travels with it: `PathBuf` sits on the base, not on a call,
    // and nothing else in the tree would bring it in.
    let mut root = maxx::model::Node::known("v_flex");
    root.push_child(node.clone());
    assert!(maxx::registry::imports(&root).contains(&"use std::path::PathBuf;"));

    // An absolute path is refused rather than written: it would resolve on one
    // machine only. Refused by `write` too, and not only reported by
    // `validate` — the inspector writes first and says afterwards, so a kind
    // that only complains still lets the value reach the file.
    assert!(maxx::registry::validate(source, "/Users/someone/logo.png").is_some());
    assert!(maxx::registry::validate(source, "assets/logo.png").is_none());
    maxx::registry::write(&mut node, source, "/Users/someone/logo.png");
    assert_eq!(maxx::codegen::render(&node, 0), "img(PathBuf::from(\"assets/logo.png\"))");
}

/// A path is decoded the way it was encoded, escape for escape.
///
/// Undoing `\\` alone left `\"` behind, and the next write escaped its
/// backslash again: the argument grew by one on every keystroke.
#[test]
fn an_awkward_file_name_does_not_grow_on_every_edit() {
    let mut node = maxx::registry::instantiate("image").expect("image is in the catalogue");
    let spec = maxx::registry::of(&node).expect("img is in the catalogue");
    let source = &spec.props[0];

    let awkward = "assets/a \"quoted\" name.png";
    maxx::registry::write(&mut node, source, awkward);
    let rendered = maxx::codegen::render(&node, 0);

    let back = maxx::parser::parse_expr(&rendered).expect("the expression must read back");
    assert_eq!(maxx::registry::read(&back, source).as_deref(), Some(awkward));

    // And writing what was read back changes nothing more.
    let mut again = back;
    maxx::registry::write(&mut again, source, awkward);
    assert_eq!(maxx::codegen::render(&again, 0), rendered);
}

/// A hand-written source expression is shown and never overwritten.
#[test]
fn a_computed_image_source_is_left_alone() {
    let node = maxx::parser::parse_expr("img(self.avatar.clone())").expect("must read back");
    let spec = maxx::registry::of(&node).expect("img is in the catalogue");
    let source = &spec.props[0];

    assert!(!maxx::registry::editable(&node, source));
    assert_eq!(maxx::registry::read(&node, source).as_deref(), Some("self.avatar.clone()"));

    let mut node = node;
    maxx::registry::write(&mut node, source, "assets/logo.png");
    assert_eq!(maxx::codegen::render(&node, 0), "img(self.avatar.clone())");
}
