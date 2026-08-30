//! Watching the project on disk: what wakes maxx, and what must not.

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use futures::channel::mpsc::Receiver;
use maxx::scaffold::Template;
use maxx::{scaffold, watch};
use notify::Event;
use notify::event::{AccessKind, DataChange, EventKind, ModifyKind};

fn scratch(name: &str) -> PathBuf {
    let dir =
        std::env::var("MAXX_SCRATCH").map(PathBuf::from).unwrap_or_else(|_| std::env::temp_dir());
    let path = dir.join(name);
    let _ = std::fs::remove_dir_all(&path);
    path
}

/// Waits up to `timeout` for one wake-up, and reports whether it came.
///
/// Polled rather than awaited: the channel is an async one, and a test has no
/// executor to await it on. `try_recv` answers `Err` both while the channel is
/// empty and once it is closed, which is the same answer here — no wake-up.
fn woken(receiver: &mut Receiver<()>, timeout: Duration) -> bool {
    let deadline = Instant::now() + timeout;
    while Instant::now() < deadline {
        if receiver.try_recv().is_ok() {
            return true;
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    false
}

#[test]
fn the_files_a_developer_edits_are_worth_waking_for() {
    let root = Path::new("/projects/trial");

    assert!(watch::interesting(root, &root.join("src/ui/home.rs")));
    assert!(watch::interesting(root, &root.join("maxx.toml")));
    assert!(watch::interesting(root, &root.join("Cargo.toml")));
    // The panel shows every file, so a note written elsewhere has to show up.
    assert!(watch::interesting(root, &root.join("NOTES.md")));
}

#[test]
fn the_noise_is_left_out() {
    let root = Path::new("/projects/trial");

    assert!(!watch::interesting(root, &root.join("target/debug/build/x.rs")));
    assert!(!watch::interesting(root, &root.join(".git/index")));
    assert!(!watch::interesting(root, &root.join("node_modules/x/index.js")));
    assert!(!watch::interesting(root, &root.join("src/ui/.home.rs.swp")));
    assert!(!watch::interesting(root, &root.join("src/ui/home.rs~")));
    assert!(!watch::interesting(root, &root.join("src/ui/#home.rs#")));
    // Outside the project entirely.
    assert!(!watch::interesting(root, Path::new("/elsewhere/home.rs")));
}

#[test]
fn a_project_under_a_hidden_directory_is_still_watched() {
    // Judged relative to the root: `~/.local/share/trial` is a hidden path, and
    // every one of its files would be rejected if the whole path were read.
    let root = Path::new("/home/dev/.local/share/trial");
    assert!(watch::interesting(root, &root.join("src/ui/home.rs")));
}

#[test]
fn an_edit_made_elsewhere_wakes_the_watcher() {
    // The whole thread-and-channel path, without opening a window — the same
    // shape as the runner's test in `tests/scaffold.rs`.
    let root = scratch("maxx_watch");
    scaffold::create_project(&root, "trial", Template::Empty).expect("the project must be created");

    let (mut receiver, _watcher) = watch::start(&root).expect("the watcher must start");
    // The watch is armed asynchronously on some platforms; a write landing in
    // that gap would be missed for reasons that have nothing to do with maxx.
    std::thread::sleep(Duration::from_millis(300));

    let path = root.join("src/ui/home.rs");
    let outside = std::fs::read_to_string(&path).unwrap().replace("Welcome", "Changed in Zed");
    std::fs::write(&path, &outside).unwrap();

    assert!(
        woken(&mut receiver, Duration::from_secs(10)),
        "a view written outside maxx must wake it"
    );
}

/// What the watcher decides, asked directly.
///
/// This used to be an end-to-end test: write twenty files into `target/`, then
/// assert that nothing reached the channel within two seconds. Asserting an
/// absence against a clock proves nothing and fails at random — it went red on
/// macOS in one run and green in the next, on the same commit, and a guard that
/// reddens by chance is a guard people learn to ignore.
///
/// The decision is a function of the event and the root, so it is asked as one.
#[test]
fn what_wakes_the_window_and_what_does_not() {
    let root = Path::new("/projects/trial");
    let event = |kind: EventKind, path: &str| Event::new(kind).add_path(root.join(path));
    let write = EventKind::Modify(ModifyKind::Data(DataChange::Content));

    assert!(watch::wakes(root, &event(write, "src/ui/home.rs")), "an edit wakes it");

    // What `cargo run` does to the open project: never a reason to wake.
    assert!(!watch::wakes(root, &event(write, "target/debug/build/out.rs")));
    // A read is not a change. FSEvents does not report them; inotify does, and
    // rust-analyzer walking `src/` would ping on every file it opens.
    assert!(!watch::wakes(root, &event(EventKind::Access(AccessKind::Read), "src/ui/home.rs")));

    // One event carries several paths, and one worth waking for is enough.
    let mixed = Event::new(write)
        .add_path(root.join("target/debug/build/out.rs"))
        .add_path(root.join("src/ui/home.rs"));
    assert!(watch::wakes(root, &mixed), "the noise beside a real edit is still an edit");
}
