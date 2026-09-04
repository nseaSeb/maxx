//! Moving between the open views.
//!
//! Questions the strip asks, and not one of them needs a window: which tab
//! comes next, which file one was on before, which tabs a "close the others"
//! closes and in what order, and what a "copy the path" hands over. Written
//! here rather than inside the workspace so they can be answered — and tested
//! — without an `App`.

use std::path::{Path, PathBuf};

/// The tab after `current`, or the one before it, wrapping at the ends.
///
/// Wrapping, unlike the palette's list: a tab strip is a ring one cycles
/// through, and stopping at the last tab would make `⌘⌥→` do nothing exactly
/// when there is somewhere to go — the first tab.
pub fn step(current: usize, count: usize, forward: bool) -> Option<usize> {
    if count == 0 {
        return None;
    }
    Some(if forward { (current + 1) % count } else { (current + count - 1) % count })
}

/// Every tab but `keep`, in the order they have to be closed.
///
/// Descending, and that is the whole reason this is a function: closing a tab
/// shifts every index after it, so a list walked from the left would name
/// whatever slid into the place of the one just closed. Walked from the right,
/// no index it still holds has moved.
pub fn others(count: usize, keep: usize) -> Vec<usize> {
    (0..count).rev().filter(|index| *index != keep).collect()
}

/// Every tab after `from`, in the order they have to be closed.
///
/// Descending for the reason above, and empty when `from` is the last tab —
/// which is when the entry says nothing happened rather than closing the strip.
pub fn to_the_right(count: usize, from: usize) -> Vec<usize> {
    ((from + 1)..count).rev().collect()
}

/// What "copy the path" puts on the clipboard for the tab at `active`.
///
/// The absolute path, not one relative to the project: what is copied is
/// pasted into a terminal, an editor or another tool, none of which knows
/// where the project root is. `None` when no tab is in front, so the command
/// can say so instead of copying an empty string over what was there.
pub fn path_to_copy(views: &[PathBuf], active: Option<usize>) -> Option<String> {
    Some(views.get(active?)?.to_string_lossy().into_owned())
}

/// Where the file one was on before is now.
///
/// A path and not an index: tabs close and shift, and an index kept across a
/// close names whatever slid into its place — which is how "previous file"
/// becomes "some other file".
pub fn position_of(views: &[PathBuf], previous: Option<&Path>) -> Option<usize> {
    let previous = previous?;
    views.iter().position(|path| path == previous)
}
