//! The editors and terminals maxx can hand a file to.
//!
//! Detection is the easy half. The hard half is that opening a file *at a
//! line* has a different spelling in every editor, and that some editors are
//! not applications at all but programs that need a terminal around them — so
//! the two settings are not independent.
//!
//! The catalogue is a table on purpose. A heuristic would be wrong for every
//! editor that does not follow the majority, and there is no majority.

use std::collections::HashSet;
use std::ffi::{OsStr, OsString};
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use gpui::App;

use rust_i18n::t;

/// The value that means "whatever is installed", the default.
pub const AUTOMATIC: &str = "auto";

/// How an editor is told which line to open on.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum LineArgument {
    /// `zed fichier:12`, `subl fichier:12`.
    Suffix,
    /// `code -g fichier:12`.
    Flag(&'static str),
    /// `nvim +12 fichier`.
    PlusLine,
    /// `idea --line 12 fichier`.
    Named(&'static str),
}

/// One editor maxx knows how to drive.
#[derive(Clone, Copy, Debug)]
pub struct Editor {
    /// Stable key, what the settings file holds.
    pub id: &'static str,
    /// What the preferences and the menu bar show.
    pub label: &'static str,
    /// The command line tool, when there is one.
    pub command: &'static str,
    /// The macOS application bundle, without `.app`.
    pub bundle: Option<&'static str>,
    /// How it is told a line number.
    pub line: LineArgument,
    /// Whether it draws inside a terminal rather than a window of its own.
    pub terminal_bound: bool,
}

/// One terminal maxx knows how to open.
#[derive(Clone, Copy, Debug)]
pub struct Terminal {
    /// Stable key, what the settings file holds.
    pub id: &'static str,
    /// What the preferences show.
    pub label: &'static str,
    /// The command line tool, when there is one.
    pub command: &'static str,
    /// The macOS application bundle, without `.app`.
    pub bundle: Option<&'static str>,
    /// The flag that points it at a directory, if it takes one.
    pub directory_flag: Option<&'static str>,
    /// The flag that hands it a command to run, if it takes one.
    ///
    /// `None` means maxx cannot start a terminal editor inside it — which is
    /// the case of Terminal.app, whose only way in is AppleScript, and that
    /// asks for an automation permission in the middle of a click.
    pub command_flag: Option<&'static str>,
}

/// Every editor maxx knows, most preferred first.
///
/// The order is what "automatic" follows, and Zed leads it because maxx writes
/// projects that open in Zed.
pub const EDITORS: &[Editor] = &[
    Editor {
        id: "zed",
        label: "Zed",
        command: "zed",
        bundle: Some("Zed"),
        line: LineArgument::Suffix,
        terminal_bound: false,
    },
    Editor {
        id: "code",
        label: "Visual Studio Code",
        command: "code",
        bundle: Some("Visual Studio Code"),
        line: LineArgument::Flag("-g"),
        terminal_bound: false,
    },
    Editor {
        id: "cursor",
        label: "Cursor",
        command: "cursor",
        bundle: Some("Cursor"),
        line: LineArgument::Flag("-g"),
        terminal_bound: false,
    },
    Editor {
        id: "subl",
        label: "Sublime Text",
        command: "subl",
        bundle: Some("Sublime Text"),
        line: LineArgument::Suffix,
        terminal_bound: false,
    },
    Editor {
        id: "rustrover",
        label: "RustRover",
        command: "rustrover",
        bundle: Some("RustRover"),
        line: LineArgument::Named("--line"),
        terminal_bound: false,
    },
    Editor {
        id: "hx",
        label: "Helix",
        command: "hx",
        bundle: None,
        line: LineArgument::Suffix,
        terminal_bound: true,
    },
    Editor {
        id: "nvim",
        label: "Neovim",
        command: "nvim",
        bundle: None,
        line: LineArgument::PlusLine,
        terminal_bound: true,
    },
    Editor {
        id: "vim",
        label: "Vim",
        command: "vim",
        bundle: None,
        line: LineArgument::PlusLine,
        terminal_bound: true,
    },
];

/// Every terminal maxx knows, most preferred first.
pub const TERMINALS: &[Terminal] = &[
    Terminal {
        id: "ghostty",
        label: "Ghostty",
        command: "ghostty",
        bundle: Some("Ghostty"),
        directory_flag: Some("--working-directory"),
        command_flag: Some("-e"),
    },
    Terminal {
        id: "wezterm",
        label: "WezTerm",
        command: "wezterm",
        bundle: Some("WezTerm"),
        directory_flag: Some("--cwd"),
        command_flag: Some("-e"),
    },
    Terminal {
        id: "kitty",
        label: "kitty",
        command: "kitty",
        bundle: Some("kitty"),
        directory_flag: Some("--directory"),
        command_flag: None,
    },
    Terminal {
        id: "alacritty",
        label: "Alacritty",
        command: "alacritty",
        bundle: Some("Alacritty"),
        directory_flag: Some("--working-directory"),
        command_flag: Some("-e"),
    },
    Terminal {
        id: "iterm",
        label: "iTerm",
        command: "",
        bundle: Some("iTerm"),
        directory_flag: None,
        command_flag: None,
    },
    Terminal {
        id: "terminal",
        label: "Terminal",
        command: "",
        bundle: Some("Terminal"),
        directory_flag: None,
        command_flag: None,
    },
];

static SEARCH_PATH: OnceLock<OsString> = OnceLock::new();
static KNOWN_PATH: OnceLock<OsString> = OnceLock::new();

/// The `PATH` maxx hands to every command it starts.
///
/// A maxx started from its icon inherits launchd's environment, whose `PATH`
/// is `/usr/bin:/bin:/usr/sbin:/sbin` — where neither `cargo` nor `zed` lives.
/// The Run button answered `cargo run: No such file or directory`, and the
/// editor could only be reached through its application bundle, losing the
/// line number. So the login shell is asked once for the `PATH` it would give
/// a terminal, and the usual directories are added behind its answer, so the
/// result is still right when there is no shell to ask.
///
/// Reading it waits for that shell, which is why `lib::run` asks for it on a
/// thread of its own as maxx starts: by the first click it is already there.
pub fn search_path() -> &'static OsString {
    SEARCH_PATH.get_or_init(build_search_path)
}

