//! Moving between the open views.
//!
//! Two questions, and neither needs a window: which tab comes next, and which
//! file one was on before. Written here rather than inside the workspace so
//! they can be answered — and tested — without an `App`.

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

/// Where the file one was on before is now.
///
/// A path and not an index: tabs close and shift, and an index kept across a
/// close names whatever slid into its place — which is how "previous file"
/// becomes "some other file".
pub fn position_of(views: &[PathBuf], previous: Option<&Path>) -> Option<usize> {
    let previous = previous?;
    views.iter().position(|path| path == previous)
}
