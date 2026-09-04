//! The catalogue only writes calls that exist.
//!
//! Two defects of exactly this shape reached a generated project: a font weight
//! written as `font_medium()`, which gpui has never had, and a scroll written
//! as `overflow_y_scroll()` before `id()`, which gpui only offers the other way
//! round. Neither fails here, in maxx — both fail in the project the developer
//! builds, on a line maxx wrote itself.
//!
//! `examples/catalogue.rs` calls every one of them, so the compiler answers the
//! question. This test only makes sure nothing was added to the tables without
//! being added there.

use std::path::PathBuf;

use maxx::registry::{CATALOGUE, Target, props};

fn compiled() -> String {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("examples/catalogue.rs");
    std::fs::read_to_string(path).expect("examples/catalogue.rs")
}

/// The same text with every run of whitespace collapsed to one space.
fn squeezed(source: &str) -> String {
    source.split_whitespace().collect::<Vec<_>>().join(" ")
}

#[test]
fn every_call_the_catalogue_writes_is_compiled_in_the_example() {
    let compiled = compiled();
    // An initializer is one string in the table and six lines in the file, so
    // it is compared with the whitespace collapsed: what has to match is the
    // code, not where `rustfmt` decided to break it.
    let flattened = squeezed(&compiled);
    let mut missing = Vec::new();

    let mut expect = |needle: String, why: &str| {
        if !compiled.contains(&needle) && !flattened.contains(&squeezed(&needle)) {
            missing.push(format!("{needle} ({why})"));
        }
    };

    for spec in CATALOGUE {
        for call in spec.default_calls {
            expect(format!(".{call}()"), spec.id);
        }
        for prop in props(spec) {
            match prop.target {
                Target::BaseArg(_) => {}
                Target::Method(name) => expect(format!(".{name}("), prop.label),
                Target::Flag(name) => expect(format!(".{name}()"), prop.label),
                Target::Family(names) => {
                    for name in names {
                        expect(format!(".{name}()"), prop.label);
                    }
                }
                Target::Variant(method, values) => {
                    expect(format!(".{method}("), prop.label);
                    for value in values {
                        expect((*value).to_string(), prop.label);
                    }
                }
                // The constructor's own argument: the base is compiled with it,
                // so what has to appear is each variant it may hold.
                //
                // Except the icons: the eighty-six are generated, and the match
                // `build.rs` writes into `designer::canvas` names every one of
                // them — which is the same compiler answering the same question,
                // without eighty-six lines copied into this file to say it twice.
                Target::VariantArg(_, _) if prop.label == "prop.icon" => {}
                Target::VariantArg(_, values) => {
                    for value in values {
                        expect((*value).to_string(), prop.label);
                    }
                }
                Target::Scrollable(name) => expect(format!(".{name}()"), prop.label),
                // Everything the switch writes, on the two types it writes it on.
                Target::Scrollbar => {
                    expect(".track_scroll(".into(), prop.label);
                    expect(".relative()".into(), prop.label);
                    expect("Scrollbar::new(".into(), prop.label);
                    expect("ScrollbarAxis::Vertical".into(), prop.label);
                }
                // The closure gpui takes, and the type it builds inside it.
                Target::Tooltip => {
                    expect(".tooltip(".into(), prop.label);
                    expect("Tooltip::new(".into(), prop.label);
                }
                // The expression the field is wrapped in: the constructor takes
                // a `Keystroke`, and the text is parsed into one.
                Target::Keystroke(_) => {
                    expect("Keystroke::parse(".into(), prop.label);
                    expect(".unwrap_or_default()".into(), prop.label);
                }
                // One call taking an array of literals — the only spelling a
                // typed child accepts from a string.
                Target::Labels(name) => expect(format!(".{name}(["), prop.label),
                // Not a call on the element but the constructor the view's
                // `new` calls, so what has to be compiled is the initializer
                // itself — both shapes of it, since the switch writes one or
                // the other and only one of them is ever in a project.
                // The closure, and the call written inside it — which is the
                // ordinary call, so the name is proved on a `div` like the rest.
                Target::Hover(inner) => {
                    expect(".hover(|".into(), prop.label);
                    match inner {
                        Target::Method(name) => expect(format!(".{name}("), prop.label),
                        Target::Family(names) => {
                            for name in names.iter() {
                                expect(format!(".{name}()"), prop.label);
                            }
                        }
                        other => panic!("{other:?} is not something a hover writes"),
                    }
                }
                Target::Initializer(init) => {
                    if let Some(off) = init.off {
                        expect(off.to_string(), prop.label);
                    }
                    // The slot is a value the inspector supplies; what the
                    // compiler has to see is the text around it.
                    if let Some((prefix, _)) = init.on.split_once("{}") {
                        expect(prefix.trim_end().trim_end_matches(',').to_string(), prop.label);
                    }
                }
            }
        }
    }

    assert!(
        missing.is_empty(),
        "these are written into generated projects and compiled nowhere:\n{}",
        missing.join("\n")
    );
}