/// The `PATH` to use without waiting: the full one once the shell has answered,
/// and the immediate one until then.
///
/// `on_path` is called while the menu bar is being built and on every repaint
/// of the preferences, and the commands maxx starts from a click — the editor,
/// the terminal, `rustfmt` on ⌘S — are started on the interface thread. None of
/// them may wait seconds for a shell. What is lost in that first moment is a
/// tool installed *only* in a directory a version manager adds; the run itself,
/// which happens on a thread of its own, does wait for the complete answer.
pub fn lookup_path() -> &'static OsString {
    SEARCH_PATH.get().unwrap_or_else(|| KNOWN_PATH.get_or_init(build_known_path))
}

/// What is known without asking anyone: the inherited `PATH` and the usual
/// directories.
fn build_known_path() -> OsString {
    join_unique(&[std::env::var_os("PATH").unwrap_or_default()])
}

/// The inherited `PATH`, then the login shell's, then the usual directories.
///
/// In that order, and without duplicates: what the process was given wins over
/// what a shell says, and a guess comes last.
fn build_search_path() -> OsString {
    let inherited = std::env::var_os("PATH").unwrap_or_default();
    match login_shell_path() {
        Some(shell) => join_unique(&[inherited, shell]),
        None => join_unique(&[inherited]),
    }
}

/// Those sources, then the usual directories, each directory kept once and in
/// the order it was first seen.
fn join_unique(sources: &[OsString]) -> OsString {
    let mut directories: Vec<PathBuf> = Vec::new();
    let mut seen: HashSet<PathBuf> = HashSet::new();
    let mut add = |value: &OsStr| {
        for directory in std::env::split_paths(value) {
            if !directory.as_os_str().is_empty() && seen.insert(directory.clone()) {
                directories.push(directory);
            }
        }
    };

    for source in sources {
        add(source);
    }
    for directory in usual_directories() {
        add(directory.as_os_str());
    }

    // `join_paths` refuses a directory containing the separator; what was
    // inherited is then the only honest answer, rather than a truncated list.
    std::env::join_paths(&directories)
        .unwrap_or_else(|_| sources.first().cloned().unwrap_or_default())
}

/// Where a tool installed by a package manager or an installer usually lands.
///
/// The fallback for a maxx started with no shell to ask: `rustup` writes
/// `~/.cargo/bin`, Homebrew `/opt/homebrew/bin` on Apple silicon and
/// `/usr/local/bin` on Intel — which is also where Zed puts its `zed`.
fn usual_directories() -> Vec<PathBuf> {
    let mut directories = Vec::new();
    if let Some(home) = home_directory() {
        directories.push(home.join(".cargo/bin"));
        directories.push(home.join(".local/bin"));
    }
    if cfg!(windows) {
        return directories;
    }
    directories.extend(
        ["/opt/homebrew/bin", "/usr/local/bin", "/opt/local/bin", "/usr/bin", "/bin"]
            .iter()
            .map(PathBuf::from),
    );
    directories
}

