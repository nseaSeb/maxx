//! The `src/menus.rs` of a generated project, open in the workshop.
//!
//! Same contract as a view: the file is the truth, only the marked region is
//! rewritten, and an entry maxx does not understand is carried through
//! unchanged.

use std::path::{Path, PathBuf};

use crate::menu_model::{ItemDef, MenuDef};
use crate::parser;

/// Gathers the actions of a list of entries, submenus included.
///
/// Recursive, and it has to be: an action written inside a submenu is spliced
/// into the file like any other, so failing to see it here would leave it
/// undeclared in `actions!` and unwired — the generated project would stop
/// compiling on a name maxx itself had just written.
fn collect_actions(items: &[ItemDef], names: &mut Vec<String>) {
    for item in items {
        match item {
            ItemDef::Action { action, os_action: None, .. }
                if !action.contains("::") && !names.contains(action) =>
            {
                names.push(action.clone());
            }
            ItemDef::Submenu(inner) => collect_actions(&inner.items, names),
            _ => {}
        }
    }
}

/// The index one step away, or nothing when there is no room.
///
/// A separate function because the same arithmetic serves menus and entries,
/// and because `index - 1` on a `usize` at zero is the kind of subtraction that
/// interrupts a process.
fn step(index: usize, up: bool, length: usize) -> Option<usize> {
    if up { index.checked_sub(1) } else { Some(index + 1).filter(|target| *target < length) }
}

/// Where a menu file's editing cursor sits.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Selection {
    /// A whole menu of the bar.
    Menu(usize),
    /// One entry of a menu.
    Item(usize, usize),
    /// One entry inside a submenu: menu, submenu, entry.
    SubItem(usize, usize, usize),
}

