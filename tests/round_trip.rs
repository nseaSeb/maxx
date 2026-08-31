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
            spec.imports.iter().all(|line| line.starts_with("use ") && line.ends_with(';')),
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
/// `&self.` binding: it goes out as a string relative to the root, comes back
/// through `syn`, and the inspector has to recognise its own writing on the way
/// in — otherwise the field shows the expression and the next keystroke
/// overwrites the whole argument.
#[test]
fn an_image_path_is_written_read_back_and_still_editable() {
    let mut node = maxx::registry::instantiate("image").expect("image is in the catalogue");
    let spec = maxx::registry::of(&node).expect("img is in the catalogue");
    let source = &spec.props[0];

    let rendered = maxx::codegen::render(&node, 0);
    // Dropped fitting: a photograph is wider than any view, and the switch is
    // there for whoever wants it at its own size.
    assert_eq!(rendered, "img(\"assets/images/image.png\").max_w_full()");

    let back = maxx::parser::parse_expr(&rendered).expect("the expression must read back");
    assert_eq!(maxx::registry::read(&back, source).as_deref(), Some("assets/images/image.png"));
    assert!(maxx::registry::editable(&back, source), "the inspector must own this argument");

    maxx::registry::write(&mut node, source, "assets/logo.png");
    assert_eq!(maxx::codegen::render(&node, 0), "img(\"assets/logo.png\").max_w_full()");

    // No `PathBuf` import to bring in any more, and that is the point: a string
    // is what gpui looks up in the `AssetSource`, a path is what it reads from
    // the working directory.
    let mut root = maxx::model::Node::known("v_flex");
    root.push_child(node.clone());
    assert!(!maxx::registry::imports(&root).contains(&"use std::path::PathBuf;"));
    assert!(maxx::registry::uses_an_asset(&root), "the tree owes the project an AssetSource");

    // An absolute path is refused rather than written: it would resolve on one
    // machine only. Refused by `write` too, and not only reported by
    // `validate` — the inspector writes first and says afterwards, so a kind
    // that only complains still lets the value reach the file.
    assert!(maxx::registry::validate(source, "/Users/someone/logo.png").is_some());
    assert!(maxx::registry::validate(source, "assets/logo.png").is_none());
    maxx::registry::write(&mut node, source, "/Users/someone/logo.png");
    assert_eq!(maxx::codegen::render(&node, 0), "img(\"assets/logo.png\").max_w_full()");
}