fn home_directory() -> Option<PathBuf> {
    let name = if cfg!(windows) { "USERPROFILE" } else { "HOME" };
    std::env::var_os(name).filter(|home| !home.is_empty()).map(PathBuf::from)
}

/// The `PATH` the user's login shell would give a terminal.
///
/// `-l -i` because the directory that holds `cargo` is as often written in an
/// interactive file (`.zshrc`) as in a login one, and a version manager writes
/// it nowhere else. Waited on for ten seconds and no longer: a shell that hangs
/// on a prompt must not hold maxx, and the usual directories are the answer in
/// that case. Ten and not one because a `.zshrc` that starts mise, asdf and
/// conda was measured between 1.6 s warm and 4.1 s cold — the wait itself costs
/// nothing, it is paid on a thread while maxx opens.
#[cfg(unix)]
fn login_shell_path() -> Option<OsString> {
    let shell = std::env::var("SHELL").ok().filter(|shell| !shell.is_empty())?;

    let (sender, receiver) = std::sync::mpsc::channel();
    let (pid_sender, pids) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let mut command = std::process::Command::new(shell);
        let command = command
            // Between two unit separators: an interactive configuration that
            // greets the user writes on the same stream, and what is wanted is
            // what lies between the markers, not the whole output.
            .args(["-l", "-i", "-c", "printf '\\037%s\\037' \"$PATH\""])
            // A configuration that asks the user something must not wait for
            // an answer, and its own noise does not belong in maxx's streams.
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::null());
        let Ok(child) = command.spawn() else {
            let _ = sender.send(None);
            return;
        };
        let _ = pid_sender.send(child.id());
        let _ = sender.send(child.wait_with_output().ok());
    });

    let output = match receiver.recv_timeout(std::time::Duration::from_secs(10)) {
        Ok(output) => output?,
        // A configuration that hangs would otherwise leave a shell of its own
        // behind for as long as maxx is open.
        Err(_) => {
            if let Ok(pid) = pids.try_recv() {
                let _ =
                    std::process::Command::new("kill").arg("-TERM").arg(pid.to_string()).status();
            }
            return None;
        }
    };
    if !output.status.success() {
        return None;
    }
    let printed = String::from_utf8(output.stdout).ok()?;
    let mut parts = printed.split('\u{1f}');
    parts.next()?;
    let path = parts.next()?.trim();
    if path.is_empty() { None } else { Some(OsString::from(path)) }
}

/// Windows has no login shell to ask: the usual directories are all there is.
#[cfg(not(unix))]
fn login_shell_path() -> Option<OsString> {
    None
}

/// Whether `command` is on the `PATH`.
///
/// Walked by hand rather than shelled out to `which`: spawning a process to
/// ask a question about processes is a poor trade, and this runs once per
/// candidate when the preferences are drawn.
pub fn on_path(command: &str) -> bool {
    if command.is_empty() {
        return false;
    }
    let path = lookup_path();

    // On Windows the file does not carry the command's name: `code` is
    // `code.cmd`, `nvim` is `nvim.exe`. Looking for the bare name never finds
    // anything there, and everything looks missing.
    let extensions: Vec<String> = if cfg!(target_os = "windows") {
        let list = std::env::var("PATHEXT").unwrap_or_else(|_| ".COM;.EXE;.BAT;.CMD".into());
        std::iter::once(String::new())
            .chain(
                list.split(';')
                    .filter(|part| !part.is_empty())
                    .map(|part| part.to_ascii_lowercase()),
            )
            .collect()
    } else {
        vec![String::new()]
    };

    std::env::split_paths(path).any(|directory| {
        extensions.iter().any(|extension| directory.join(format!("{command}{extension}")).is_file())
    })
}

/// Whether a macOS application bundle of that name is installed.
fn bundle_installed(bundle: &str) -> bool {
    if !cfg!(target_os = "macos") {
        return false;
    }
    ["/Applications", "/System/Applications", "/System/Applications/Utilities"]
        .iter()
        .any(|directory| Path::new(directory).join(format!("{bundle}.app")).is_dir())
        || std::env::var("HOME").is_ok_and(|home| {
            Path::new(&home).join("Applications").join(format!("{bundle}.app")).is_dir()
        })
}

impl Editor {
    /// Whether this editor is installed.
    pub fn installed(&self) -> bool {
        on_path(self.command) || self.bundle.is_some_and(bundle_installed)
    }
}

