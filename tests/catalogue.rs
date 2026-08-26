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
                Target::Scrollable(name) => expect(format!(".{name}()"), prop.label),
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
