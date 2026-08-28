//! Watching the project on disk: what wakes maxx, and what must not.

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use futures::channel::mpsc::Receiver;
use maxx::scaffold::Template;
use maxx::{scaffold, watch};

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

#[test]
fn a_build_does_not_wake_the_watcher() {
    let root = scratch("maxx_watch_target");
    scaffold::create_project(&root, "trial", Template::Empty).expect("the project must be created");

    let (mut receiver, _watcher) = watch::start(&root).expect("the watcher must start");
    std::thread::sleep(Duration::from_millis(300));

    // What `cargo run` does to the open project, in miniature: `target/` is not
    // watched at all, so this must not reach the channel.
    let build = root.join("target/debug/build");
    std::fs::create_dir_all(&build).unwrap();
    for index in 0..20 {
        std::fs::write(build.join(format!("out{index}.rs")), "fn main() {}").unwrap();
    }

    assert!(!woken(&mut receiver, Duration::from_secs(2)), "a build must not wake the canvas");
}
