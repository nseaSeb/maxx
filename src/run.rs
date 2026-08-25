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
/// In the order they take precedence: `LOCALAPPDATA` on Windows,
/// `XDG_CACHE_HOME` when the user set it, and otherwise the home directory —
/// `Library/Caches` on macOS, `.cache` elsewhere. The same order
/// `settings::directory` follows, so the two never disagree about which
/// convention wins.
fn cache_dir() -> PathBuf {
    if cfg!(target_os = "windows")
        && let Ok(local) = std::env::var("LOCALAPPDATA")
        && !local.is_empty()
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

    open_terminal_bundle(terminal, path);
}

/// Opens a terminal through its macOS application bundle.
///
/// A bundle is a macOS notion and `open` a macOS tool: everywhere else the
/// command on the `PATH` is the only way in, and its absence is the reason
/// nothing happened. Written as a function with an empty counterpart rather
/// than a `cfg` block inside the caller, so the caller keeps the same shape on
/// every system — a `cfg`-ed block at the end of a function turns the line
/// above it into a tail `return`, which `clippy -D warnings` refuses.
#[cfg(target_os = "macos")]
fn open_terminal_bundle(terminal: &Terminal, path: &Path) {
    let Some(bundle) = terminal.bundle else {
        return;
    };
    // `-n` and `--args` are what carry the flag through to a bundle that has
    // no command line tool of its own.
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

#[cfg(not(target_os = "macos"))]
fn open_terminal_bundle(_terminal: &Terminal, _path: &Path) {}

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
    open_editor_bundle(editor, path);
}

/// Opens a file through the editor's macOS application bundle.
///
/// The line number is lost here: `open -a` has nowhere to put it.
#[cfg(target_os = "macos")]
fn open_editor_bundle(editor: &Editor, path: &Path) {
    if let Some(bundle) = editor.bundle {
        let _ = Command::new("open").arg("-a").arg(bundle).arg(path).spawn();
    }
}

#[cfg(not(target_os = "macos"))]
fn open_editor_bundle(_editor: &Editor, _path: &Path) {}

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

    open_terminal_editor_bundle(terminal, editor, flag, &arguments);
}

/// Starts a terminal editor through the terminal's macOS bundle.
#[cfg(target_os = "macos")]
fn open_terminal_editor_bundle(
    terminal: &Terminal,
    editor: &Editor,
    flag: &str,
    arguments: &[String],
) {
    if let Some(bundle) = terminal.bundle {
        let mut passed = vec![flag.to_string(), editor.command.to_string()];
        passed.extend(arguments.iter().cloned());
        let _ = Command::new("open")
            .arg("-na")
            .arg(bundle)
            .arg("--args")
            .args(&passed)
            .spawn();
    }
}

#[cfg(not(target_os = "macos"))]
fn open_terminal_editor_bundle(
    _terminal: &Terminal,
    _editor: &Editor,
    _flag: &str,
    _arguments: &[String],
) {
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
///
/// The freedesktop specification asks for both keys, and for the path to be
/// percent-encoded: a file named `100%.rs` would otherwise be decoded wrongly
/// and restored somewhere else, or nowhere.
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

    let body = format!(
        "[Trash Info]\nPath={}\nDeletionDate={}\n",
        percent_encode(&absolute.to_string_lossy()),
        deletion_date()
    );
    let _ = std::fs::write(
        info.join(format!("{}.trashinfo", name.to_string_lossy())),
        body,
    );
}

/// Percent-encodes a path the way the trash specification asks.
///
/// Everything outside the unreserved set is escaped, except the separator —
/// which the specification keeps readable.
fn percent_encode(path: &str) -> String {
    let mut out = String::with_capacity(path.len());
    for byte in path.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' | b'/' => {
                out.push(byte as char)
            }
            _ => out.push_str(&format!("%{byte:02X}")),
        }
    }
    out
}

