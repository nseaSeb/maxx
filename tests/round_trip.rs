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
    assert!(matches!(parser::parse(file), Err(parser::Error::NoMarkers)));
    assert!(matches!(parser::splice(file, "v_flex()"), Err(parser::Error::NoMarkers)));
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
        let block = maxx::codegen::render_for_splice(&node, region.width());
        file = parser::splice(&file, &block).expect("markers are present");
    }

    let saved = file
        .lines()
        .find(|line| line.contains("spinner()"))
        .expect("the expression is still there");
    assert_eq!(
        saved, "            spinner()",
        "l'indentation ne doit pas croître à chaque enregistrement"
    );
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
    assert_eq!(maxx::codegen::render(&node, 0), source, "l'expression d'origine doit être intacte");
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
        maxx::registry::props(spec).into_iter().find(|prop| prop.label == "Largeur").unwrap();
    let colour = maxx::registry::props(spec).into_iter().find(|prop| prop.label == "Fond").unwrap();

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
    assert!(root.at(&[1]).is_none(), "le nœud a bien disparu");

    root = before;
    assert!(
        root.at(&[1]).is_some(),
        "et il est de retour après l'annulation, donc la sélection tient"
    );
}

#[test]
fn interleaved_children_keep_their_place() {
    // Lifting every child to the end of the chain moved a header below a list.
    let source = "v_flex().child(entete()).children(self.lignes()).child(pied())";
    assert_eq!(reparse(source), source);

    let conditional = "v_flex().child(a()).when(self.gros, |d| d.child(b())).child(c())";
    assert_eq!(reparse(conditional), conditional);
}

#[test]
fn a_brace_in_a_comment_does_not_end_a_block() {
    let source = "impl Accueil {\n    /// Ferme le panneau } et remet tout à zéro.\n    pub fn r(&mut self) {}\n}\n";
    let open = source.find('{').unwrap();
    let close = maxx::parser::matching_brace(source, open).unwrap();
    assert_eq!(&source[close..], "}\n", "le bloc se ferme à la bonne accolade");

    let with_string = "fn f() { let s = \"} pas une accolade\"; }\n";
    let open = with_string.find('{').unwrap();
    let close = maxx::parser::matching_brace(with_string, open).unwrap();
    assert_eq!(&with_string[close..], "}\n");
}

#[test]
fn a_multiline_string_is_not_reindented() {
    let file = file_with("v_flex()");
    let block = "div().child(\n    \"ligne un\nligne deux\",\n)";
    let spliced = parser::splice(&file, block).expect("markers are present");
    assert!(
        spliced.contains("\nligne deux\","),
        "les espaces ne doivent pas entrer dans la chaîne :\n{spliced}"
    );
}

#[test]
fn a_length_must_be_a_rust_literal() {
    let mut node = maxx::registry::instantiate("button").unwrap();
    let spec = maxx::registry::of(&node).unwrap();
    let width =
        maxx::registry::props(spec).into_iter().find(|prop| prop.label == "Largeur").unwrap();

    for refused in [".5", "inf", "NaN", "-inf", "12px", "1.2.3"] {
        maxx::registry::write(&mut node, width, refused);
        assert!(
            maxx::registry::validate(width, refused).is_some(),
            "« {refused} » doit être signalé"
        );
        assert!(
            !maxx::codegen::render(&node, 0).contains("px("),
            "« {refused} » ne doit pas atteindre le fichier"
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
    let label = spec.props.iter().find(|p| p.label == "Libellé").unwrap();

    assert!(
        !maxx::registry::editable(&node, label),
        "une expression écrite à la main ne s'édite pas en texte libre"
    );
}

#[test]
fn a_multiline_string_survives_a_full_save_cycle() {
    // `splice` stopped indenting inside literals, but `dedent` still stripped
    // them on the way back in, so the fix was one-sided and the string drifted
    // a little on every save.
    let mut file = String::from(
        "impl Render for Accueil {\n\
         \x20   fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {\n\
         \x20       // maxx:begin\n\
         \x20       div().child(\n\
         \"ligne un\n\
         \x20 ligne deux\",\n\
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

    assert_eq!(seen[0], seen[1], "le fichier ne doit plus bouger");
    assert_eq!(seen[1], seen[2]);
    assert!(
        seen[0].contains("\n  ligne deux\""),
        "l'indentation de la chaîne doit rester intacte :\n{}",
        seen[0]
    );
}

#[test]
fn a_lifetime_is_not_a_char_literal() {
    let source = "impl Foo {\n    fn f<'a>() { let c = 'x'; }\n    fn g() {}\n}\n";
    let open = source.find('{').unwrap();
    let close = maxx::parser::matching_brace(source, open).unwrap();
    assert_eq!(&source[close..], "}\n", "le bloc doit se fermer sur l'accolade de l'impl");
}
