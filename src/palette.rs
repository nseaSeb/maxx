//! The command palette, `⌘K`.
//!
//! The list is not written here: it is read from the menu bar. `menus::app_menus`
//! already names every command maxx has, in the user's language, next to the
//! action it dispatches — building a second list beside it would mean two places
//! to remember whenever a command is added, and the second one would be the one
//! that falls behind.
//!
//! So the palette is the menu bar, flattened, searchable, and annotated with the
//! keystroke each command answers to.

use gpui::{Action, App, Menu, MenuItem, SharedString};

/// One line of the palette.
pub struct Command {
    /// The path through the menus: `File ▸ Add to project ▸ The settings`.
    pub label: SharedString,
    /// The keystroke bound to it, when there is one.
    pub shortcut: Option<SharedString>,
    /// What running it dispatches.
    pub action: Box<dyn Action>,
}

/// Every command the menu bar offers, in menu order.
///
/// Separators, the system's own submenus and the recent-project entries are left
/// out: the first two are not commands, and the third is a list of paths that
/// changes under the palette rather than a thing to run.
pub fn commands(cx: &App) -> Vec<Command> {
    flatten(crate::menus::app_menus(cx))
}

/// The same, over a menu bar handed in.
///
/// Split from [`commands`] so the flattening — which is all the logic there is
/// here — can be exercised without an `App`, which only exists to read the
/// recent projects.
pub fn flatten(menus: Vec<Menu>) -> Vec<Command> {
    let shortcuts = shortcuts();
    let mut out = Vec::new();
    for menu in menus {
        let prefix = menu.name.clone();
        collect(&prefix, menu.items, &shortcuts, &mut out);
    }
    out
}

/// Walks one menu, prefixing each entry with the path that leads to it.
fn collect(
    prefix: &SharedString,
    items: Vec<MenuItem>,
    shortcuts: &[(&'static str, SharedString)],
    out: &mut Vec<Command>,
) {
    for item in items {
        match item {
            MenuItem::Separator | MenuItem::SystemMenu(_) => {}
            MenuItem::Submenu(Menu { name, items }) => {
                let deeper = SharedString::from(format!("{prefix} ▸ {name}"));
                collect(&deeper, items, shortcuts, out);
            }
            MenuItem::Action { name, action, .. } => {
                // `NoRecentProject` is the placeholder shown when the list is
                // empty, and it does nothing on purpose.
                if action.name() == "maxx::NoRecentProject" {
                    continue;
                }
                let shortcut = shortcuts
                    .iter()
                    .find(|(bound, _)| *bound == action.name())
                    .map(|(_, keys)| keys.clone());
                out.push(Command {
                    label: SharedString::from(format!("{prefix} ▸ {name}")),
                    shortcut,
                    action,
                });
            }
        }
    }
}

/// The keystroke bound to each action, read back from maxx's own keymap.
///
/// Read back rather than written twice: a shortcut shown in the palette that
/// does not match the keymap is worse than no shortcut at all, and the keymap is
/// the one that decides.
fn shortcuts() -> Vec<(&'static str, SharedString)> {
    crate::actions::key_bindings()
        .into_iter()
        .map(|binding| {
            let keys = binding
                .keystrokes()
                .iter()
                .map(|keystroke| keystroke.to_string())
                .collect::<Vec<_>>()
                .join(" ");
            (binding.action().name(), SharedString::from(keys))
        })
        .collect()
}

/// The commands whose label answers to `query`, in menu order.
///
/// Every word of the query has to appear somewhere in the label, in any order:
/// `add set` finds `File ▸ Add to project ▸ The settings` without asking anyone
/// to remember which menu holds what. That is the whole point of a palette.
pub fn filter(commands: Vec<Command>, query: &str) -> Vec<Command> {
    let words: Vec<String> = query.split_whitespace().map(fold).filter(|w| !w.is_empty()).collect();
    if words.is_empty() {
        return commands;
    }
    commands
        .into_iter()
        .filter(|command| {
            let label = fold(&command.label);
            words.iter().all(|word| label.contains(word.as_str()))
        })
        .collect()
}

/// Lowercase, and without the accents.
fn fold(value: &str) -> String {
    value
        .to_lowercase()
        .chars()
        .map(|character| match character {
            'á'..='å' | 'à' => 'a',
            'é' | 'è' | 'ê' | 'ë' => 'e',
            'í' | 'ì' | 'î' | 'ï' => 'i',
            'ó' | 'ò' | 'ô' | 'ö' | 'õ' => 'o',
            'ú' | 'ù' | 'û' | 'ü' => 'u',
            'ç' => 'c',
            other => other,
        })
        .collect()
}
