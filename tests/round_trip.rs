//! The round-trip contract: what the designer writes, it must be able to read
//! back, and what a human writes by hand must survive being read and rewritten.

use maxx::codegen::render;
use maxx::model::{Arg, Base, Node};
use maxx::parser;

/// Wraps an expression in the smallest file carrying a managed region.
fn file_with(expr: &str) -> String {
    let mut out = String::from(
        "use gpui::*;\n\n\
         impl Render for Accueil {\n\
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
    let source = "if self.busy { spinner() } else { Label::new(\"prêt\") }";
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
    assert_eq!(
        node.call("w").unwrap().args[0],
        Arg::Verbatim("px(120.)".into())
    );
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
        node.children.push(child);
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

/// Un commentaire écrit à la main, qui doit survivre.
impl Render for Accueil {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        // maxx:begin
        v_flex()
        // maxx:end
    }

    /// Une méthode ajoutée à la main.
    fn valider(&mut self) {}
}
";
    let spliced = parser::splice(file, "h_flex().gap_2()").expect("markers are present");

    assert!(spliced.contains("Un commentaire écrit à la main"));
    assert!(spliced.contains("Une méthode ajoutée à la main"));
    assert!(spliced.contains("        h_flex().gap_2()\n"));
    assert!(!spliced.contains("v_flex()"));

    // And the spliced file is still readable.
    let (node, _) = parser::parse(&spliced).expect("the region should parse");
    assert_eq!(node.base.path(), Some("h_flex"));
}

#[test]
fn a_file_without_markers_is_refused_not_rewritten() {
    let file = "fn main() {}\n";
    assert!(matches!(
        parser::parse(file),
        Err(parser::Error::NoMarkers)
    ));
    assert!(matches!(
        parser::splice(file, "v_flex()"),
        Err(parser::Error::NoMarkers)
    ));
}

#[test]
fn a_multiline_opaque_expression_does_not_drift() {
    // The bug this guards: the opaque slice kept its file indentation, and
    // `splice` added the region indent again on every save.
    let expr = "if self.busy {\n    spinner()\n} else {\n    Label::new(\"prêt\")\n}";
    let mut file = file_with(expr);

    for _ in 0..3 {
        let (node, region) = parser::parse(&file).expect("the region should parse");
        assert!(node.is_opaque());
        let block = maxx::codegen::render_for_splice(&node, region.indent);
        file = parser::splice(&file, &block).expect("markers are present");
    }

    let saved = file
        .lines()
        .filter(|line| line.contains("spinner()"))
        .next()
        .expect("the expression is still there");
    assert_eq!(
        saved, "            spinner()",
        "l'indentation ne doit pas croître à chaque enregistrement"
    );
}

#[test]
fn moving_a_node_into_a_later_sibling_works() {
    let mut root = Node::known("v_flex");
    root.children.push(Node::known("Label::new"));
    root.children.push(Node::known("h_flex"));

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
    let id_prop = spec
        .props
        .iter()
        .find(|prop| prop.label == "Identifiant")
        .expect("Button has an id property");

    assert!(
        !maxx::registry::editable(&node, id_prop),
        "un argument écrit à la main n'est pas éditable"
    );
    maxx::registry::write(&mut node, id_prop, "ok");
    assert_eq!(
        maxx::codegen::render(&node, 0),
        source,
        "l'expression d'origine doit être intacte"
    );
}

#[test]
fn an_invalid_field_name_is_refused() {
    let mut node = maxx::registry::instantiate("input").expect("input is in the catalogue");
    let spec = maxx::registry::of(&node).expect("Input is in the catalogue");
    let prop = &spec.props[0];

    for refused in ["", "mon champ", "2champ", "champ-x"] {
        maxx::registry::write(&mut node, prop, refused);
        assert_eq!(
            maxx::codegen::render(&node, 0),
            "Input::new(&self.champ)",
            "« {refused} » ne doit pas être écrit dans le source"
        );
    }

    maxx::registry::write(&mut node, prop, "adresse");
    assert_eq!(maxx::codegen::render(&node, 0), "Input::new(&self.adresse)");
}

#[test]
fn a_node_cannot_be_dropped_into_itself() {
    let mut root = Node::known("v_flex");
    let mut column = Node::known("v_flex");
    column.children.push(Node::known("Label::new"));
    root.children.push(column);

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
        root.children.push(child);
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
