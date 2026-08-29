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

#[test]
fn every_call_the_catalogue_writes_is_compiled_in_the_example() {
    let compiled = compiled();
    let mut missing = Vec::new();

    let mut expect = |needle: String, why: &str| {
        if !compiled.contains(&needle) {
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
/// `IconName` has no `FromStr`, so `designer::canvas::icon_named` is a table — and a
/// table drifts. An icon offered but not drawn shows as the fallback asterisk,
/// which looks like a bug in the icon rather than a hole in a list.
#[test]
fn every_icon_offered_is_an_icon_the_canvas_draws() {
    let designer = std::fs::read_to_string(
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/designer/canvas.rs"),
    )
    .expect("src/designer/canvas.rs");

    let icons = CATALOGUE
        .iter()
        .flat_map(|spec| spec.props.iter())
        .filter_map(|prop| match prop.target {
            Target::VariantArg(_, values) if prop.label == "prop.icon" => Some(values),
            _ => None,
        })
        .flatten();

    let missing: Vec<&str> =
        icons.filter(|icon| !designer.contains(&format!("\"{icon}\" => "))).copied().collect();

    assert!(missing.is_empty(), "offered but never drawn: {}", missing.join(", "));
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
