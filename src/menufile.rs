//! The `src/menus.rs` of a generated project, open in the workshop.
//!
//! Same contract as a view: the file is the truth, only the marked region is
//! rewritten, and an entry maxx does not understand is carried through
//! unchanged.

use std::path::{Path, PathBuf};

use crate::menu_model::{ItemDef, MenuDef};
use crate::parser;

/// Where a menu file's editing cursor sits.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Selection {
    /// A whole menu of the bar.
    Menu(usize),
    /// One entry of a menu.
    Item(usize, usize),
}

/// The menu bar of a project, loaded from its source.
pub struct MenuFile {
    /// Absolute path of `src/menus.rs`.
    pub path: PathBuf,
    /// Full text of the file.
    pub source: String,
    /// The menus parsed from the managed region.
    pub menus: Vec<MenuDef>,
    /// What the panel is editing.
    pub selected: Option<Selection>,
    /// The menus as they stand on disk.
    saved: Vec<MenuDef>,
}

impl MenuFile {
    /// Whether this path is a project's menu file.
    ///
    /// The basename alone would capture a view called `src/ui/menus.rs`, which
    /// could then never be opened in the designer.
    pub fn is_menu_file(path: &Path) -> bool {
        path.file_name().is_some_and(|name| name == "menus.rs")
            && path
                .parent()
                .and_then(|parent| parent.file_name())
                .is_some_and(|parent| parent == "src")
    }

    /// Reads and parses a menu file.
    pub fn load(path: &Path) -> Result<Self, String> {
        let source = std::fs::read_to_string(path).map_err(|error| error.to_string())?;
        let region = parser::locate(&source).map_err(|error| error.to_string())?;
        // Dedented first, exactly as `parser::parse` does: an opaque entry kept
        // with its file indentation gains a level on every save, because
        // `splice` re-indents the whole block on the way out.
        let dedented = parser::dedent(&source[region.start..region.end], &region.indent);
        let inner = dedented.trim();
        let expr: syn::Expr = syn::parse_str(inner).map_err(|error| error.to_string())?;
        let menus = crate::menu_model::parse(&expr, inner)
            .ok_or("la zone gérée n'est pas un « vec![Menu { .. }] »")?;

        Ok(Self {
            path: path.to_path_buf(),
            source,
            saved: menus.clone(),
            menus,
            selected: None,
        })
    }

    /// The file name, for the tab.
    pub fn name(&self) -> String {
        self.path
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_default()
    }

    /// Whether the menus differ from what is on disk.
    pub fn dirty(&self) -> bool {
        self.menus != self.saved
    }

    /// Whether the file on disk differs from the copy this was built from.
    pub fn disk_changed(&self) -> bool {
        match std::fs::read_to_string(&self.path) {
            Ok(text) => text != self.source,
            Err(_) => false,
        }
    }

    /// Re-reads the file.
    pub fn reload(&mut self) -> Result<(), String> {
        let reloaded = MenuFile::load(&self.path)?;
        self.source = reloaded.source;
        self.menus = reloaded.menus;
        self.saved = reloaded.saved;
        self.selected = None;
        Ok(())
    }

    /// The actions maxx must declare and wire.
    ///
    /// An `os_action` entry is handed to the system — registering a handler of
    /// our own for `Copy` or `Undo` would shadow the very behaviour it exists to
    /// delegate. A qualified path belongs to another module, so it is not ours
    /// to declare either.
    pub fn actions(&self) -> Vec<String> {
        let mut names = Vec::new();
        for menu in &self.menus {
            for item in &menu.items {
                if let ItemDef::Action {
                    action,
                    os_action: None,
                    ..
                } = item
                    && !action.contains("::")
                    && !names.contains(action)
                {
                    names.push(action.clone());
                }
            }
        }
        names
    }