/// The id comes before the overflow, and the example says so.
///
/// `overflow_y_scroll` lives on a stateful element: gpui offers it only after
/// `id`. The order is not a matter of taste, and it is not something the model
/// can be trusted to remember on its own.
#[test]
fn the_scroll_example_keeps_the_id_first() {
    let compiled = compiled();
    for axis in ["overflow_y_scroll", "overflow_x_scroll"] {
        // The prose above says the same thing in words; it is the code that has
        // to be checked.
        let line = compiled
            .lines()
            .find(|line| line.contains(axis) && !line.trim_start().starts_with("//"))
            .unwrap_or_else(|| panic!("{axis} is compiled nowhere"));
        let id = line.find(".id(").unwrap_or(usize::MAX);
        let overflow = line.find(axis).expect("just found");
        assert!(id < overflow, "the id has to come first: {line}");
    }
}

/// Every icon the inspector offers is one the canvas can draw.
///
/// Both sides come from `build.rs` now, so they cannot drift by construction —
/// but the two are wired together by hand in two files, and a wiring can be cut.
/// Asked of the function rather than of the source text: what matters is that
/// the canvas answers, not that a line reads a certain way.
#[test]
fn every_icon_offered_is_an_icon_the_canvas_draws() {
    let missing: Vec<&str> =
        offered_icons().filter(|icon| maxx::designer::icon_name(icon).is_none()).collect();

    assert!(missing.is_empty(), "offered but never drawn: {}", missing.join(", "));
}

/// The icons the inspector puts in front of anyone, read off the catalogue.
fn offered_icons() -> impl Iterator<Item = &'static str> {
    CATALOGUE
        .iter()
        .flat_map(|spec| spec.props.iter())
        .filter_map(|prop| match prop.target {
            Target::VariantArg(_, values) if prop.label == "prop.icon" => Some(values),
            _ => None,
        })
        .flatten()
        .copied()
}

/// And the fallback is a fallback: a name the crate does not carry is refused.
///
/// The half that makes the test above mean something — a function answering
/// `Some` to everything would pass it without drawing a thing.
#[test]
fn a_name_the_crate_does_not_carry_is_refused() {
    assert!(maxx::designer::icon_name("IconName::NoSuchIcon").is_none());
    assert!(maxx::designer::icon_name("self.icon.clone()").is_none());
}

/// Whatever maxx writes into an initializer, it can read back.
///
/// The two are two strings in the table — the shape a fresh drop gets, and the
/// shape the property writes — and nothing but this holds them together. Let
/// them drift and the field goes quiet: the value no longer reads back, so the
/// inspector decides the line is the developer's and refuses to touch it, on a
/// line maxx wrote a second earlier.
#[test]
fn every_initializer_maxx_writes_reads_back() {
    let mut deaf = Vec::new();
    for spec in CATALOGUE {
        let Some(state) = spec.state else { continue };
        for prop in props(spec) {
            if let Target::Initializer(init) = prop.target
                && init.read(prop.kind, state.initializer).is_none()
            {
                deaf.push(format!("{} / {}", spec.id, prop.label));
            }
        }
    }
    assert!(
        deaf.is_empty(),
        "these initializers are written by maxx and unreadable to it:\n{}",
        deaf.join("\n")
    );
}