/// The moment of the deletion, as the specification spells it.
///
/// In UTC rather than local time, which is what the specification asks for:
/// `std` has no timezone, and reaching for a crate to write one line in a file
/// nobody reads by hand is a poor trade. Every desktop tested reads it.
fn deletion_date() -> String {
    let seconds = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|elapsed| elapsed.as_secs())
        .unwrap_or(0);

    let days = (seconds / 86_400) as i64;
    let time = seconds % 86_400;
    let (year, month, day) = civil_from_days(days);
    format!(
        "{year:04}-{month:02}-{day:02}T{:02}:{:02}:{:02}",
        time / 3600,
        (time % 3600) / 60,
        time % 60
    )
}

/// Turns a count of days since 1970-01-01 into a civil date.
///
/// Howard Hinnant's algorithm, the one every date library uses: it shifts the
/// year to start in March so the leap day lands at the end and needs no case
/// of its own.
fn civil_from_days(days: i64) -> (i64, u32, u32) {
    let shifted = days + 719_468;
    let era = if shifted >= 0 { shifted } else { shifted - 146_096 } / 146_097;
    let day_of_era = shifted - era * 146_097;
    let year_of_era =
        (day_of_era - day_of_era / 1460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let year = year_of_era + era * 400;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let shifted_month = (5 * day_of_year + 2) / 153;
    let day = (day_of_year - (153 * shifted_month + 2) / 5 + 1) as u32;
    let month = if shifted_month < 10 {
        shifted_month + 3
    } else {
        shifted_month - 9
    } as u32;
    (if month <= 2 { year + 1 } else { year }, month, day)
}

/// Moves a file or a whole directory, for the case `rename` refuses.
///
/// In Rust rather than through `mv` or `cmd /C move`: `move` refuses to carry
/// a directory from one drive to another, which is exactly the case that
/// brings us here on Windows, and `Command::arg` escapes for rules `cmd.exe`
/// does not follow — a path holding `&`, `^` or `%` would break the command
/// line.
fn move_across_volumes(from: &Path, to: &Path) -> Result<bool, String> {
    copy_recursively(from, to).map_err(|error| error.to_string())?;
    // Only once the copy is whole: removing first would turn a failed copy
    // into a deletion.
    let removed = if from.is_dir() {
        std::fs::remove_dir_all(from)
    } else {
        std::fs::remove_file(from)
    };
    removed.map_err(|error| error.to_string())?;
    Ok(true)
}

/// Copies a file, or a directory and everything under it.
fn copy_recursively(from: &Path, to: &Path) -> std::io::Result<()> {
    if !from.is_dir() {
        std::fs::copy(from, to)?;
        return Ok(());
    }
    std::fs::create_dir_all(to)?;
    for entry in std::fs::read_dir(from)? {
        let entry = entry?;
        copy_recursively(&entry.path(), &to.join(entry.file_name()))?;
    }
    Ok(())
}

/// Runs `rustfmt` on `path`, and answers whether the file came out changed.
///
/// `rustfmt` and not `cargo fmt`: the second formats a whole crate, the first
/// takes a file — and finds the project's `rustfmt.toml` on its own by walking
/// up from it, so the developer's conventions are honoured rather than
/// replaced by maxx's.
///
/// `status` and not `spawn`, contrary to everything else maxx launches: the
/// caller has to re-read the file afterwards, so it must wait. rustfmt on one
/// file is a matter of milliseconds — a terminal, which never exits, is the
/// case that made the rule.
pub fn format_rust(path: &Path) -> Result<bool, String> {
    let before = std::fs::read_to_string(path).map_err(|error| error.to_string())?;

    let status = Command::new("rustfmt")
        .arg("--edition")
        .arg("2024")
        .arg(path)
        // Ses diagnostics sont repris dans le message rendu au-dessus ; les
        // laisser passer ferait ressembler un refus attendu à un incident.
        .stderr(Stdio::null())
        .status()
        .map_err(|error| match error.kind() {
            std::io::ErrorKind::NotFound => {
                "rustfmt est introuvable — `rustup component add rustfmt`".to_string()
            }
            _ => error.to_string(),
        })?;

    if !status.success() {
        // A file rustfmt refuses is a file that does not parse, and maxx has
        // just written it: saying so is more useful than a silent no-op.
        return Err(format!(
            "rustfmt a refusé {} — le fichier ne se lit pas comme du Rust",
            path.display()
        ));
    }

    let after = std::fs::read_to_string(path).map_err(|error| error.to_string())?;
    Ok(before != after)
}