/// A project written by an older maxx keeps the spelling it has.
///
/// `PathBuf::from("…")` and `"…"` do not mean the same thing at runtime — one
/// is read from the working directory, the other is looked up in the
/// application's `AssetSource` — so flipping an existing one under the
/// developer would change what their project does, on a line they never
/// touched. And it would leave the file with a `use std::path::PathBuf;`
/// nothing uses, because `imports` adds and never prunes.
#[test]
fn an_older_project_keeps_the_path_form_it_had() {
    let source = "img(PathBuf::from(\"assets/images/old.png\")).max_w_full()";
    let mut node = maxx::parser::parse_expr(source).expect("the expression must read back");
    let spec = maxx::registry::of(&node).expect("img is in the catalogue");
    let prop = &spec.props[0];

    assert_eq!(
        maxx::registry::read(&node, prop).as_deref(),
        Some("assets/images/old.png"),
        "the older spelling still reads back"
    );
    assert!(maxx::registry::editable(&node, prop), "and stays the inspector's to write");

    maxx::registry::write(&mut node, prop, "assets/images/new.png");
    assert_eq!(
        maxx::codegen::render(&node, 0),
        "img(PathBuf::from(\"assets/images/new.png\")).max_w_full()"
    );

    let mut root = maxx::model::Node::known("v_flex");
    root.push_child(node);
    assert!(maxx::registry::imports(&root).contains(&"use std::path::PathBuf;"));
    assert!(!maxx::registry::uses_an_asset(&root), "a path owes the project nothing");
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

    // Quote, backslash, tab: the three that `escape` writes as two characters,
    // and the three a naive decoder gives back as the letter that followed.
    let awkward = "assets/a \"quoted\"\tname\\here.png";
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

/// Scrolling is two calls, not one, and the id is what makes it work.
///
/// gpui keeps a scroll offset per element id: the overflow flag on its own
/// clips the content and never moves it. The pair has to be written together,
/// and the id has to be one no sibling is using.
#[test]
fn scrolling_writes_the_pair_it_needs() {
    let mut column = maxx::registry::instantiate("column").expect("column is in the catalogue");
    let spec = maxx::registry::of(&column).expect("v_flex is in the catalogue");
    let scroll =
        spec.props.iter().find(|prop| prop.label == "prop.scroll").expect("a column scrolls");

    maxx::registry::write(&mut column, scroll, "true");
    let rendered = maxx::codegen::render(&column, 0);
    assert!(rendered.contains(".overflow_y_scroll()"), "{rendered}");
    assert!(rendered.contains(".id("), "the offset needs somewhere to live: {rendered}");
    // And before the overflow: `overflow_y_scroll` lives on a stateful element,
    // so gpui only offers it once `id` has been called. Written the other way
    // round, the chain does not compile — in the developer's project, not here.
    let id = rendered.find(".id(").expect("just asserted");
    let overflow = rendered.find(".overflow_y_scroll(").expect("just asserted");
    assert!(id < overflow, "the id has to come first: {rendered}");
    // And a box whose height follows its content grows instead of scrolling.
    assert!(rendered.contains(".h_full()"), "the axis that scrolls is held: {rendered}");

    // It reads back as what it is, and the catalogue owns the call rather than
    // showing it among the ones it does not know.
    let back = maxx::parser::parse_expr(&rendered).expect("must read back");
    assert_eq!(maxx::registry::read(&back, scroll).as_deref(), Some("true"));
    assert!(maxx::registry::covers(spec, "overflow_y_scroll"));

    // Turned off, the overflow goes and the rest stays. maxx cannot tell its
    // own `h_full` from one written by hand, and deleting a layout call nobody
    // asked it to touch is worse than leaving one that shows in the inspector.
    let mut back = back;
    maxx::registry::write(&mut back, scroll, "false");
    let rendered = maxx::codegen::render(&back, 0);
    assert!(!rendered.contains("overflow_y_scroll"), "{rendered}");
    assert!(rendered.contains(".h_full()"), "{rendered}");
    assert!(rendered.contains(".id("), "{rendered}");
    // And the switch owns the overflow alone: the two calls it leaves behind
    // are shown among the ones the catalogue does not know, under their own
    // names, for whoever wants them gone.
    assert!(!maxx::registry::covers(spec, "h_full"), "the hold is left visible");
    assert!(!maxx::registry::covers(spec, "id"), "so is the id");

    // A height set by hand says what to hold the box to already.
    let mut sized = maxx::registry::instantiate("column").expect("column is in the catalogue");
    let height =
        spec.props.iter().chain(maxx::registry::COMMON).find(|prop| prop.label == "prop.height");
    maxx::registry::write(&mut sized, height.expect("a column has a height"), "300");
    maxx::registry::write(&mut sized, scroll, "true");
    let rendered = maxx::codegen::render(&sized, 0);
    assert!(!rendered.contains(".h_full()"), "{rendered}");
    assert!(rendered.contains(".h(px(300.))"), "{rendered}");

    // A row scrolls sideways, because that is where its content overflows.
    let row = maxx::registry::instantiate("row").expect("row is in the catalogue");
    let spec = maxx::registry::of(&row).expect("h_flex is in the catalogue");
    assert!(maxx::registry::covers(spec, "overflow_x_scroll"));
}

/// The id handed out is one no node of the tree is already using.
#[test]
fn a_second_scrolling_container_gets_its_own_id() {
    let mut root = maxx::model::Node::known("v_flex");
    let mut first = maxx::model::Node::known("v_flex");
    first.set_call("id", maxx::model::Arg::Str("scroll".into()));
    root.push_child(first);

    assert_eq!(maxx::registry::unique_element_id(&root), "scroll_2");
}

/// A path that climbs out of the project is refused like an absolute one.
///
/// `../../Desktop/logo.png` resolves here — the canvas draws it, and so does
/// the binary on this machine — and nowhere else. That is the whole reason the
/// property refuses anything but a path relative to the root.
#[test]
fn a_path_that_leaves_the_project_is_refused() {
    let mut node = maxx::registry::instantiate("image").expect("image is in the catalogue");
    let spec = maxx::registry::of(&node).expect("img is in the catalogue");
    let source = &spec.props[0];

    for refused in ["/Users/someone/logo.png", "../../Desktop/logo.png", "..\\logo.png"] {
        assert!(maxx::registry::validate(source, refused).is_some(), "{refused}");
        maxx::registry::write(&mut node, source, refused);
        assert_eq!(
            maxx::codegen::render(&node, 0),
            "img(\"assets/images/image.png\").max_w_full()",
            "{refused}"
        );
    }
    assert!(maxx::registry::validate(source, "assets/images/logo.png").is_none());
}

/// What the table says a component needs on its first frame, it gets.
///
/// And the two questions an image raises are kept apart: how its box relates to
/// what holds it, and how the picture fills that box.
#[test]
fn a_dropped_image_fits_and_the_two_sizes_are_separate() {
    let mut node = maxx::registry::instantiate("image").expect("image is in the catalogue");
    let spec = maxx::registry::of(&node).expect("img is in the catalogue");
    let size = spec.props.iter().find(|prop| prop.label == "prop.size").expect("an image sizes");
    let fit = spec.props.iter().find(|prop| prop.label == "prop.fit").expect("an image fits");

    assert_eq!(maxx::registry::read(&node, size).as_deref(), Some("max_w_full"));
    assert!(maxx::registry::covers(spec, "max_w_full"), "the choice owns the call it writes");

    maxx::registry::write(&mut node, size, "w_full");
    let rendered = maxx::codegen::render(&node, 0);
    assert!(rendered.contains(".w_full()") && !rendered.contains("max_w_full"), "{rendered}");

    // The fill mode is an enum variant, written out as it goes into the file,
    // and it brings its type with it.
    maxx::registry::write(&mut node, fit, "ObjectFit::Cover");
    let rendered = maxx::codegen::render(&node, 0);
    assert!(rendered.contains(".object_fit(ObjectFit::Cover)"), "{rendered}");

    let mut root = maxx::model::Node::known("v_flex");
    root.push_child(node.clone());
    assert!(maxx::registry::imports(&root).contains(&"use gpui::ObjectFit;"));

    // Read back from what was written, and refused when it is not one of them.
    let back = maxx::parser::parse_expr(&rendered).expect("must read back");
    assert_eq!(maxx::registry::read(&back, fit).as_deref(), Some("ObjectFit::Cover"));
    maxx::registry::write(&mut node, fit, "ObjectFit::Whatever");
    assert!(maxx::codegen::render(&node, 0).contains("ObjectFit::Cover"));
}

/// The shared style properties follow the component.
///
/// An image draws no text of its own: offering it a font weight is five rows
/// that do nothing, drowning the two that matter.
#[test]
fn an_image_is_not_offered_text_properties() {
    let image = maxx::registry::by_id("image").expect("image is in the catalogue");
    let label = maxx::registry::by_id("label").expect("label is in the catalogue");

    let labels =
        |spec| maxx::registry::props(spec).iter().map(|prop| prop.label).collect::<Vec<_>>();
    assert!(!labels(image).contains(&"prop.weight"), "{:?}", labels(image));
    assert!(labels(image).contains(&"prop.width"), "a box still has a size");
    assert!(labels(label).contains(&"prop.weight"), "a label writes text");
}

/// The id handed out avoids the ones written as constructor arguments too.
///
/// A button, a checkbox, a switch carry theirs as an argument rather than as an
/// `id` call: looking only at the call would hand out an id one of them already
/// answers to, which is the collision the function exists to prevent.
#[test]
fn a_new_element_id_avoids_the_ones_written_as_arguments() {
    let mut root = maxx::model::Node::known("v_flex");
    let mut button = maxx::registry::instantiate("button").expect("button is in the catalogue");
    let spec = maxx::registry::of(&button).expect("Button is in the catalogue");
    let id = spec.props.iter().find(|prop| prop.label == "prop.id").expect("a button has an id");

    maxx::registry::write(&mut button, id, "scroll");
    root.push_child(button);

    assert_eq!(maxx::registry::unique_element_id(&root), "scroll_2");
}

/// Every default argument is encoded from the property that targets it.
///
/// Deciding for the whole list from the first property was right only as long
/// as the one component with a path argument had a single argument.
#[test]
fn a_second_default_argument_is_not_written_as_the_first() {
    let alert = maxx::registry::instantiate("alert").expect("alert is in the catalogue");
    let rendered = maxx::codegen::render(&alert, 0);
    assert!(!rendered.contains("PathBuf::from"), "{rendered}");
}

/// Every component the palette offers reads back exactly as it was dropped.
///
/// The catalogue writes three shapes of constructor argument — a string, a
/// binding on the view, an enumeration variant — and only the first is what a
/// naive encoder produces. An icon written as `Icon::new("IconName::Check")`
/// would round-trip perfectly here and fail to compile in the project.
#[test]
fn every_component_dropped_reads_back_as_it_was_written() {
    for spec in maxx::registry::CATALOGUE {
        let node = maxx::registry::instantiate(spec.id)
            .unwrap_or_else(|| panic!("{} must instantiate", spec.id));
        let written = render(&node, 0);
        assert_eq!(reparse(&written), written, "{}", spec.id);
    }
}

#[test]
fn an_icon_carries_its_variant_as_a_path() {
    let node = maxx::registry::instantiate("icon").expect("the icon is in the catalogue");
    let written = render(&node, 0);
    assert!(written.contains("Icon::new(IconName::Check)"), "{written}");
    // The quoted form compiles nowhere: `Icon::new` takes something that
    // converts into an `Icon`, and a string does not.
    assert!(!written.contains("\"IconName"), "{written}");
}

#[test]
fn a_stateful_component_is_dropped_bound_to_a_field() {
    // Not the text input alone: a dropdown, a slider and a colour picker are
    // entities the view owns too, and one written as `Select::new()` does not
    // compile.
    for id in ["input", "select", "slider", "color_picker"] {
        let node = maxx::registry::instantiate(id).unwrap_or_else(|| panic!("{id}"));
        let written = render(&node, 0);
        assert!(written.contains("(&self."), "{id}: {written}");
    }
}

/// A tooltip is written as the closure gpui takes, and read back out of it.
#[test]
fn a_tooltip_is_written_as_a_closure_after_the_id() {
    use maxx::registry::{self, Kind, Target};

    let prop = registry::Prop { label: "prop.tooltip", target: Target::Tooltip, kind: Kind::Text };
    let mut node = Node::known("v_flex");
    registry::write(&mut node, &prop, "Hint");

    let written = render(&node, 0);
    assert!(written.contains(".id("), "the id has to be there: {written}");
    let id = written.find(".id(").unwrap();
    let tooltip = written.find(".tooltip(").expect("the call must be written");
    assert!(id < tooltip, "gpui offers the tooltip only after the id: {written}");
    assert!(written.contains("Tooltip::new(\"Hint\").build(window, cx)"), "{written}");

    // What was written comes back as the text, not as the closure.
    assert_eq!(registry::read(&node, &prop).as_deref(), Some("Hint"));
    assert_eq!(reparse(&written), written);

    // And emptying it takes the call away again.
    registry::write(&mut node, &prop, "");
    assert!(!render(&node, 0).contains(".tooltip("), "the call must go");
}

/// A tooltip brings its type in, and it is not a prefix that says so.
#[test]
fn a_tooltip_carries_its_import() {
    use maxx::registry::{self, Kind, Target};

    let prop = registry::Prop { label: "prop.tooltip", target: Target::Tooltip, kind: Kind::Text };
    let mut node = Node::known("v_flex");
    registry::write(&mut node, &prop, "Hint");

    assert!(
        registry::imports(&node).contains(&"use gpui_component::tooltip::Tooltip;"),
        "{:?}",
        registry::imports(&node)
    );
}

/// The visible scrollbar: what the switch builds, and what reads back.
///
/// The bar is a *sibling* of the box, not a child of it: gpui moves every child
/// of a scrolling element by the scroll offset, so a bar written inside would
/// travel with the content and leave the screen.
#[test]
fn the_scrollbar_is_a_sibling_of_the_box_it_watches() {
    use maxx::registry;

    let mut box_node = Node::known("v_flex");
    box_node.set_call("id", Arg::Str("scroller".into()));
    box_node.set_flag("overflow_y_scroll", true);
    box_node.push_child(Node::known("Label::new"));
    // The box already answers to a name, so the one offered is not spent.

    let wrapper = registry::scrollbar_assembly(box_node, ["box", "bar"], "scroll");
    let written = render(&wrapper, 0);

    assert!(registry::is_scrollbar_wrapper(&wrapper), "{written}");
    assert_eq!(wrapper.children.len(), 2, "the box and the bar, side by side");
    assert!(wrapper.call("relative").is_some(), "the bar is positioned against it");
    assert!(wrapper.children[0].call("track_scroll").is_some(), "{written}");
    // Nothing of the box moved: its own children stay where they were, which is
    // what the wrapper is for.
    assert_eq!(wrapper.children[0].children.len(), 1, "{written}");
    assert!(written.contains("ScrollbarAxis::Vertical"), "{written}");
    assert_eq!(written.matches("&self.scroll").count(), 2, "one handle, shared: {written}");
    assert_eq!(reparse(&written), written);

    // And unwrapping gives the box back, handle removed.
    let back = registry::unwrap_scrollbar(&wrapper).expect("maxx wrote this wrapper");
    assert!(back.call("track_scroll").is_none());
    assert_eq!(back.children.len(), 1);
    assert!(back.call("overflow_y_scroll").is_some(), "the box still scrolls");
}

/// A row scrolls sideways, so its bar is the horizontal one.
#[test]
fn the_bar_follows_the_axis_the_box_scrolls_on() {
    let mut row = Node::known("h_flex");
    row.set_flag("overflow_x_scroll", true);
    let wrapper = maxx::registry::scrollbar_assembly(row, ["box", "bar"], "scroll");
    let written = render(&wrapper, 0);
    assert!(written.contains("ScrollbarAxis::Horizontal"), "{written}");
    // And the wrapper holds the width rather than the height.
    assert!(wrapper.call("w_full").is_some(), "{written}");
}

/// What maxx did not write, maxx does not take away.
#[test]
fn a_wrapper_the_developer_touched_is_not_unwrapped() {
    use maxx::registry;

    let mut box_node = Node::known("v_flex");
    box_node.set_flag("overflow_y_scroll", true);
    let mut wrapper = registry::scrollbar_assembly(box_node, ["box", "bar"], "scroll");

    // A node dropped beside the bar: the shape is no longer the one maxx
    // wrote, so it stays whole.
    wrapper.push_child(Node::known("Label::new"));
    assert!(!registry::is_scrollbar_wrapper(&wrapper));
    assert!(registry::unwrap_scrollbar(&wrapper).is_none());
}

/// The handle is a field of the view, and it is not an entity.
///
/// Its `use` line does not come through `registry::imports` — that one answers
/// what the *tree* needs — but through the state field the view declares, which
/// is where `Entity<InputState>` comes from too.
#[test]
fn the_scrollbar_asks_the_view_for_a_handle() {
    let node = Node::known("v_flex");
    let wrapper = maxx::registry::scrollbar_assembly(node, ["box", "bar"], "scroll");
    let imports = maxx::registry::imports(&wrapper);
    assert!(imports.contains(&"use gpui_component::scroll::Scrollbar;"), "{imports:?}");
    // The axis type comes with the axis, not with the bar.
    assert!(imports.contains(&"use gpui_component::scroll::ScrollbarAxis;"), "{imports:?}");
    let bare = Node::known("Scrollbar::new");
    assert!(
        !maxx::registry::imports(&bare).contains(&"use gpui_component::scroll::ScrollbarAxis;"),
        "a bar with no axis must not carry the type"
    );

    let state = maxx::registry::by_id("scrollbar").expect("the entry exists").state.expect("state");
    assert_eq!(state.ty, "ScrollHandle", "a handle is a value, not an entity");
    assert_eq!(state.initializer, "ScrollHandle::new()");
    assert!(state.imports.contains(&"use gpui::ScrollHandle;"), "{:?}", state.imports);
}

/// What the palette offers is what can stand on its own.
#[test]
fn the_palette_does_not_offer_what_only_a_property_writes() {
    let scrollbar = maxx::registry::by_id("scrollbar").expect("the entry exists for the reader");
    assert!(!scrollbar.palette, "a bar dropped alone has no handle and draws nothing");
    // Every other entry is droppable: an entry nobody can reach and no property
    // writes would be dead weight.
    for spec in maxx::registry::CATALOGUE {
        assert!(spec.palette || spec.id == "scrollbar", "{} is offered by nothing", spec.id);
    }
}

/// A tooltip carrying a quote survives the trip to the file and back.
#[test]
fn a_tooltip_with_a_quote_reads_back_whole() {
    use maxx::registry::{self, Kind, Target};

    let prop = registry::Prop { label: "prop.tooltip", target: Target::Tooltip, kind: Kind::Text };
    let mut node = Node::known("v_flex");
    registry::write(&mut node, &prop, "He said \"hi\"");

    let written = render(&node, 0);
    assert!(written.contains("Tooltip::new(\"He said \\\"hi\\\"\")"), "{written}");
    // Split on the first quote, the text would come back cut in half — and the
    // inspector would then write the half back.
    assert_eq!(registry::read(&node, &prop).as_deref(), Some("He said \"hi\""));
    assert_eq!(reparse(&written), written);
}

/// A closure the developer wrote is shown as it is, not half-read.
#[test]
fn a_hand_written_tooltip_closure_is_left_alone() {
    use maxx::registry::{self, Kind, Target};

    let prop = registry::Prop { label: "prop.tooltip", target: Target::Tooltip, kind: Kind::Text };
    let mut node = Node::known("v_flex");
    node.set_call(
        "tooltip",
        Arg::Verbatim(
            "|window, cx| Tooltip::new(\"Hint\").key_binding(None).build(window, cx)".into(),
        ),
    );

    let shown = registry::read(&node, &prop).expect("something is shown");
    assert!(shown.starts_with("|window, cx|"), "the source itself: {shown}");
}

/// A comment written in the managed region comes back out of it.
///
/// It used to disappear at the next save, silently: `syn` throws comments away
/// and `codegen` rewrites the region from the model, so what the model did not
/// hold was erased — on the developer's own words, which is the one thing an
/// editor may never do.
#[test]
fn a_comment_survives_the_region() {
    let source =
        "v_flex()\n    // le titre de la page\n    .gap_2()\n    .child(Label::new(\"Nom\"))";
    assert_eq!(reparse(source), source);
}

#[test]
fn a_comment_survives_wherever_it_stands() {
    // Above the chain, above a call, above a child, and after everything.
    let source = "// tout en haut\nv_flex()\n    .gap_2()\n    // le nom\n    .child(Label::new(\"Nom\"))\n    // fin";
    assert_eq!(reparse(source), source);
}

#[test]
fn a_comment_inside_a_child_stays_inside_it() {
    let source = "v_flex()\n    // un\n    // deux\n    .gap_2()\n    .child(\n        v_flex()\n            // dedans\n            .gap_1()\n            .child(Label::new(\"a\")),\n    )";
    assert_eq!(reparse(source), source);
}

/// A comment on the last line of a child does not collect a comma.
///
/// Found by `tests/property.rs`, and the nastiest shape of the class this file
/// exists for: the comma that closes a broken-out `.child(` argument was
/// written straight after the chain, so a chain ending on a `//` line took it
/// *inside* the comment. The next read gave the comment its text plus a comma,
/// the next save added another, and the developer's own words grew by one
/// character per save without anything ever failing.
#[test]
fn a_trailing_comment_in_a_child_does_not_collect_a_comma() {
    let source = "v_flex()\n    .child(\n        v_flex()\n            .gap_1()\n            // pourquoi cette marge\n    )";
    let once = reparse(source);
    assert_eq!(once, source);
    // The second pass is the one that used to differ.
    assert_eq!(reparse(&once), once);
    assert!(!once.contains(",\n"), "no comma may follow a comment line: {once}");
}

/// A block comment keeps the shape it was written in.
#[test]
fn a_block_comment_keeps_its_alignment() {
    let source = "v_flex()\n    /* un bloc\n       sur deux lignes */\n    .gap_2()";
    assert_eq!(reparse(source), source);
}

/// A chain that would fit on one line is broken when it carries a comment.
#[test]
fn a_commented_chain_is_never_written_inline() {
    let source = "v_flex()\n    // court\n    .gap_2()";
    let written = reparse(source);
    assert!(written.contains('\n'), "a comment has nowhere to go inline: {written}");
    assert_eq!(written, source);
}

/// What looks like a comment inside a string is not one.
#[test]
fn a_slash_slash_in_a_string_is_not_a_comment() {
    let source = "v_flex().child(Label::new(\"https://exemple.org\"))";
    assert_eq!(reparse(source), source);
}

/// A comment written inside an expression maxx keeps verbatim is not written
/// twice.
#[test]
fn a_comment_inside_an_opaque_expression_is_left_where_it_is() {
    let source = "v_flex().child(match self.state {\n    // le cas vide\n    0 => Label::new(\"a\"),\n    _ => Label::new(\"b\"),\n})";
    let written = reparse(source);
    assert_eq!(written.matches("le cas vide").count(), 1, "written twice: {written}");
    // The layout of an opaque child is maxx's — it is re-laid-out like any
    // other node — but its text is the developer's, comment included, and a
    // second pass changes nothing more.
    assert!(written.contains("// le cas vide"), "{written}");
    assert_eq!(reparse(&written), written, "the second save must be a no-op");
}

/// A comment inside a closure argument stays in the closure.
#[test]
fn a_comment_inside_an_argument_is_not_lifted_out() {
    let source = "v_flex()\n    .on_click(cx.listener(|_this, _, _, _cx| {\n        // rien pour l'instant\n    }))";
    let written = reparse(source);
    assert_eq!(written.matches("rien pour l'instant").count(), 1, "{written}");
    assert_eq!(written, source);
}

/// A block comment nests, and cutting it in half breaks the file.
#[test]
fn a_nested_block_comment_comes_back_whole() {
    let source = "v_flex()\n    /* un /* deux */ trois */\n    .gap_2()";
    // Stopping at the first `*/` would write back an open comment, and
    // everything after it in the developer's file becomes comment text.
    assert_eq!(reparse(source), source);
}

/// A comment between the parentheses belongs to nobody, so it used to vanish.
#[test]
fn a_comment_inside_an_argument_list_is_kept() {
    for source in [
        "v_flex().gap(/* huit */ 8)",
        "v_flex().child(Label::new(/* le nom */ \"Nom\"))",
        "v_flex().child(Label::new(\"a\") /* apres */)",
    ] {
        let written = reparse(source);
        let word = source
            .split("/*")
            .nth(1)
            .and_then(|rest| rest.split("*/").next())
            .expect("a comment")
            .trim()
            .to_string();
        assert!(written.contains(&word), "lost from {source}: {written}");
        // The line moves — maxx lays the chain out its own way — but a second
        // save changes nothing more.
        assert_eq!(reparse(&written), written, "{written}");
    }
}

/// A comment above a call maxx then removes is not removed with it.
#[test]
fn a_comment_outlives_the_call_it_stood_above() {
    use maxx::registry::{self, Kind, Prop, Target};

    let source = "v_flex()\n    // pourquoi ce hold\n    .h_full()\n    .gap_2()";
    let (mut node, _) = parser::parse(&file_with(source)).expect("parse");

    // The inspector turning a flag off takes the call away; the sentence above
    // it is the developer's.
    let flag = Prop { label: "prop.hold", target: Target::Flag("h_full"), kind: Kind::Bool };
    registry::write(&mut node, &flag, "false");

    let written = render(&node, 0);
    assert!(!written.contains(".h_full()"), "{written}");
    assert!(written.contains("// pourquoi ce hold"), "the comment went with the call: {written}");
    assert_eq!(reparse(&written), written);
}

/// A duplicated scrolling box does not share the original's handle.
#[test]
fn a_copied_assembly_gets_a_handle_of_its_own() {
    use maxx::registry;

    let mut box_node = Node::known("v_flex");
    box_node.set_flag("overflow_y_scroll", true);
    let wrapper = registry::scrollbar_assembly(box_node, ["box", "scroll"], "field");

    let mut root = Node::known("v_flex");
    root.push_child(wrapper.clone());
    let mut copy = wrapper;
    registry::rebind_state_fields(&mut copy, &root);
    root.push_child(copy);

    let written = render(&root, 0);
    // The box holds its handle in a call and the bar in a constructor
    // argument: renaming only one leaves a copy that scrolls in step with the
    // original — it compiles, and it is wrong only once it runs.
    assert_eq!(written.matches("&self.field)").count(), 2, "{written}");
    assert_eq!(written.matches("&self.field_2)").count(), 2, "{written}");
    // And two siblings must not answer to the same element id.
    assert_eq!(written.matches(".id(\"scroll\")").count(), 1, "{written}");
}

/// The bar is only ever put over a box that scrolls.
#[test]
fn a_bar_makes_its_box_scroll() {
    let wrapper =
        maxx::registry::scrollbar_assembly(Node::known("v_flex"), ["box", "bar"], "scroll");
    let box_node = &wrapper.children[0];
    assert!(box_node.call("overflow_y_scroll").is_some(), "a bar over a still box watches nothing");
    assert!(box_node.call("h_full").is_some(), "and a box that grows scrolls nothing");

    // A row scrolls sideways, and takes the width instead.
    let row = maxx::registry::scrollbar_assembly(Node::known("h_flex"), ["box", "bar"], "scroll");
    assert!(row.children[0].call("overflow_x_scroll").is_some());
    assert!(row.children[0].call("w_full").is_some());
}

/// A box that did not scroll gets everything it needs, id included.
#[test]
fn a_bar_makes_its_box_stateful() {
    let wrapper =
        maxx::registry::scrollbar_assembly(Node::known("v_flex"), ["scroll", "bar"], "handle");
    let written = render(&wrapper, 0);
    let box_node = &wrapper.children[0];

    // `overflow_y_scroll` and `track_scroll` both live on a stateful element,
    // which a `div` only becomes once it carries an id: written without one,
    // the generated project does not compile.
    assert!(box_node.call("id").is_some(), "{written}");
    let id = written.find(".id(").expect("the id must be written");
    let scroll = written.find(".overflow_y_scroll()").expect("the box must scroll");
    let track = written.find(".track_scroll(").expect("the box must be tracked");
    assert!(id < scroll && id < track, "the id comes first: {written}");
    assert_eq!(reparse(&written), written);
}

/// Two assemblies bound to the same field do not trade handles.
#[test]
fn two_pasted_assemblies_keep_their_own_handles() {
    use maxx::registry;

    let make = || {
        let mut box_node = Node::known("v_flex");
        box_node.set_flag("overflow_y_scroll", true);
        registry::scrollbar_assembly(box_node, ["scroll", "bar"], "field")
    };

    let mut root = Node::known("v_flex");
    root.push_child(make());
    let mut copy = make();
    registry::rebind_state_fields(&mut copy, &root);
    root.push_child(copy);

    let written = render(&root, 0);
    // Each wrapper holds one handle, named twice: the box that scrolls and the
    // bar that watches it. A sweep over every `&self.…` of the subtree would
    // have made the first box track the second's handle.
    for field in ["&self.field)", "&self.field_2)"] {
        assert_eq!(written.matches(field).count(), 2, "{field}: {written}");
    }
}

/// A comment above a call maxx removes lands on the child below it, not in a
/// slot that writes nothing.
#[test]
fn a_comment_above_a_removed_call_reaches_the_child_under_it() {
    use maxx::registry::{self, Kind, Prop, Target};

    let source = "v_flex()\n    // garde-moi\n    .gap_4()\n    .child(Label::new(\"x\"))";
    let (mut node, _) = parser::parse(&file_with(source)).expect("parse");
    let flag = Prop { label: "prop.gap", target: Target::Flag("gap_4"), kind: Kind::Bool };
    registry::write(&mut node, &flag, "false");

    let written = render(&node, 0);
    assert!(!written.contains(".gap_4()"), "{written}");
    assert!(written.contains("// garde-moi"), "the comment fell into a slot: {written}");
    assert_eq!(reparse(&written), written);
}

/// The wrapper's own comments come back with the box when the bar is removed.
#[test]
fn unwrapping_keeps_what_was_written_above_the_wrapper() {
    use maxx::registry;

    let mut box_node = Node::known("v_flex");
    box_node.set_flag("overflow_y_scroll", true);
    let mut wrapper = registry::scrollbar_assembly(box_node, ["scroll", "bar"], "field");
    wrapper.comments = vec!["// la liste des vues".into()];

    let back = registry::unwrap_scrollbar(&wrapper).expect("maxx wrote this wrapper");
    assert_eq!(back.comments, vec!["// la liste des vues".to_string()]);
}

#[test]
fn an_empty_handler_is_filled_with_the_box_that_was_asked_for() {
    for (kind, imports, body) in maxx::scaffold::templates::BOXES {
        let filled = maxx::view::fill_handler(HANDLER, "pressed", kind)
            .unwrap_or_else(|error| panic!("{kind} must fill: {error}"));

        assert!(filled.contains(body.trim_start()), "{kind}: {filled}");
        for import in *imports {
            assert!(filled.contains(import), "{kind} needs {import}: {filled}");
        }
        // The two parameters the body uses lose their underscore; the event the
        // body does not touch keeps its own.
        assert!(filled.contains("window: &mut Window"), "{kind}: {filled}");
        assert!(filled.contains("cx: &mut Context<Self>"), "{kind}: {filled}");
        assert!(filled.contains("_event: &ClickEvent"), "{kind}: {filled}");
        // And what maxx manages is untouched: a handler is beside the region,
        // not inside it.
        assert!(filled.contains("// maxx:begin"), "{kind}: {filled}");
    }
}

#[test]
fn a_handler_that_holds_something_is_never_written_over() {
    let written = HANDLER.replace("    ) {\n    }", "    ) {\n        self.count += 1;\n    }");
    let error = maxx::view::fill_handler(&written, "pressed", "dialog")
        .expect_err("a body that holds something must be refused");
    assert!(error.contains("pressed"), "{error}");
    // And the file is untouched, which is the point of the refusal.
    assert!(written.contains("self.count += 1;"));
}

#[test]
fn a_handler_that_is_not_there_is_named_rather_than_invented() {
    let error = maxx::view::fill_handler(HANDLER, "absent", "dialog")
        .expect_err("a method that is not written must be refused");
    assert!(error.contains("absent"), "{error}");

    let error = maxx::view::fill_handler(HANDLER, "pressed", "carousel")
        .expect_err("a box maxx does not know must be refused");
    assert!(error.contains("carousel"), "{error}");
}

/// A view carrying one empty handler, as `ensure_handler` writes it.
const HANDLER: &str = r#"use gpui::{ClickEvent, Context, Window, prelude::*};
use gpui_component::v_flex;

pub struct Home {}

impl Home {
    /// Written by maxx; the body is yours.
    pub fn pressed(
        &mut self,
        _event: &ClickEvent,
        _window: &mut Window,
        _cx: &mut Context<Self>,
    ) {
    }
}

impl Render for Home {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        // maxx:begin
        v_flex()
        // maxx:end
    }
}
"#;