/// The tooltip comes after the id, and the example says so.
///
/// `tooltip` lives on a stateful element: gpui offers it only once `id` has
/// been called, exactly like the scroll above. Written the other way round, the
/// chain does not compile — in the developer's project, on a line maxx wrote.
#[test]
fn the_tooltip_example_keeps_the_id_first() {
    let compiled = compiled();
    let line = compiled
        .lines()
        .find(|line| line.contains(".tooltip(|") && !line.trim_start().starts_with("//"))
        .expect("the tooltip closure is compiled nowhere");
    let id = line.find(".id(").unwrap_or(usize::MAX);
    let tooltip = line.find(".tooltip(").unwrap_or(0);
    assert!(id < tooltip, "the id has to come first: {line}");
}

/// Every property of the catalogue is filed under a heading, deliberately.
///
/// The table is keyed on the label, so a property added without an answer there
/// falls back to `Appearance` — misfiled rather than invisible, which is the
/// right failure but still a failure. This is what says so before it ships.
#[test]
fn every_property_of_the_catalogue_has_a_heading() {
    let mut unfiled: Vec<&str> = Vec::new();
    for spec in CATALOGUE {
        for prop in maxx::registry::props(spec) {
            if !maxx::registry::has_group(prop) && !unfiled.contains(&prop.label) {
                unfiled.push(prop.label);
            }
        }
    }
    assert!(
        unfiled.is_empty(),
        "no heading for: {}\nAdd them to registry::GROUPS.",
        unfiled.join(", ")
    );
}

/// And the table names nothing the catalogue does not.
///
/// The other half: a label left behind by a rename is a line nobody will ever
/// read again, and the sign that the property it filed has moved.
#[test]
fn the_heading_table_names_no_property_that_is_gone() {
    let mut known: Vec<&str> = Vec::new();
    for spec in CATALOGUE {
        for prop in maxx::registry::props(spec) {
            known.push(prop.label);
        }
    }
    let orphans: Vec<&str> =
        maxx::registry::grouped_labels().into_iter().filter(|l| !known.contains(l)).collect();
    assert!(orphans.is_empty(), "filed but not in the catalogue: {}", orphans.join(", "));
}

/// An initializer the developer extended is not maxx's to rewrite.
///
/// `Init::read` answers `Some` for the line maxx wrote and `None` for anything
/// else, and that answer is the *whole* proof the inspector uses before
/// overwriting. Stripping a prefix and a suffix says where maxx's line starts
/// and ends, not that what sits between them is only the value: an input whose
/// state gained a `.placeholder("Nom")` in `new` left `true).placeholder("Nom"`
/// between them, and the flag was read as if maxx owned the line. Toggling the
/// switch then rewrote it from the table and the placeholder was gone —
/// silently, in the developer's own file.
#[test]
fn an_initializer_the_developer_extended_is_left_alone() {
    let Some(spec) = maxx::registry::by_id("input") else {
        panic!("the text input is in the catalogue");
    };
    let Some((init, kind)) = spec.props.iter().find_map(|prop| match prop.target {
        maxx::registry::Target::Initializer(init) => Some((init, prop.kind)),
        _ => None,
    }) else {
        panic!("the text input carries an initializer property");
    };

    // What maxx writes, both ways: read back as the value and nothing else.
    let on = init.write(kind, "true").expect("the on shape");
    assert_eq!(init.read(kind, &on).as_deref(), Some("true"), "maxx reads its own line");
    let off = init.write(kind, "false").expect("the off shape");
    assert_eq!(init.read(kind, &off).as_deref(), Some(""), "and its other one");

    // The same line with one call added by hand. `None` is what leaves it be.
    let extended = on.replace("))", ").placeholder(\"Nom\"))");
    assert_ne!(extended, on, "the fixture really does add a call");
    assert_eq!(
        init.read(kind, &extended),
        None,
        "a line the developer extended is not maxx's to rewrite: {extended}"
    );

    // And a value that is not the flag at all.
    let odd = on.replace("true", "self.wants_lines");
    assert_eq!(init.read(kind, &odd), None, "nor is an expression maxx cannot read: {odd}");
}
