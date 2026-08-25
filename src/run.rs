//! Running the open project from inside maxx.
//!
//! `cargo run` is spawned in a thread rather than an async task because the
//! reads on its pipes are blocking; the thread talks back over a channel that a
//! foreground task drains a few times a second.

use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use crate::tools::{Editor, LineArgument, Terminal, on_path};
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

/// Opens `terminal` at `path`.
///
/// Two ways in, and the first one matters: a terminal with a command line tool
/// takes its working directory as a flag, while `open -a <App> <dir>` launches
/// the application without necessarily starting the shell there. Terminal.app,
/// which every Mac has, is the fallback that does honour a directory argument.
pub fn open_terminal(terminal: Option<&Terminal>, path: &Path) {
    let Some(terminal) = terminal else {
        return;
    };

    // `spawn` and never `status`: a terminal does not exit until its window is
    // closed, and these are called from the interface thread. Waiting on one
    // freezes maxx until the user quits the terminal.
    if !terminal.command.is_empty()
        && let Some(flag) = terminal.directory_flag
        && on_path(terminal.command)
        && Command::new(terminal.command)
            // The `--flag=value` form, not two arguments: Ghostty accepts only
            // that one, and every other terminal here takes it too.
            .arg(format!("{flag}={}", path.display()))
            .spawn()
            .is_ok()
    {
        return;
    }

    if let Some(bundle) = terminal.bundle {
        // `-n` and `--args` are what carry the flag through to a bundle that
        // has no command line tool of its own.
        if let Some(flag) = terminal.directory_flag {
            let opened = Command::new("open")
                .arg("-na")
                .arg(bundle)
                .arg("--args")
                .arg(format!("{flag}={}", path.display()))
                .spawn()
                .is_ok();
            if opened {
                return;
            }
        }
        let _ = Command::new("open").arg("-a").arg(bundle).arg(path).spawn();
    }
}

/// The argument list that opens `path` in `editor`, at `line` when given.
///
/// A table, not a heuristic: every editor spells this differently, and there
/// is no majority to follow.
pub fn editor_arguments(editor: &Editor, path: &Path, line: Option<usize>) -> Vec<String> {
    let file = path.display().to_string();
    let Some(line) = line else {
        return vec![file];
    };
    match editor.line {
        LineArgument::Suffix => vec![format!("{file}:{line}")],
        LineArgument::Flag(flag) => vec![flag.to_string(), format!("{file}:{line}")],
        LineArgument::PlusLine => vec![format!("+{line}"), file],
        LineArgument::Named(name) => vec![name.to_string(), line.to_string(), file],
    }
}

/// Opens `path` in a windowed editor.
///
/// Through its command line tool when it has one, and through the application
/// bundle otherwise — losing the line number in that second case, because
/// `open -a` has nowhere to put it.
pub fn open_editor(editor: &Editor, path: &Path, line: Option<usize>) {
    let arguments = editor_arguments(editor, path, line);
    if on_path(editor.command)
        && Command::new(editor.command).args(&arguments).spawn().is_ok()
    {
        return;
    }
    if let Some(bundle) = editor.bundle {
        let _ = Command::new("open").arg("-a").arg(bundle).arg(path).spawn();
    }
}

/// Opens `path` in an editor that draws inside a terminal.
///
/// The two settings are not independent here: the editor needs a terminal
/// around it, and not every terminal can be handed a command — Terminal.app's
/// only way in is AppleScript, which asks for an automation permission in the
/// middle of a click.
pub fn open_editor_in_terminal(
    editor: &Editor,
    terminal: Option<&Terminal>,
    path: &Path,
    line: Option<usize>,
) {
    let Some(terminal) = terminal else {
        return;
    };
    let Some(flag) = terminal.command_flag else {
        eprintln!(
            "{} a besoin d'un terminal capable de lancer une commande ; {} n'en est pas un",
            editor.label, terminal.label
        );
        return;
    };

    let arguments = editor_arguments(editor, path, line);
    let directory = path.parent().unwrap_or(path);

    let mut command = Command::new(terminal.command);
    if let Some(directory_flag) = terminal.directory_flag {
        command.arg(format!("{directory_flag}={}", directory.display()));
    }
    command.arg(flag).arg(editor.command).args(&arguments);

    if on_path(terminal.command) && command.spawn().is_ok() {
        return;
    }

    if let Some(bundle) = terminal.bundle {
        let mut passed = vec![flag.to_string(), editor.command.to_string()];
        passed.extend(arguments);
        let _ = Command::new("open")
            .arg("-na")
            .arg(bundle)
            .arg("--args")
            .args(&passed)
            .spawn();
    }
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

/// Moves `path` to the user's Trash and answers where it landed.
///
/// A plain rename rather than a call to the Finder: scripting the Finder needs
/// an automation permission the first time, and a prompt in the middle of a
/// delete is worse than losing the "Put Back" entry. The name is made unique
/// because `~/.Trash` may already hold a file of that name from another
/// project.
pub fn move_to_trash(path: &Path) -> Result<PathBuf, String> {
    let home = std::env::var("HOME").map_err(|_| "HOME n'est pas défini".to_string())?;
    let trash = PathBuf::from(home).join(".Trash");
    std::fs::create_dir_all(&trash).map_err(|error| error.to_string())?;

    let name = path
        .file_name()
        .ok_or_else(|| "chemin sans nom de fichier".to_string())?
        .to_string_lossy()
        .into_owned();
    let (stem, extension) = match name.rsplit_once('.') {
        Some((stem, extension)) if !stem.is_empty() => (stem.to_string(), format!(".{extension}")),
        _ => (name.clone(), String::new()),
    };

    let mut target = trash.join(&name);
    let mut index = 1;
    while target.exists() {
        target = trash.join(format!("{stem} {index}{extension}"));
        index += 1;
    }

    // Across volumes `rename` fails with `EXDEV`; the project may well sit on
    // an external disk, so fall back to a copy followed by a removal.
    if std::fs::rename(path, &target).is_ok() {
        return Ok(target);
    }
    let status = Command::new("/bin/mv")
        .arg(path)
        .arg(&target)
        .status()
        .map_err(|error| error.to_string())?;
    if status.success() {
        Ok(target)
    } else {
        Err(format!("déplacement vers la corbeille refusé : {}", path.display()))
    }
}
