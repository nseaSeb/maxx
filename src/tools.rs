//! The editors and terminals maxx can hand a file to.
//!
//! Detection is the easy half. The hard half is that opening a file *at a
//! line* has a different spelling in every editor, and that some editors are
//! not applications at all but programs that need a terminal around them — so
//! the two settings are not independent.
//!
//! The catalogue is a table on purpose. A heuristic would be wrong for every
//! editor that does not follow the majority, and there is no majority.

use std::path::Path;

use gpui::App;

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

/// Whether `command` is on the `PATH`.
///
/// Walked by hand rather than shelled out to `which`: spawning a process to
/// ask a question about processes is a poor trade, and this runs once per
/// candidate when the preferences are drawn.
pub fn on_path(command: &str) -> bool {
    if command.is_empty() {
        return false;
    }
    let Ok(path) = std::env::var("PATH") else {
        return false;
    };

    // Sur Windows le fichier ne porte pas le nom de la commande : `code` est
    // `code.cmd`, `nvim` est `nvim.exe`. Chercher le nom nu n'y trouve jamais
    // rien, et tout paraît absent.
    let extensions: Vec<String> = if cfg!(target_os = "windows") {
        let list = std::env::var("PATHEXT").unwrap_or_else(|_| ".COM;.EXE;.BAT;.CMD".into());
        std::iter::once(String::new())
            .chain(list.split(';').filter(|part| !part.is_empty()).map(|part| {
                part.to_ascii_lowercase()
            }))
            .collect()
    } else {
        vec![String::new()]
    };

    std::env::split_paths(&path).any(|directory| {
        extensions
            .iter()
            .any(|extension| directory.join(format!("{command}{extension}")).is_file())
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
            Path::new(&home)
                .join("Applications")
                .join(format!("{bundle}.app"))
                .is_dir()
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
    TERMINALS
        .iter()
        .filter(|terminal| terminal.installed())
        .collect()
}

/// The editor to use: the one chosen, or the first installed.
///
/// `$VISUAL` and `$EDITOR` are consulted before the catalogue when nothing is
/// chosen: someone who set them has already said what they want, and it costs
/// a lookup to honour it.
pub fn editor(cx: &App) -> Option<&'static Editor> {
    let chosen = crate::settings::prefs(cx).editor.clone();
    if chosen != AUTOMATIC
        && let Some(editor) = EDITORS
            .iter()
            .find(|editor| editor.id == chosen && editor.installed())
    {
        return Some(editor);
    }

    let from_environment = ["VISUAL", "EDITOR"]
        .iter()
        .filter_map(|name| std::env::var(name).ok())
        .find_map(|value| {
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
        && let Some(terminal) = TERMINALS
            .iter()
            .find(|terminal| terminal.id == chosen && terminal.installed())
    {
        return Some(terminal);
    }
    TERMINALS.iter().find(|terminal| terminal.installed())
}

/// What the menu bar and the inspector call the chosen editor.
pub fn editor_label(cx: &App) -> String {
    editor(cx)
        .map(|editor| editor.label.to_string())
        .unwrap_or_else(|| "l'éditeur".into())
}

/// Opens `path` in the chosen editor, at `line` when there is one.
pub fn open_in_editor(cx: &App, path: &Path, line: Option<usize>) {
    let Some(editor) = editor(cx) else {
        return;
    };
    if editor.terminal_bound {
        crate::run::open_editor_in_terminal(editor, terminal(cx), path, line);
    } else {
        crate::run::open_editor(editor, path, line);
    }
}

/// Opens the chosen terminal at `path`.
pub fn open_terminal(cx: &App, path: &Path) {
    crate::run::open_terminal(terminal(cx), path);
}

/// The dropdown entries for the preferences: `(valeur, libellé)`.
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

/// "Automatique" plus what it currently resolves to, so the choice is informed.
fn automatic_editor_label() -> String {
    match EDITORS.iter().find(|editor| editor.installed()) {
        Some(editor) => format!("Automatique ({})", editor.label),
        None => "Automatique (aucun trouvé)".into(),
    }
}

/// Same, for terminals.
fn automatic_terminal_label() -> String {
    match TERMINALS.iter().find(|terminal| terminal.installed()) {
        Some(terminal) => format!("Automatique ({})", terminal.label),
        None => "Automatique (aucun trouvé)".into(),
    }
}