#[test]
fn every_subtree_template_reads_back_as_a_tree() {
    for (id, _, source) in maxx::scaffold::templates::SUBTREES {
        let node = maxx::parser::parse_expr(source)
            .unwrap_or_else(|error| panic!("{id} must parse: {error}"));
        // An opaque node is one maxx cannot take apart, which is exactly what a
        // template must never be: it is dropped in to be edited.
        assert!(!node.is_opaque(), "{id} must not be opaque");
        assert!(!node.children.is_empty(), "{id} must hold something");

        // And it comes back out the way it went in: a template is a piece of
        // Rust in a table, so the table is the round trip's own fixture.
        let written = maxx::codegen::render(&node, 0);
        assert_eq!(written.trim(), source.trim(), "{id} must survive the round trip");
    }
}

#[test]
fn every_subtree_template_wears_a_label() {
    for (id, _, _) in maxx::scaffold::templates::SUBTREES {
        assert!(
            maxx::registry::SUBTREE_LABELS.iter().any(|(this, _)| this == id),
            "{id} has no label — the palette would show a blank row"
        );
    }
    for (id, _) in maxx::registry::SUBTREE_LABELS {
        assert!(
            maxx::scaffold::templates::SUBTREES.iter().any(|(this, _, _)| this == id),
            "{id} has a label but no expression"
        );
    }
}