impl Terminal {
    /// Whether this terminal is installed.
    pub fn installed(&self) -> bool {
        on_path(self.command) || self.bundle.is_some_and(bundle_installed)
    }
}

/// The editors found on this machine.
pub fn installed_editors() -> Vec<&'static Editor> {
    EDITORS.iter().filter(|editor| editor.installed()).collect()
}

/// The terminals found on this machine.
pub fn installed_terminals() -> Vec<&'static Terminal> {
    TERMINALS.iter().filter(|terminal| terminal.installed()).collect()
}

/// The editor to use: the one chosen, or the first installed.
///
/// `$VISUAL` and `$EDITOR` are consulted before the catalogue when nothing is
/// chosen: someone who set them has already said what they want, and it costs
/// a lookup to honour it.
pub fn editor(cx: &App) -> Option<&'static Editor> {
    let chosen = crate::settings::prefs(cx).editor.clone();
    if chosen != AUTOMATIC
        && let Some(editor) =
            EDITORS.iter().find(|editor| editor.id == chosen && editor.installed())
    {
        return Some(editor);
    }

    let from_environment =
        ["VISUAL", "EDITOR"].iter().filter_map(|name| std::env::var(name).ok()).find_map(|value| {
            let command = value.split_whitespace().next()?.to_string();
            let name = Path::new(&command).file_name()?.to_string_lossy().into_owned();
            EDITORS.iter().find(|editor| editor.command == name)
        });
    from_environment.or_else(|| EDITORS.iter().find(|editor| editor.installed()))
}

/// The terminal to use: the one chosen, or the first installed.
pub fn terminal(cx: &App) -> Option<&'static Terminal> {
    let chosen = crate::settings::prefs(cx).terminal.clone();
    // An editor or a terminal chosen on another machine, or since uninstalled,
    // falls back rather than running a command that is not there.
    if chosen != AUTOMATIC
        && let Some(terminal) =
            TERMINALS.iter().find(|terminal| terminal.id == chosen && terminal.installed())
    {
        return Some(terminal);
    }
    TERMINALS.iter().find(|terminal| terminal.installed())
}

/// What the menu bar and the inspector call the chosen editor.
pub fn editor_label(cx: &App) -> String {
    editor(cx)
        .map(|editor| editor.label.to_string())
        .unwrap_or_else(|| crate::tr("tools.the_editor").to_string())
}

/// Opens `path` in the chosen editor, at `line` when there is one, and answers
/// whether anything opened.
///
/// `false` covers the three ways this ends in nothing: no editor found at all,
/// a terminal editor with no terminal able to hold it, and a command that did
/// not start. The caller says so in the window — the alternative is a menu item
/// that looks broken.
pub fn open_in_editor(cx: &App, path: &Path, line: Option<usize>) -> bool {
    let Some(editor) = editor(cx) else {
        return false;
    };
    if editor.terminal_bound {
        crate::run::open_editor_in_terminal(editor, terminal(cx), path, line)
    } else {
        crate::run::open_editor(editor, path, line)
    }
}

/// Opens the chosen terminal at `path`.
pub fn open_terminal(cx: &App, path: &Path) {
    crate::run::open_terminal(terminal(cx), path);
}

/// The dropdown entries for the preferences: `(value, label)`.
///
/// An editor that is not installed is left out rather than shown greyed: the
/// list is short, and a choice that cannot work is noise.
pub fn editor_options() -> Vec<(String, String)> {
    let mut options = vec![(AUTOMATIC.to_string(), automatic_editor_label())];
    options.extend(
        installed_editors()
            .into_iter()
            .map(|editor| (editor.id.to_string(), editor.label.to_string())),
    );
    options
}

/// The dropdown entries for the terminals.
pub fn terminal_options() -> Vec<(String, String)> {
    let mut options = vec![(AUTOMATIC.to_string(), automatic_terminal_label())];
    options.extend(
        installed_terminals()
            .into_iter()
            .map(|terminal| (terminal.id.to_string(), terminal.label.to_string())),
    );
    options
}

/// "Automatic" plus what it currently resolves to, so the choice is informed.
fn automatic_editor_label() -> String {
    match EDITORS.iter().find(|editor| editor.installed()) {
        Some(editor) => t!("tools.automatic", tool = editor.label).into_owned(),
        None => crate::tr("tools.automatic_none").to_string(),
    }
}

/// Same, for terminals.
fn automatic_terminal_label() -> String {
    match TERMINALS.iter().find(|terminal| terminal.installed()) {
        Some(terminal) => t!("tools.automatic", tool = terminal.label).into_owned(),
        None => crate::tr("tools.automatic_none").to_string(),
    }
}