    /// Writes the menus back, declaring and wiring any new action on the way.
    pub fn save(&mut self, force: bool) -> Result<(), String> {
        if !force && self.disk_changed() {
            return Err(
                "fichier modifié en dehors de maxx — Fichier > Recharger, ou Écraser".into(),
            );
        }
        if !self.dirty() && !force {
            // Nothing to write: rewriting would still add handlers and churn
            // the file for a plain ⌘S on an untouched menu bar.
            return Ok(());
        }

        let block = crate::menu_model::render(&self.menus);
        let mut source = parser::splice(&self.source, &block).map_err(|e| e.to_string())?;
        for action in self.actions() {
            source = ensure_action(source, &action);
        }

        std::fs::write(&self.path, &source).map_err(|error| error.to_string())?;
        self.source = source;
        self.saved = self.menus.clone();
        Ok(())
    }

    /// The selected menu, if a menu or one of its entries is selected.
    pub fn selected_menu(&self) -> Option<&MenuDef> {
        match self.selected? {
            Selection::Menu(index) | Selection::Item(index, _) => self.menus.get(index),
        }
    }

    /// The selected entry.
    pub fn selected_item(&self) -> Option<&ItemDef> {
        let Selection::Item(menu, item) = self.selected? else {
            return None;
        };
        self.menus.get(menu)?.items.get(item)
    }

    /// Adds a menu at the end of the bar.
    pub fn add_menu(&mut self) {
        self.menus.push(MenuDef::named("Menu"));
        self.selected = Some(Selection::Menu(self.menus.len() - 1));
    }

    /// Adds an entry after the selection, or at the end of the selected menu.
    pub fn add_item(&mut self, item: ItemDef) {
        let Some(selection) = self.selected else {
            return;
        };
        let (menu_index, at) = match selection {
            Selection::Menu(menu) => (menu, self.menus[menu].items.len()),
            Selection::Item(menu, item) => (menu, item + 1),
        };
        let Some(menu) = self.menus.get_mut(menu_index) else {
            return;
        };
        let at = at.min(menu.items.len());
        menu.items.insert(at, item);
        self.selected = Some(Selection::Item(menu_index, at));
    }

    /// Removes what is selected.
    pub fn remove_selected(&mut self) {
        match self.selected {
            Some(Selection::Menu(index)) if index < self.menus.len() => {
                self.menus.remove(index);
                self.selected = None;
            }
            Some(Selection::Item(menu, item)) => {
                if let Some(menu) = self.menus.get_mut(menu)
                    && item < menu.items.len()
                {
                    menu.items.remove(item);
                }
                self.selected = None;
            }
            _ => {}
        }
    }
}

/// Makes sure an action is declared in `actions!` and has a handler.
///
/// Both insertions are textual, on anchors the template puts there, and an
/// action already present is left exactly as it is.
fn ensure_action(source: String, name: &str) -> String {
    let mut source = source;

    if let Some(start) = source.find("actions!(")
        && let Some(open) = source[start..].find('[').map(|index| start + index)
        && let Some(close) = source[open..].find(']').map(|index| open + index)
        && !declared(&source[open + 1..close], name)
    {
        let list = source[open + 1..close].trim_end().trim_end_matches(',');
        let separator = if list.trim().is_empty() { "" } else { ", " };
        let replacement = format!("{list}{separator}{name}");
        source.replace_range(open + 1..close, &replacement);
    }

    // A handler for an action nobody registered leaves the menu item greyed
    // out, which reads as a bug rather than as work left to do.
    if !source.contains(&format!("&{name},")) && !source.contains(&format!("&{name}, cx")) {
        let anchor = "    // maxx:handlers\n";
        if let Some(offset) = source.find(anchor) {
            let handler = format!(
                "    cx.on_action(|_: &{name}, _cx: &mut App| {{\n\
                 \x20       // Écrit par maxx ; à toi de le remplir.\n\
                 \x20   }});\n"
            );
            source.insert_str(offset, &handler);
        }
    }

    source
}

/// Whether `list` — the inside of `actions!(app, [ .. ])` — already names it.
fn declared(list: &str, name: &str) -> bool {
    list.split(',').any(|entry| entry.trim() == name)
}