/// A braced import whose names are only partly there adds the rest, not itself.
///
/// The shape a real project arrived in: `use gpui_component::button::Button;`
/// already written by an earlier save, and maxx then owing
/// `use gpui_component::button::{Button, ButtonVariants};` for a `.primary()`.
/// Every name of a braced needle has to be found before it counts as imported,
/// so this one was not — and maxx wrote the whole statement, giving the file two
/// `Button` imports. `E0252`, in the developer's project, on a line maxx wrote.
#[test]
fn a_partly_imported_brace_adds_only_what_is_missing() {
    let source = "\
use gpui::prelude::*;
use gpui_component::button::Button;

fn main() {}
";
    let out = maxx::view::ensure_imports_for_test(
        source.to_string(),
        &["use gpui_component::button::{Button, ButtonVariants};"],
    );
    assert_eq!(
        out.matches("use gpui_component::button::").count(),
        2,
        "the plain import stays and one line is added for the trait:\n{out}"
    );
    assert!(out.contains("use gpui_component::button::ButtonVariants;"), "{out}");
    assert!(
        !out.contains("{Button, ButtonVariants}"),
        "the whole statement must not be written again:\n{out}"
    );
}

/// An import maxx adds joins the header, not the last `use` of the file.
///
/// A `use` is a top-level item like any other, and nothing forbids one below an
/// `impl` — a developer moving a type around leaves them there all the time.
/// Anchored on the last of them, every import maxx writes went down with it, to
/// the bottom of the file, away from the block where imports are read. It still
/// compiles, which is why nothing said anything.
#[test]
fn an_added_import_joins_the_header_and_not_the_last_use_of_the_file() {
    let source = "\
//! What this view is.

use gpui::prelude::*;

pub struct Home;

impl Home {
    pub fn new() -> Self {
        Self
    }
}

use std::fmt;
";
    let out = maxx::view::ensure_imports_for_test(
        source.to_string(),
        &["use gpui_component::button::Button;"],
    );
    let lines: Vec<&str> = out.lines().collect();
    assert_eq!(lines[2], "use gpui::prelude::*;");
    assert_eq!(lines[3], "use gpui_component::button::Button;", "in the header:\n{out}");
    assert_eq!(*lines.last().expect("a last line"), "use std::fmt;", "which is left alone");
}