impl Selection {
    /// The menu of the bar this selection lives in.
    pub fn menu(self) -> usize {
        match self {
            Selection::Menu(menu) | Selection::Item(menu, _) | Selection::SubItem(menu, _, _) => {
                menu
            }
        }
    }
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
        Self::from_source(path.to_path_buf(), source)
    }

    /// Parses a menu file already in hand.
    ///
    /// Split out from [`load`](Self::load) so the parsing can be exercised
    /// without a file on disk — and so a source obtained some other way can be
    /// read the same way.
    pub fn from_source(path: PathBuf, source: String) -> Result<Self, String> {
        let region = parser::locate(&source).map_err(|error| error.to_string())?;
        // Dedented first, exactly as `parser::parse` does: an opaque entry kept
        // with its file indentation gains a level on every save, because
        // `splice` re-indents the whole block on the way out.
        let dedented = parser::dedent(&source[region.start..region.end], &region.indent);
        let inner = dedented.trim();
        let expr: syn::Expr = syn::parse_str(inner).map_err(|error| error.to_string())?;
        let menus = crate::menu_model::parse(&expr, inner)
            .ok_or("la zone gérée n'est pas un « vec![Menu { .. }] »")?;

        Ok(Self { path, source, saved: menus.clone(), menus, selected: None })
    }

    /// The file name, for the tab.
    pub fn name(&self) -> String {
        self.path.file_name().map(|name| name.to_string_lossy().into_owned()).unwrap_or_default()
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
            collect_actions(&menu.items, &mut names);
        }
        names
    }

    /// Writes the menus back, declaring and wiring any new action on the way.
    pub fn save(&mut self, force: bool) -> Result<(), String> {
        if !force && self.disk_changed() {
            return Err(
                "fichier modifié en dehors de maxx — Fichier > Recharger, ou Écraser".into()
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

    /// The line where an action's handler is registered.
    ///
    /// The handlers live in `register`, not in a method of a view, so the
    /// anchor is the `cx.on_action(|_: &Nom,` the template and `ensure_action`
    /// both write.
    pub fn handler_line(&self, action: &str) -> Option<usize> {
        let needle = format!("&{action},");
        self.source.lines().position(|line| line.contains(&needle)).map(|index| index + 1)
    }

    /// The selected menu, if a menu or one of its entries is selected.
    pub fn selected_menu(&self) -> Option<&MenuDef> {
        self.menus.get(self.selected?.menu())
    }

    /// The selected entry.
    pub fn selected_item(&self) -> Option<&ItemDef> {
        match self.selected? {
            Selection::Item(menu, item) => self.menus.get(menu)?.items.get(item),
            Selection::SubItem(menu, sub, item) => self.submenu(menu, sub)?.items.get(item),
            Selection::Menu(_) => None,
        }
    }

    /// The selected entry, to change it.
    pub fn selected_item_mut(&mut self) -> Option<&mut ItemDef> {
        let (list, index) = self.list_mut()?;
        list.get_mut(index)
    }

    /// The submenu at `sub` in the menu at `menu`.
    fn submenu(&self, menu: usize, sub: usize) -> Option<&MenuDef> {
        match self.menus.get(menu)?.items.get(sub)? {
            ItemDef::Submenu(inner) => Some(inner),
            _ => None,
        }
    }

    /// The same, to change it.
    fn submenu_mut(&mut self, menu: usize, sub: usize) -> Option<&mut MenuDef> {
        match self.menus.get_mut(menu)?.items.get_mut(sub)? {
            ItemDef::Submenu(inner) => Some(inner),
            _ => None,
        }
    }

    /// The list the selection sits in, and where in it.
    ///
    /// The one place that knows the shape of a selection: everything that adds,
    /// removes or moves goes through it rather than matching the variants
    /// again, which is what kept the submenu case from being forgotten in one
    /// of them.
    fn list_mut(&mut self) -> Option<(&mut Vec<ItemDef>, usize)> {
        match self.selected? {
            Selection::Menu(_) => None,
            Selection::Item(menu, item) => Some((&mut self.menus.get_mut(menu)?.items, item)),
            Selection::SubItem(menu, sub, item) => {
                Some((&mut self.submenu_mut(menu, sub)?.items, item))
            }
        }
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
        // Un sous-menu accueille l'entrée à son tour : sélectionner un
        // sous-menu et ajouter y ajoute, plutôt qu'à côté de lui.
        if let Selection::Item(menu, index) = selection
            && matches!(
                self.menus.get(menu).and_then(|m| m.items.get(index)),
                Some(ItemDef::Submenu(_))
            )
            && let Some(inner) = self.submenu_mut(menu, index)
        {
            let at = inner.items.len();
            inner.items.push(item);
            self.selected = Some(Selection::SubItem(menu, index, at));
            return;
        }

        let at = match selection {
            Selection::Menu(_) => usize::MAX,
            Selection::Item(_, item) | Selection::SubItem(_, _, item) => item + 1,
        };
        let (list, insert_at) = match selection {
            Selection::Menu(menu) => match self.menus.get_mut(menu) {
                Some(menu) => {
                    let at = menu.items.len();
                    (&mut menu.items, at)
                }
                None => return,
            },
            Selection::Item(menu, _) => match self.menus.get_mut(menu) {
                Some(menu) => {
                    let at = at.min(menu.items.len());
                    (&mut menu.items, at)
                }
                None => return,
            },
            Selection::SubItem(menu, sub, _) => match self.submenu_mut(menu, sub) {
                Some(inner) => {
                    let at = at.min(inner.items.len());
                    (&mut inner.items, at)
                }
                None => return,
            },
        };
        list.insert(insert_at, item);
        self.selected = Some(match selection {
            Selection::Menu(menu) | Selection::Item(menu, _) => Selection::Item(menu, insert_at),
            Selection::SubItem(menu, sub, _) => Selection::SubItem(menu, sub, insert_at),
        });
    }

    /// Moves the selection one place up, or down.
    ///
    /// A menu moves among the menus, an entry among the entries of its own
    /// menu — never from one menu to another. Crossing that boundary is what a
    /// drag would be for, and a drag is a different gesture with a different
    /// affordance; two keys that only ever reorder within one list can be held
    /// down without ever surprising anyone.
    ///
    /// Answers whether anything moved, so the caller can stay quiet when the
    /// selection is already at the end of its list.
    pub fn move_selected(&mut self, up: bool) -> bool {
        let Some(selection) = self.selected else {
            return false;
        };
        match selection {
            Selection::Menu(index) => {
                let Some(target) = step(index, up, self.menus.len()) else {
                    return false;
                };
                self.menus.swap(index, target);
                self.selected = Some(Selection::Menu(target));
                true
            }
            Selection::Item(..) | Selection::SubItem(..) => {
                let Some((list, item)) = self.list_mut() else {
                    return false;
                };
                // Une sélection périmée ne doit pas interrompre le processus :
                // `remove_selected` s'en garde déjà, et `swap` hors bornes est
                // une panique là où ne rien faire suffit.
                if item >= list.len() {
                    return false;
                }
                let Some(target) = step(item, up, list.len()) else {
                    return false;
                };
                list.swap(item, target);
                self.selected = Some(match selection {
                    Selection::SubItem(menu, sub, _) => Selection::SubItem(menu, sub, target),
                    _ => Selection::Item(selection.menu(), target),
                });
                true
            }
        }
    }

    /// Removes what is selected.
    pub fn remove_selected(&mut self) {
        match self.selected {
            Some(Selection::Menu(index)) if index < self.menus.len() => {
                self.menus.remove(index);
                self.selected = None;
            }
            Some(Selection::Item(..)) | Some(Selection::SubItem(..)) => {
                if let Some((list, item)) = self.list_mut()
                    && item < list.len()
                {
                    list.remove(item);
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
