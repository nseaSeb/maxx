//! Moving between open views, and closing them: the ring, the way back, what
//! "close the others" closes, and what "copy the path" hands over.
//!
//! All of it is answered without a window, which is why it lives in `tabs`
//! rather than inside the workspace — the context menus that call it cannot be
//! tested without pixels, but what they command can.

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

/// Closing the other tabs walks them from the right.
///
/// The order is the whole content of the function: closing a tab shifts every
/// index after it, so a list walked from the left would close whatever slid
/// into the place of the one just closed — the third tab, then the fifth as it
/// stands, then nothing where the sixth used to be.
#[test]
fn the_others_are_closed_from_the_right() {
    assert_eq!(tabs::others(4, 1), vec![3, 2, 0]);
    // The kept tab is never named, wherever it sits.
    assert_eq!(tabs::others(3, 0), vec![2, 1]);
    assert_eq!(tabs::others(3, 2), vec![1, 0]);
    // A single tab has no others, which is what lets the entry say so rather
    // than close the strip.
    assert!(tabs::others(1, 0).is_empty());
}

#[test]
fn the_tabs_to_the_right_stop_at_the_end() {
    assert_eq!(tabs::to_the_right(4, 1), vec![3, 2]);
    // On the last tab there is nothing to the right — and above all, the tabs
    // to the *left* are not it.
    assert!(tabs::to_the_right(4, 3).is_empty());
    assert!(tabs::to_the_right(0, 0).is_empty());
}

/// What "copy the path" hands over is the absolute path, and nothing when no
/// tab is in front.
///
/// Absolute because what is copied leaves maxx: a terminal, another editor or
/// a bug report knows nothing of the project root. And `None` rather than an
/// empty string, so the command can say there is no view instead of wiping
/// whatever was on the clipboard.
#[test]
fn the_copied_path_is_the_whole_path() {
    let views = paths(&["/tmp/trial/src/ui/home.rs", "/tmp/trial/src/ui/about.rs"]);
    assert_eq!(tabs::path_to_copy(&views, Some(1)).as_deref(), Some("/tmp/trial/src/ui/about.rs"));
    assert_eq!(tabs::path_to_copy(&views, None), None);
    // An index past the strip is nothing to copy either: the active tab and the
    // list are two fields, and they part company for a frame when one closes.
    assert_eq!(tabs::path_to_copy(&views, Some(7)), None);
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