/// `mod inner;` above the imports does not end the header.
///
/// The rule is "the first item with a body", and not "the first item that is
/// not a `use`": a declaration is written above the imports as often as below,
/// and a header cut short by one would send the import above the `//!`.
#[test]
fn a_module_declaration_does_not_end_the_header() {
    let source = "\
//! What this view is.

mod inner;

use gpui::prelude::*;

pub struct Home;
";
    let out = maxx::view::ensure_imports_for_test(
        source.to_string(),
        &["use gpui_component::button::Button;"],
    );
    let lines: Vec<&str> = out.lines().collect();
    assert_eq!(lines[4], "use gpui::prelude::*;");
    assert_eq!(lines[5], "use gpui_component::button::Button;", "{out}");
}

/// With no import to join, the line goes under the `//!` and not above it.
///
/// The anchor was the start of the file, so a view carrying no import at all
/// took maxx's first one before its own inner doc comment — where a `use` is not
/// merely odd but refused: `//!` and `#![…]` have to come first, and the file
/// stopped compiling on the line maxx had just written.
#[test]
fn the_first_import_of_a_file_goes_under_its_inner_doc_comment() {
    let source = "\
//! What this view is.

pub struct Home;
";
    let out = maxx::view::ensure_imports_for_test(
        source.to_string(),
        &["use gpui_component::button::Button;"],
    );
    assert!(
        out.starts_with("//! What this view is.\n"),
        "the inner doc comment stays first:\n{out}"
    );
    assert!(out.contains("use gpui_component::button::Button;"), "{out}");
    syn::parse_file(&out).expect("and what comes out still parses");
}

