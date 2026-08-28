//! Watching the open project on disk, so an edit made elsewhere reaches the
//! canvas without waiting for the window to come back.
//!
//! The same split as `run`: the operating system's half lives here, knows
//! nothing of `Workspace` or `App`, and talks back over a channel the
//! foreground drains — which is what makes it testable without a window.

use std::path::Path;

use futures::channel::mpsc::{Receiver, channel};
use notify::{RecommendedWatcher, RecursiveMode, Watcher};

/// Starts watching `root` and returns the channel its changes arrive on.
///
/// The watcher itself comes back with the receiver, and has to be kept alive:
/// dropped, it stops watching within the second. `None` means the platform
/// refused — too many watches, a filesystem with no notifications — and maxx
/// then falls back to the check it already does when the window regains focus.
///
/// What is watched is `src/` in full, plus the root's own files. Not the root
/// in full: on Linux a recursive watch registers one inotify watch per
/// directory, so `target/` alone can exhaust `max_user_watches` and take the
/// whole watch down with it. On macOS the saving is smaller, and worth saying
/// honestly — FSEvents streams the subtree either way, and `notify` filters a
/// non-recursive path in its own callback — but a build's writes are then
/// dropped one step earlier than `interesting` would drop them.
///
/// The cost, and it is real: a picture dropped into `assets/` from outside is
/// not noticed until the window regains focus.
pub fn start(root: &Path) -> Option<(Receiver<()>, RecommendedWatcher)> {
    // One slot, and a send that gives up rather than waits: the ping carries no
    // path, so a second one queued behind the first says nothing new. A `git
    // checkout` rewriting five hundred files becomes one wake-up.
    let (mut sender, receiver) = channel(1);

    // Canonicalized once, and used for the watch as well as the filter:
    // `notify` builds an event's path from the path it was handed — only the
    // macOS backend resolves it — so registering one spelling and filtering
    // against another rejects every event. Both platforms have a way of
    // spelling the same directory twice: `/var/folders/…` for
    // `/private/var/folders/…` on macOS, the 8.3 short path on Windows.
    // `Project::open` canonicalizes for its own reasons, so in the running
    // application the two already agree.
    let root = std::fs::canonicalize(root).unwrap_or_else(|_| root.to_path_buf());
    let filter = root.clone();

    let mut watcher = notify::recommended_watcher(move |event: notify::Result<notify::Event>| {
        let Ok(event) = event else {
            return;
        };
        // A read is not a change. FSEvents does not report them; inotify does,
        // and rust-analyzer walking `src/` would ping on every file it opens.
        if event.kind.is_access() {
            return;
        }
        if !event.paths.iter().any(|path| interesting(&filter, path)) {
            return;
        }
        // The channel carries no path: `check_disk` sweeps every open view and
        // re-reads each file anyway, so what changed is not the question — only
        // that something did.
        let _ = sender.try_send(());
    })
    .map_err(|error| eprintln!("the project is not being watched: {error}"))
    .ok()?;

    // Either watch alone is worth keeping: `Open Folder…` takes any directory,
    // and one without a `src/` would otherwise lose the root watch with it.
    let mut watched = watcher.watch(&root.join("src"), RecursiveMode::Recursive).is_ok();
    // Non-recursive on the root, which is what notices `maxx.toml`, `Cargo.toml`
    // and any file created beside them.
    watched |= watcher.watch(&root, RecursiveMode::NonRecursive).is_ok();
    if !watched {
        // Said to the log and not to the status bar: a developer cannot act on
        // it, nothing they asked for is lost, and coming back to the window
        // still reloads. `settings::save` reports its own failures the same way.
        eprintln!("the project is not being watched: {}", root.display());
        return None;
    }

    Some((receiver, watcher))
}

/// Whether a change at `path` is worth waking the workspace for.
///
/// It excludes rather than allows: the project panel shows every file, so a
/// `NOTES.md` written in the editor has to appear there too. The exclusions are
/// the ones `project::read_children` applies to the tree — dotfiles, `target`,
/// `node_modules` — plus what an editor leaves behind mid-save. That last group
/// is the one place the two lists differ, and knowingly: the panel would show a
/// `home.rs~`, but waking the window for a file the editor is about to delete
/// buys nothing, and the row arrives with the next wake-up anyway.
///
/// Judged relative to `root`, and not on the whole path: a project living under
/// a hidden directory — `~/.local/share/…` — would otherwise have every one of
/// its files rejected.
pub fn interesting(root: &Path, path: &Path) -> bool {
    // Outside the project: not ours to react to. `notify` reports the paths it
    // was given, so this only fires on a root that moved under the watch.
    let Ok(inside) = path.strip_prefix(root) else {
        return false;
    };

    let mut names = inside.components().filter_map(|component| match component {
        std::path::Component::Normal(name) => name.to_str(),
        _ => None,
    });
    if names.any(|name| name.starts_with('.') || name == "target" || name == "node_modules") {
        return false;
    }

    let Some(name) = inside.file_name().and_then(|name| name.to_str()) else {
        return false;
    };
    // What an editor leaves behind while saving: Vim's swap file, Emacs' `#…#`,
    // and the `name~` backup both of them write.
    !(name.ends_with('~')
        || name.ends_with(".tmp")
        || name.ends_with(".swp")
        || name.ends_with('#'))
}
