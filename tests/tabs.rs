//! Moving between open views: the ring, and the way back.
//!
//! Both questions are answered without a window, which is why they live in
//! `tabs` rather than inside the workspace.

use std::path::PathBuf;

use maxx::tabs;

fn paths(names: &[&str]) -> Vec<PathBuf> {
    names.iter().map(PathBuf::from).collect()
}

#[test]
fn the_strip_is_a_ring() {
    // Forward from the last tab reaches the first: stopping there would do
    // nothing exactly when there is somewhere to go.
    assert_eq!(tabs::step(0, 3, true), Some(1));
    assert_eq!(tabs::step(2, 3, true), Some(0));
    assert_eq!(tabs::step(0, 3, false), Some(2));
    assert_eq!(tabs::step(1, 3, false), Some(0));
}

#[test]
fn one_tab_is_its_own_neighbour() {
    assert_eq!(tabs::step(0, 1, true), Some(0));
    assert_eq!(tabs::step(0, 1, false), Some(0));
}

#[test]
fn no_tab_has_no_neighbour() {
    assert_eq!(tabs::step(0, 0, true), None);
    assert_eq!(tabs::step(0, 0, false), None);
}

#[test]
fn the_way_back_follows_the_file_and_not_its_place() {
    let views = paths(&["a.rs", "b.rs", "c.rs"]);
    let previous = PathBuf::from("c.rs");
    assert_eq!(tabs::position_of(&views, Some(&previous)), Some(2));

    // A tab closed before it shifts every index after: an index kept across a
    // close names whatever slid into its place, so the path is what is kept.
    let after = paths(&["b.rs", "c.rs"]);
    assert_eq!(tabs::position_of(&after, Some(&previous)), Some(1));

    // And a file that is no longer open is nowhere to go back to.
    let gone = paths(&["a.rs", "b.rs"]);
    assert_eq!(tabs::position_of(&gone, Some(&previous)), None);
    assert_eq!(tabs::position_of(&views, None), None);
}

/// Every way of opening a file leaves a trace to come back to.
///
/// The gesture is only useful between the two files one is actually working
/// between, and those are rarely reached by clicking tabs: the tree, the menu
/// and `⌘P` all have to record where one came from. When they did not, `⌃⇥`
/// answered "nowhere to go back to" in exactly its own use case.
#[test]
fn the_trace_is_left_by_every_way_in() {
    let source = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/workspace/views.rs"),
    )
    .expect("src/workspace/views.rs");

    // `focus_view` is the one place that writes it, so nothing else may set
    // `self.active` — a second way in is a way in that forgets.
    let writes: Vec<&str> = source
        .lines()
        .map(str::trim)
        .filter(|line| line.starts_with("self.active = Some("))
        .collect();
    assert_eq!(
        writes.len(),
        1,
        "only `focus_view` may bring a view forward, or the trace is lost: {writes:?}"
    );
    assert!(source.contains("fn focus_view(&mut self, index: usize)"));
}