/// A file `syn` will not parse reads its header the same way.
///
/// The fallback is what maxx had before it could ask `syn`, and it carried the
/// same defect: the last `\nuse ` of the text, wherever it sat. A view is
/// unparseable exactly while it is being written, which is when maxx is most
/// likely to be saving it.
#[test]
fn the_text_fallback_stops_at_the_header_too() {
    let source = "\
//! Broken.

use gpui::prelude::*;

impl Home { fn new( }

use std::fmt;
";
    assert!(
        syn::parse_file(source).is_err(),
        "the case only exists for a file that does not parse"
    );
    let out = maxx::view::ensure_imports_for_test(
        source.to_string(),
        &["use gpui_component::button::Button;"],
    );
    let lines: Vec<&str> = out.lines().collect();
    assert_eq!(lines[2], "use gpui::prelude::*;");
    assert_eq!(lines[3], "use gpui_component::button::Button;", "{out}");
}

/// The text fallback does not walk into a module body.
///
/// `mod tests {` is not `mod tests;`. Read as a declaration, the scan goes on
/// into the body and anchors on the `use super::*;` inside it: the import is
/// then written **in the module**, where the code maxx generated at the top
/// level cannot see it — and `already_imported` finds it there textually, so it
/// is never written again in the right place.
#[test]
fn the_text_fallback_does_not_walk_into_a_module_body() {
    let source = "\
//! Broken.

use gpui::prelude::*;

mod tests {
    use super::*;
    fn t() { broken(
}
";
    let out = added_to(source);
    let lines: Vec<&str> = out.lines().collect();
    assert_eq!(lines[3], "use gpui_component::button::Button;", "at the top level:\n{out}");
    assert_eq!(lines[5], "mod tests {", "and the module is untouched:\n{out}");
}

/// An import is never written between an attribute and the item it decorates.
///
/// A `///` line and a `#[derive(…)]` line are not remarks about the file, they
/// belong to what follows. With no import to join, anchoring under them does
/// not merely read oddly: the attribute then applies to the `use`, and the file
/// stops compiling. This is exactly the "view with no import yet" shape.
#[test]
fn an_import_is_never_written_between_an_attribute_and_its_item() {
    // The last two carry a line between the attribute and its item: a blank, a
    // note, a block comment. Anything that closes the run there puts the import
    // back between the two.
    for source in [
        "//! Doc.\n\n#[derive(Clone)]\npub struct Home;\n\nfn broken( {\n",
        "//! Doc.\n\n/// What this holds.\npub struct Home;\n\nfn broken( {\n",
        "//! Doc.\n\n#[derive(Clone)]\n// a note\npub struct Home;\n\nfn broken( {\n",
        "//! Doc.\n\n#[derive(Clone)]\n/* a note */\npub struct Home;\n\nfn broken( {\n",
    ] {
        let out = added_to(source);
        let lines: Vec<&str> = out.lines().collect();
        assert_eq!(lines[2], "use gpui_component::button::Button;", "above it:\n{out}");
        assert_eq!(
            *lines.last().filter(|line| !line.is_empty()).unwrap_or(&""),
            "fn broken( {",
            "and nothing has moved below:\n{out}"
        );
        assert!(
            out.contains("#[derive(Clone)]\n") || out.contains("/// What this holds.\n"),
            "what decorates the item is still above it:\n{out}"
        );
    }
}

/// An import left open does not swallow the rest of the file.
///
/// A half-typed `use gpui::{` is the single most likely reason a view does not
/// parse, since it does not parse for exactly as long as it is being written.
/// Counted as a header line all the way to the end, it sent the new import to
/// the bottom of the file — the very defect this pass exists to remove, and a
/// step back from the text scan that came before it. Only a construct that
/// closes advances the header.
#[test]
fn an_unterminated_import_does_not_swallow_the_file() {
    let source = "\
//! Broken.

use gpui::{Context,

pub struct Home;

fn broken( {
";
    let out = added_to(source);
    let lines: Vec<&str> = out.lines().collect();
    assert_eq!(lines[2], "use gpui_component::button::Button;", "in the header:\n{out}");
    assert_ne!(
        *lines.last().expect("a last line"),
        "use gpui_component::button::Button;",
        "and not at the bottom of the file:\n{out}"
    );
}

/// A block comment is delimited, not recognised line by line.
///
/// Matching a leading `*` ends the header on the first unstarred line of a
/// licence header — and the import is then written *inside* the comment, where
/// it does nothing at all and where `already_imported` will nonetheless find it.
#[test]
fn a_block_comment_is_not_read_line_by_line() {
    let source = "\
/*
Copyright someone.
*/
use gpui::prelude::*;
fn broken( {
";
    let out = added_to(source);
    let lines: Vec<&str> = out.lines().collect();
    assert_eq!(lines[1], "Copyright someone.", "the comment is left whole:\n{out}");
    assert_eq!(lines[4], "use gpui_component::button::Button;", "{out}");
}

/// A brace inside a trailing comment does not open an import.
///
/// `use a::b; // {` counted as an import left open, so every line after it was
/// read as its continuation and a stray `}` further down closed the count in
/// the middle of the file. The counter reads the code, not the comment.
#[test]
fn a_brace_in_a_trailing_comment_does_not_open_an_import() {
    let source = "\
//! Doc.

use a::b; // {

const X: u8 = 1;
}
fn broken( {
";
    let out = added_to(source);
    let lines: Vec<&str> = out.lines().collect();
    assert_eq!(lines[2], "use a::b; // {");
    assert_eq!(lines[3], "use gpui_component::button::Button;", "right after it:\n{out}");
}

/// A declaration is known by its visibility and its semicolon.
///
/// `pub(crate) mod inner;` belongs to the header exactly as much as
/// `mod inner;`, and reading only the second split the import block in two. And
/// a declaration ends on a semicolon, not merely on the absence of a brace:
/// `mod inner` with the brace on the next line passes that weaker test and lets
/// the scan walk into the body, which is where the import would then be written.
#[test]
fn a_declaration_is_known_by_its_visibility_and_its_semicolon() {
    let out =
        added_to("//! Doc.\n\npub(crate) mod inner;\n\nuse gpui::prelude::*;\n\nfn broken( {\n");
    let lines: Vec<&str> = out.lines().collect();
    assert_eq!(lines[4], "use gpui::prelude::*;");
    assert_eq!(lines[5], "use gpui_component::button::Button;", "after the import:\n{out}");

    let out = added_to(
        "//! Doc.\n\nuse gpui::prelude::*;\n\nmod inner\n{\n    use super::*;\n    fn t( {\n}\n",
    );
    let lines: Vec<&str> = out.lines().collect();
    assert_eq!(lines[3], "use gpui_component::button::Button;", "at the top level:\n{out}");
    assert_eq!(lines[5], "mod inner", "and the module is untouched:\n{out}");
}

/// A re-export is an import too.
///
/// `syn` calls `pub use crate::a::B;` an `Item::Use` like any other; a scan
/// looking for a line opening on `use ` does not, so a header opening on
/// re-exports took the new line above them instead of after the last import.
#[test]
fn the_text_fallback_sees_a_re_export() {
    let source = "\
//! Broken.

pub use crate::a::B;
use gpui::prelude::*;

fn broken( {
";
    let out = added_to(source);
    let lines: Vec<&str> = out.lines().collect();
    assert_eq!(lines[2], "pub use crate::a::B;");
    assert_eq!(lines[4], "use gpui_component::button::Button;", "after the last one:\n{out}");
}

/// A file whose last line has no terminator still gets its import on a line.
///
/// The anchor is then the very end of the source, and the import was written
/// onto that last line — swallowed by the `//!` for the one shape that reaches
/// it, and found there afterwards by `already_imported`, so never written
/// anywhere else.
#[test]
fn a_file_with_no_final_newline_still_gets_its_import_on_a_line() {
    let out = added_to("//! What this view is.");
    assert_eq!(out.lines().next(), Some("//! What this view is."), "{out}");
    assert!(out.contains("\nuse gpui_component::button::Button;"), "on its own line:\n{out}");
    syn::parse_file(&out).expect("and what comes out still parses");
}

/// One import asked for, for every shape of header the tests above pin down.
fn added_to(source: &str) -> String {
    maxx::view::ensure_imports_for_test(
        source.to_string(),
        &["use gpui_component::button::Button;"],
    )
}

/// An import written twice is pointed at, in the file, on the line.
///
/// maxx adds; it does not take away what it did not write. One of the two lines
/// may be the developer's, and removing theirs is a border this program does not
/// cross — so it says what is wrong and leaves the choice where it belongs.
#[test]
fn a_duplicate_import_is_flagged_where_it_is() {
    let source = "\
use gpui::prelude::*;
use gpui_component::button::Button;
use gpui_component::button::{Button, ButtonVariants};

fn main() {}
";
    let flagged = maxx::view::flag_duplicate_imports_for_test(source.to_string());
    let lines: Vec<&str> = flagged.lines().collect();
    assert_eq!(lines[2], "// maxx: Button is imported twice — one of these two lines has to go.");
    assert_eq!(lines[3], "use gpui_component::button::{Button, ButtonVariants};");
    assert!(flagged.contains("use gpui_component::button::Button;"), "nothing is removed");
}

/// Saying it a hundred times is saying it once.
#[test]
fn the_mark_is_written_once_and_taken_back_when_it_is_fixed() {
    let broken = "\
use gpui_component::button::Button;
use gpui_component::button::{Button, ButtonVariants};
";
    let once = maxx::view::flag_duplicate_imports_for_test(broken.to_string());
    let twice = maxx::view::flag_duplicate_imports_for_test(once.clone());
    assert_eq!(once, twice, "a second save must not stack a second mark");

    // And once the developer has taken a line away, maxx takes its own back.
    let fixed = once.replace("use gpui_component::button::Button;\n", "");
    let cleared = maxx::view::flag_duplicate_imports_for_test(fixed);
    assert!(!cleared.contains("// maxx:"), "the mark goes when the reason goes:\n{cleared}");
}

/// A spelling maxx does not read is a spelling maxx says nothing about.
#[test]
fn an_import_maxx_cannot_read_is_left_in_peace() {
    for source in [
        "use gpui_component::button::*;\nuse gpui_component::button::Button;\n",
        "use gpui_component::button::Button as B;\nuse gpui_component::button::Button;\n",
        "use gpui_component::{button::Button, label::Label};\nuse gpui_component::button::Button;\n",
    ] {
        let flagged = maxx::view::flag_duplicate_imports_for_test(source.to_string());
        assert_eq!(flagged, source, "saying nothing beats saying something wrong:\n{flagged}");
    }
}

/// A CRLF file stays a CRLF file.
///
/// `lines()` drops the `\r` and joining on `\n` puts back only half of it: a
/// whole-file diff for a change that did not happen. The rule was already
/// written down for the copied modules; this pass runs on every save and had to
/// carry it too.
#[test]
fn flagging_imports_keeps_the_files_line_endings() {
    let source = "use gpui::prelude::*;\r\nuse a::b::C;\r\nuse a::b::{C, D};\r\n";
    let flagged = maxx::view::flag_duplicate_imports_for_test(source.to_string());
    assert!(flagged.contains("// maxx: C is imported twice"), "{flagged:?}");
    assert!(!flagged.contains("\n\n"), "no bare newline may appear: {flagged:?}");
    assert_eq!(flagged.matches("\r\n").count(), flagged.lines().count(), "{flagged:?}");
}

/// A `use` inside a module shadows; it does not clash.
#[test]
fn an_import_in_an_inner_module_is_not_a_duplicate() {
    let source = "\
use a::b::C;

#[cfg(test)]
mod tests {
    use a::b::C;
}
";
    assert_eq!(
        maxx::view::flag_duplicate_imports_for_test(source.to_string()),
        source,
        "a shadowing import is perfectly good Rust"
    );
}

/// And a line that merely looks like maxx's mark is not maxx's to remove.
#[test]
fn a_note_the_developer_wrote_is_left_alone() {
    let source = "\
use a::b::C;

fn main() {
    // maxx: their own note, indented, and none of maxx's business
    let _ = 1;
}
";
    assert_eq!(maxx::view::flag_duplicate_imports_for_test(source.to_string()), source);
}

/// A note the developer wrote past the imports is not maxx's to remove.
///
/// The mark is recognised while the import block lasts, and no further: a
/// `// maxx: …` at column zero above a function is theirs, and maxx adds — it
/// does not take away what it did not write.
#[test]
fn a_note_below_the_imports_survives() {
    let source = "\
use a::b::C;

// maxx: keep this in sync with home.rs
fn helper() {}
";
    assert_eq!(maxx::view::flag_duplicate_imports_for_test(source.to_string()), source);
}

/// The save-time import pass leaves alone everything it has nothing to say
/// about.
///
/// Three shapes it damaged, found by probing rather than by thinking, an hour
/// before this was to be published:
///
/// - a file whose line endings are mixed came back all CRLF;
/// - a `use …;` at column zero **inside a raw string** was read as an import,
///   and maxx wrote its comment into the string literal — changing what the
///   developer's code means, silently;
/// - a `// maxx: …` note the developer wrote above an import was deleted.
#[test]
fn the_import_pass_touches_nothing_it_has_no_business_with() {
    let untouched = [
        // Mixed endings: a rejoin on one ending rewrites every other line.
        "use a::b::C;\r\nuse d::e::F;\nfn main() {}\n",
        // No trailing newline: adding one is a diff of its own.
        "use a::b::C;\nfn main() {}",
        // A raw string is not an import block. `syn` sees items; a string is
        // not one.
        "use a::b::C;\n\nconst T: &str = r#\"\nuse a::b::C;\n\"#;\n",
        // Their note, not maxx's sentence.
        "// maxx: keep this in sync with home.rs\nuse a::b::C;\n",
        // And a `use` inside a module shadows rather than clashes.
        "use a::b::C;\n\nmod tests {\n    use a::b::C;\n}\n",
    ];
    for source in untouched {
        assert_eq!(
            maxx::view::flag_duplicate_imports_for_test(source.to_string()),
            source,
            "left alone:\n{source:?}"
        );
    }
}

/// And a CRLF file that does get a mark keeps its endings.
#[test]
fn a_mark_written_into_a_crlf_file_is_written_in_crlf() {
    let source = "use a::b::C;\r\nuse a::b::{C, D};\r\n";
    let flagged = maxx::view::flag_duplicate_imports_for_test(source.to_string());
    assert!(flagged.contains("// maxx: C is imported twice"), "{flagged:?}");
    assert!(!flagged.replace("\r\n", "").contains('\n'), "no bare newline: {flagged:?}");
}

/// The four ways the pass could still touch what is not its business.
///
/// Found by probing the second half of it, an hour after the first half was
/// fixed the same way: moving *insertion* onto `syn` left *removal* scanning
/// raw text, so maxx went on deleting its own sentence from inside a string
/// constant.
#[test]
fn the_import_pass_leaves_alone_what_is_not_an_import_block() {
    let untouched = [
        // Maxx's sentence, inside a raw string. Not above an import, so not
        // maxx's to take away.
        "use a::b::C;\n\nconst T: &str = r#\"\n// maxx: C is imported twice — one of these two lines has to go.\nhello\n\"#;\n",
        // A file that does not parse keeps its marks: taking away a warning
        // that cannot be written back is churn at the worst moment.
        "// maxx: C is imported twice — one of these two lines has to go.\nuse a::b::{C, D};\nuse a::b::C;\nfn main( {}\n",
    ];
    for source in untouched {
        assert_eq!(
            maxx::view::flag_duplicate_imports_for_test(source.to_string()),
            source,
            "left alone:\n{source:?}"
        );
    }
}

/// An import carrying an attribute or a doc comment is still an import.
///
/// A span covers the attributes above the item, so reading the item's first
/// line read the attribute's: the import went unseen, and the plain duplicate
/// under it went unflagged with it.
#[test]
fn an_attributed_import_is_read_like_any_other() {
    for above in ["#[allow(unused_imports)]", "/// what this is for"] {
        let source = format!("{above}\nuse a::b::C;\nuse a::b::{{C, D}};\n");
        let flagged = maxx::view::flag_duplicate_imports_for_test(source.clone());
        assert!(flagged.contains("// maxx: C is imported twice"), "{above}:\n{flagged}");
        // And saying it twice is saying it once.
        assert_eq!(
            maxx::view::flag_duplicate_imports_for_test(flagged.clone()),
            flagged,
            "{above}"
        );
    }
}

/// The mark keeps the file's endings even where the line has none.
///
/// A regression, introduced by the fix for mixed endings itself: the ending was
/// read off the flagged line, and the last line of a file with no final newline
/// carries none — so a CRLF file got a bare `\n`. The two shapes were each
/// covered and never together.
#[test]
fn a_crlf_file_with_no_final_newline_still_gets_a_crlf_mark() {
    let source = "use a::b::C;\r\nuse a::b::{C, D};";
    let flagged = maxx::view::flag_duplicate_imports_for_test(source.to_string());
    assert!(flagged.contains("// maxx: C is imported twice"), "{flagged:?}");
    assert!(!flagged.replace("\r\n", "").contains('\n'), "no bare newline: {flagged:?}");
}

/// A mark goes above an import's attributes, and is taken back from there.
///
/// Anchored on the keyword instead, a mark that ended up above an
/// `#[allow(…)]` was unreachable: it stayed for good and a second stacked under
/// it at every save — against the one thing this pass promises.
#[test]
fn a_mark_sits_above_the_whole_item_and_comes_back_off() {
    let source = "\
use a::b::C;
#[allow(unused_imports)]
use a::b::{C, D};
";
    let once = maxx::view::flag_duplicate_imports_for_test(source.to_string());
    let lines: Vec<&str> = once.lines().collect();
    assert_eq!(lines[1], "// maxx: C is imported twice — one of these two lines has to go.");
    assert_eq!(lines[2], "#[allow(unused_imports)]", "above the attribute, not under it");

    assert_eq!(
        maxx::view::flag_duplicate_imports_for_test(once.clone()),
        once,
        "a second save must not stack a second mark"
    );
    let fixed = once.replace("use a::b::C;\n", "");
    assert!(
        !maxx::view::flag_duplicate_imports_for_test(fixed).contains("// maxx:"),
        "and it comes off when the reason goes"
    );
}

/// An indented import gets an indented mark.
#[test]
fn a_mark_follows_the_indentation_of_what_it_points_at() {
    let source = "  use a::b::C;\n  use a::b::{C, D};\n";
    let flagged = maxx::view::flag_duplicate_imports_for_test(source.to_string());
    assert!(
        flagged.lines().any(|line| line.starts_with("  // maxx: C is imported twice")),
        "{flagged:?}"
    );
}
