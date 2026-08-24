//! Running the open project from inside maxx.
//!
//! `cargo run` is spawned in a thread rather than an async task because the
//! reads on its pipes are blocking; the thread talks back over a channel that a
//! foreground task drains a few times a second.

use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::mpsc::{Receiver, Sender, channel};

/// What the runner thread sends back.
pub enum Message {
    /// The operating system pid, so the run can be stopped.
    Started(u32),
    /// One line of output, from either stream.
    Line(String),
    /// The process is over; `true` if it exited successfully.
    Finished(bool),
}

/// Where a run is in its life.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum State {
    /// Nothing has been run yet.
    Idle,
    /// `cargo run` is building or the application is up.
    Running,
    /// The process exited.
    Finished {
        /// Whether it exited with a success status.
        ok: bool,
    },
}

/// Starts `cargo run` in `root` and returns the channel its output arrives on.
pub fn start(root: PathBuf) -> Receiver<Message> {
    spawn_cargo(root, "run")
}

/// Starts `cargo build` in `root`, to pay the cost of the dependency tree while
/// the user is still drawing.
pub fn prewarm(root: PathBuf) -> Receiver<Message> {
    spawn_cargo(root, "build")
}

/// The directory every generated project compiles into.
///
/// Sharing it is what makes the second project cheap: `gpui` and
/// `gpui-component` are around 750 crates, and a project with its own `target/`
/// recompiles all of them. The path is written into the project's
/// `.cargo/config.toml` rather than passed as an environment variable, so a
/// `cargo run` typed in a terminal lands in the same cache.
pub fn shared_target_dir() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".into());
    PathBuf::from(home).join("Library/Caches/maxx/target")
}

fn spawn_cargo(root: PathBuf, subcommand: &'static str) -> Receiver<Message> {
    let (sender, receiver) = channel();
    std::thread::spawn(move || run(root, subcommand, sender));
    receiver
}

/// Opens a terminal in `path`.
///
/// Ghostty needs its own `--working-directory` flag: `open -a Ghostty <dir>`
/// launches the application but does not necessarily start the shell there.
/// Terminal.app, which every Mac has, is the fallback and does honour a
/// directory argument.
pub fn open_terminal(path: &Path) {
    if Path::new("/Applications/Ghostty.app").exists() {
        let opened = Command::new("open")
            .arg("-na")
            .arg("Ghostty")
            .arg("--args")
            .arg(format!("--working-directory={}", path.display()))
            .status()
            .map(|status| status.success())
            .unwrap_or(false);
        if opened {
            return;
        }
    }
    let _ = Command::new("open").arg("-a").arg("Terminal").arg(path).status();
}

/// Opens `path` in Zed, through its command line tool when it is installed and
/// through the application bundle otherwise.
pub fn open_editor(path: &Path) {
    if Command::new("zed").arg(path).status().is_ok() {
        return;
    }
    let _ = Command::new("open").arg("-a").arg("Zed").arg(path).status();
}

/// Opens `path` in Zed at `line`.
pub fn open_editor_at(path: &Path, line: usize) {
    let target = format!("{}:{}", path.display(), line);
    if Command::new("zed").arg(&target).status().is_ok() {
        return;
    }
    let _ = Command::new("open").arg("-a").arg("Zed").arg(path).status();
}

/// Kills a run by its operating system pid.
///
/// The child is a `cargo` process that has itself spawned the application, so
/// the whole process group is signalled — killing `cargo` alone would leave the
/// window it opened on screen.
pub fn stop(pid: u32) {
    let _ = Command::new("kill")
        .arg("-TERM")
        .arg(format!("-{pid}"))
        .status();
    let _ = Command::new("kill").arg("-TERM").arg(pid.to_string()).status();
}

fn run(root: PathBuf, subcommand: &str, sender: Sender<Message>) {
    use std::os::unix::process::CommandExt as _;

    let child = Command::new("cargo")
        .arg(subcommand)
        .current_dir(&root)
        // Colour codes would end up in the panel as escape sequences.
        .env("CARGO_TERM_COLOR", "never")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        // Without this the child stays in maxx's own group, and signalling
        // `-pid` reaches a group that does not exist — or someone else's.
        .process_group(0)
        .spawn();

    let mut child = match child {
        Ok(child) => child,
        Err(error) => {
            let _ = sender.send(Message::Line(format!("cargo {subcommand} : {error}")));
            let _ = sender.send(Message::Finished(false));
            return;
        }
    };

    let _ = sender.send(Message::Started(child.id()));

    // cargo writes its progress and its errors to stderr, the application
    // writes to stdout; both belong in the panel.
    let stderr = child.stderr.take();
    let to_stderr = sender.clone();
    let pump = std::thread::spawn(move || {
        if let Some(stderr) = stderr {
            for line in BufReader::new(stderr).lines().map_while(Result::ok) {
                if to_stderr.send(Message::Line(line)).is_err() {
                    return;
                }
            }
        }
    });

    if let Some(stdout) = child.stdout.take() {
        for line in BufReader::new(stdout).lines().map_while(Result::ok) {
            if sender.send(Message::Line(line)).is_err() {
                break;
            }
        }
    }
    let _ = pump.join();

    let ok = child.wait().map(|status| status.success()).unwrap_or(false);
    let _ = sender.send(Message::Finished(ok));
}
