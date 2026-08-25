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
    cache_dir().join("maxx/target")
}

/// Where this system puts caches.
///
/// The three conventions, in the order they take precedence:
/// `XDG_CACHE_HOME` when the user set it, `LOCALAPPDATA` on Windows, and
/// otherwise the home directory — `Library/Caches` on macOS, `.cache`
/// elsewhere.
fn cache_dir() -> PathBuf {
    if cfg!(target_os = "windows")
        && let Ok(local) = std::env::var("LOCALAPPDATA")
    {
        return PathBuf::from(local);
    }
    if let Ok(xdg) = std::env::var("XDG_CACHE_HOME")
        && !xdg.is_empty()
    {
        return PathBuf::from(xdg);
    }
    let Ok(home) = std::env::var("HOME") else {
        return std::env::temp_dir();
    };
    let home = PathBuf::from(home);
    if cfg!(target_os = "macos") {
        home.join("Library/Caches")
    } else {
        home.join(".cache")
    }
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

    // The bundle is a macOS notion, and `open` is a macOS tool: elsewhere the
    // command on the `PATH` is the only way in, and its absence is why nothing
    // happened.
    #[cfg(target_os = "macos")]
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
    #[cfg(target_os = "macos")]
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

    #[cfg(target_os = "macos")]
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
#[cfg(unix)]
pub fn stop(pid: u32) {
    let _ = Command::new("kill")
        .arg("-TERM")
        .arg(format!("-{pid}"))
        .status();
    let _ = Command::new("kill").arg("-TERM").arg(pid.to_string()).status();
}

/// Kills a run by its process id, and the tree under it.
///
/// Windows has no process groups to signal, so the tree is walked by
/// `taskkill /T`, which is the equivalent gesture: `/F` because cargo's child
/// is a window that will not close on a polite request.
#[cfg(windows)]
pub fn stop(pid: u32) {
    let _ = Command::new("taskkill")
        .arg("/PID")
        .arg(pid.to_string())
        .arg("/T")
        .arg("/F")
        .status();
}

fn run(root: PathBuf, subcommand: &str, sender: Sender<Message>) {
    let mut command = Command::new("cargo");
    let child = command
        .arg(subcommand)
        .current_dir(&root)
        // Colour codes would end up in the panel as escape sequences.
        .env("CARGO_TERM_COLOR", "never")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    // Without a group of its own the child stays in maxx's, and signalling
    // `-pid` reaches a group that does not exist — or someone else's. Windows
    // has no such thing; `stop` walks the tree with `taskkill /T` instead.
    #[cfg(unix)]
    let child = {
        use std::os::unix::process::CommandExt as _;
        child.process_group(0)
    };

    let child = child
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

/// Moves `path` to the system's trash and answers where it landed.
///
/// Never an erase: a wrong click must cost a trip to the file manager, not the
/// afternoon. Done by moving the file rather than by asking the desktop
/// environment, which on macOS means scripting the Finder — an automation
/// permission prompt in the middle of a delete is worse than losing the "Put
/// Back" entry.
///
/// Each system keeps its trash somewhere else, and Linux keeps a record beside
/// it: [`trash_dir`] and [`write_trashinfo`] hold what differs.
pub fn move_to_trash(path: &Path) -> Result<PathBuf, String> {
    let trash = trash_dir()?;
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

    // The trash may already hold a file of that name, from this project or
    // another.
    let mut target = trash.join(&name);
    let mut index = 1;
    while target.exists() {
        target = trash.join(format!("{stem} {index}{extension}"));
        index += 1;
    }

    // Across volumes `rename` fails with `EXDEV`; the project may well sit on
    // an external disk, so fall back to a move that copies.
    if std::fs::rename(path, &target).is_err() {
        let moved = move_across_volumes(path, &target)?;
        if !moved {
            return Err(format!(
                "déplacement vers la corbeille refusé : {}",
                path.display()
            ));
        }
    }

    write_trashinfo(&target, path);
    Ok(target)
}

/// The directory this system's trash keeps its files in.
fn trash_dir() -> Result<PathBuf, String> {
    if cfg!(target_os = "macos") {
        let home = std::env::var("HOME").map_err(|_| "HOME n'est pas défini".to_string())?;
        return Ok(PathBuf::from(home).join(".Trash"));
    }

    if cfg!(target_os = "windows") {
        // The real Recycle Bin is only reachable through the shell API, which
        // would cost a dependency and a `unsafe` block for a gesture that has
        // to stay simple. maxx keeps its own, in the place Windows puts
        // application data, and says so.
        let local = std::env::var("LOCALAPPDATA")
            .map_err(|_| "LOCALAPPDATA n'est pas défini".to_string())?;
        return Ok(PathBuf::from(local).join("maxx/corbeille"));
    }

    // The freedesktop.org specification: `$XDG_DATA_HOME/Trash/files`, with a
    // record of where each file came from in `../info`.
    let data = match std::env::var("XDG_DATA_HOME") {
        Ok(data) if !data.is_empty() => PathBuf::from(data),
        _ => {
            let home = std::env::var("HOME").map_err(|_| "HOME n'est pas défini".to_string())?;
            PathBuf::from(home).join(".local/share")
        }
    };
    Ok(data.join("Trash/files"))
}

/// Writes the record a Linux desktop needs to offer "Restore".
///
/// Without it the file is in the trash but the desktop does not know where it
/// came from, and the entry cannot be put back. Elsewhere there is nothing to
/// write.
fn write_trashinfo(target: &Path, original: &Path) {
    if cfg!(target_os = "macos") || cfg!(target_os = "windows") {
        return;
    }
    let Some(files) = target.parent() else {
        return;
    };
    let Some(trash) = files.parent() else {
        return;
    };
    let Some(name) = target.file_name() else {
        return;
    };

    let info = trash.join("info");
    if std::fs::create_dir_all(&info).is_err() {
        return;
    }
    let absolute = std::fs::canonicalize(original.parent().unwrap_or(original))
        .map(|parent| match original.file_name() {
            Some(name) => parent.join(name),
            None => parent,
        })
        .unwrap_or_else(|_| original.to_path_buf());

    // The date is the one part maxx cannot fill honestly without a clock it
    // does not have here; the specification allows it to be approximate, and
    // every desktop tolerates it missing.
    let body = format!(
        "[Trash Info]\nPath={}\n",
        absolute.to_string_lossy()
    );
    let _ = std::fs::write(
        info.join(format!("{}.trashinfo", name.to_string_lossy())),
        body,
    );
}

/// Moves a file the way the system moves files, for the case `rename` refuses.
fn move_across_volumes(from: &Path, to: &Path) -> Result<bool, String> {
    let mut command = if cfg!(target_os = "windows") {
        let mut command = Command::new("cmd");
        command.arg("/C").arg("move").arg("/Y").arg(from).arg(to);
        command
    } else {
        let mut command = Command::new("/bin/mv");
        command.arg(from).arg(to);
        command
    };
    command
        .status()
        .map(|status| status.success())
        .map_err(|error| error.to_string())
}
