//! The project model: a root directory plus the flattened tree of entries that
//! the project panel renders.

use gpui::SharedString;
use std::collections::HashSet;
use std::path::{Path, PathBuf};

/// Beyond this, a picture is refused rather than read.
///
/// Higher than the reader's ceiling for text, because the cost is different:
/// decoding a photograph once, not parsing a buffer with tree-sitter. A ceiling
/// all the same — a window frozen on a hundred-megabyte file looks like a crash
/// whatever the reason, and importing one maxx would then refuse to show would
/// be worse still.
pub const MAX_IMAGE_BYTES: u64 = 16 * 1024 * 1024;

/// Whether this file is a picture maxx can show.
///
/// gpui's own list, read back rather than written a second time: an extension
/// it cannot decode would draw nothing, with no error to see.
pub fn is_image(path: &Path) -> bool {
    let extension =
        path.extension().and_then(|value| value.to_str()).unwrap_or_default().to_lowercase();
    gpui::Img::extensions().contains(&extension.as_str())
}

/// A directory opened as a project.
pub struct Project {
    /// Absolute path of the project root.
    pub root: PathBuf,
    /// Display name of the project, i.e. the last component of `root`.
    pub name: SharedString,
}

impl Project {
    /// Opens `root` as a project, canonicalizing the path so the titlebar and
    /// the recent-documents list always show an absolute location.
    pub fn open(root: PathBuf) -> Self {
        let root = std::fs::canonicalize(&root).unwrap_or(root);
        let name = root
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_else(|| root.to_string_lossy().into_owned());

        Self { root, name: name.into() }
    }
}

/// A single row of the project panel.
#[derive(Clone)]
pub struct Entry {
    /// Absolute path of the file or directory.
    pub path: PathBuf,
    /// Name shown in the panel.
    pub name: SharedString,
    /// Whether this entry can be expanded.
    pub is_dir: bool,
    /// Nesting level relative to the project root, used for indentation.
    pub depth: usize,
}

/// Beyond this, the quick-open list stops growing.
///
/// Not a limit anyone reaches by hand: it is the guard against a project that
/// holds a vendored dependency tree, where walking everything would freeze the
/// window on `⌘P` — the one gesture that has to feel instant.
pub const MAX_QUICK_OPEN_FILES: usize = 5_000;

/// Every file of the project, relative to its root, for the quick-open list.
///
/// The same exclusions the panel uses — dotfiles, `target`, `node_modules` —
/// because a file maxx hides in the tree is a file nobody is looking for in the
/// palette either.
pub fn walk_files(root: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(directory) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&directory) else {
            continue;
        };
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().into_owned();
            if name.starts_with('.') || name == "target" || name == "node_modules" {
                continue;
            }
            let path = entry.path();
            if entry.file_type().map(|kind| kind.is_dir()).unwrap_or(false) {
                stack.push(path);
            } else if let Ok(relative) = path.strip_prefix(root) {
                out.push(relative.to_path_buf());
                if out.len() >= MAX_QUICK_OPEN_FILES {
                    // Sorted on this way out too: a project at the ceiling
                    // would otherwise get the list in the order the disk
                    // happened to hand it over.
                    out.sort();
                    return out;
                }
            }
        }
    }
    out.sort();
    out
}

/// Reads the direct children of `dir`, hiding dotfiles and build output.
///
/// Directories sort before files, then alphabetically, case-insensitively.
/// Unreadable directories yield an empty list rather than an error: a missing
/// row in the panel is a better outcome than failing to open the project.
pub fn read_children(dir: &Path, depth: usize) -> Vec<Entry> {
    let Ok(read_dir) = std::fs::read_dir(dir) else {
        return Vec::new();
    };

    let mut entries: Vec<Entry> = read_dir
        .flatten()
        .filter_map(|entry| {
            let name = entry.file_name().to_string_lossy().into_owned();
            if name.starts_with('.') || name == "target" || name == "node_modules" {
                return None;
            }

            let is_dir = entry.file_type().map(|ty| ty.is_dir()).unwrap_or(false);
            Some(Entry { path: entry.path(), name: name.into(), is_dir, depth })
        })
        .collect();

    entries.sort_by(|a, b| {
        b.is_dir.cmp(&a.is_dir).then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
    });
    entries
}

/// Rebuilds the flattened entry list for `root`, descending into every
/// directory present in `expanded`.
///
/// The tree is read lazily on purpose: only expanded directories are visited,
/// so opening a large project never walks it in full.
pub fn flatten(root: &Path, expanded: &HashSet<PathBuf>) -> Vec<Entry> {
    let mut entries = Vec::new();
    push_children(root, 0, expanded, &mut entries);
    entries
}

fn push_children(dir: &Path, depth: usize, expanded: &HashSet<PathBuf>, out: &mut Vec<Entry>) {
    for entry in read_children(dir, depth) {
        let expand = entry.is_dir && expanded.contains(&entry.path);
        let path = entry.path.clone();
        out.push(entry);
        if expand {
            push_children(&path, depth + 1, expanded, out);
        }
    }
}
