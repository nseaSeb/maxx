//! The menu-bar editor: its panel's boxes, and every edit to `src/menus.rs`.

use super::*;

impl Workspace {
    /// Refuses to drop menu edits that have not been written.
    ///
    /// Returns `true` when the caller must stop.
    pub(super) fn discard_menu_edits(&mut self, cx: &mut Context<Self>) -> bool {
        if self.menu_file().is_some_and(|menus| menus.dirty()) {
            self.message = Some(crate::tr("message.menus_unsaved"));
            cx.notify();
            return true;
        }
        false
    }

    /// The text box bound to a field of the menu panel.
    pub(crate) fn menu_input(&self, field: MenuField) -> Option<&Entity<InputState>> {
        self.menu_inputs.iter().find(|(candidate, _)| *candidate == field).map(|(_, state)| state)
    }

    /// Rebuilds the menu panel's boxes when its selection changes.
    pub(super) fn sync_menu_inputs(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let key = self.menu_file().map(|menus| menus.selected);
        if key == self.menu_synced {
            return;
        }
        self.menu_synced = key;
        self.menu_inputs.clear();

        let Some(menus) = self.menu_file() else {
            return;
        };
        let mut fields = Vec::new();
        match menus.selected {
            Some(Selection::Menu(_)) => {
                if let Some(menu) = menus.selected_menu() {
                    fields.push((MenuField::Name, menu.name.clone()));
                }
            }
            Some(Selection::Item(..)) | Some(Selection::SubItem(..)) => {
                match menus.selected_item() {
                    Some(ItemDef::Action { label, action, os_action, shortcut }) => {
                        fields.push((MenuField::Label, label.clone()));
                        fields.push((MenuField::Action, action.clone()));
                        // A system action carries the shortcut the system gives
                        // it: offering another one would be lying.
                        if os_action.is_none() {
                            fields
                                .push((MenuField::Shortcut, shortcut.clone().unwrap_or_default()));
                        }
                    }
                    // A submenu carries a title, under the same Label field the
                    // inspector shows.
                    Some(ItemDef::Submenu(inner)) => {
                        fields.push((MenuField::Label, inner.name.clone()));
                    }
                    _ => {}
                }
            }
            None => {}
        }

        for (field, value) in fields {
            let state = cx.new(|cx| InputState::new(window, cx).default_value(value));
            cx.subscribe(&state, move |this, state, event: &InputEvent, cx| {
                if matches!(event, InputEvent::Change) {
                    let value = state.read(cx).value().to_string();
                    this.edit_menu_field(field, &value, cx);
                }
            })
            .detach();
            self.menu_inputs.push((field, state));
        }
    }

    /// Writes one field of the menu panel.
    fn edit_menu_field(&mut self, field: MenuField, value: &str, cx: &mut Context<Self>) {
        let Some(menus) = self.menu_file_mut() else {
            return;
        };
        let Some(selection) = menus.selected else {
            return;
        };
        match (selection, field) {
            (Selection::Menu(index), MenuField::Name) => {
                if let Some(menu) = menus.menus.get_mut(index) {
                    menu.name = value.to_string();
                }
            }
            (Selection::Item(..) | Selection::SubItem(..), _) => {
                match menus.selected_item_mut() {
                    Some(ItemDef::Action { label, action, shortcut, .. }) => match field {
                        MenuField::Label => *label = value.to_string(),
                        // An action name is a Rust type: refuse what would not
                        // compile rather than write it.
                        MenuField::Action if is_type_name(value) => *action = value.to_string(),
                        // Kept in the model and written at ⌘S with the rest.
                        // Written on the spot, it went to disk on every key —
                        // so on every half-typed state — and outlived the entry
                        // it named.
                        MenuField::Shortcut => {
                            let keystroke = value.trim();
                            *shortcut = match keystroke {
                                "" => None,
                                keystroke if crate::menufile::is_keystroke(keystroke) => {
                                    Some(keystroke.to_string())
                                }
                                // What is being typed is not readable yet:
                                // keep what was there.
                                _ => shortcut.clone(),
                            };
                        }
                        _ => {}
                    },
                    Some(ItemDef::Submenu(inner)) if field == MenuField::Label => {
                        inner.name = value.to_string();
                    }
                    _ => {}
                }
            }
            _ => {}
        }
        cx.notify();
    }

    /// Opens the handler of the selected entry in Zed, on its own line.
    pub fn open_menu_handler(&mut self, cx: &mut Context<Self>) {
        let Some(menus) = self.menu_file() else {
            return;
        };
        let Some(ItemDef::Action { action, os_action, .. }) = menus.selected_item() else {
            return;
        };
        if os_action.is_some() {
            self.message = Some(crate::tr("message.entry_is_system"));
            cx.notify();
            return;
        }
        if action.contains("::") {
            self.message = Some(SharedString::from(
                t!("message.action_elsewhere", action = action).into_owned(),
            ));
            cx.notify();
            return;
        }

        match menus.handler_line(action) {
            Some(line) => crate::tools::open_in_editor(cx, &menus.path, Some(line)),
            None => {
                self.message = Some(SharedString::from(
                    t!("message.action_unwired", action = action).into_owned(),
                ));
                cx.notify();
            }
        }
    }

    /// Selects a menu or one of its entries.
    pub fn select_menu(&mut self, selection: Selection, cx: &mut Context<Self>) {
        if let Some(menus) = self.menu_file_mut() {
            menus.selected = Some(selection);
            cx.notify();
        }
    }

    /// Leaves the menu editor.
    pub fn close_menu_file(&mut self, cx: &mut Context<Self>) {
        if self.discard_menu_edits(cx) {
            return;
        }
        self.menu_synced = None;
        self.show_designer();
        cx.notify();
    }

    /// Opens the project's menu bar for editing.
    ///
    /// `src/menus.rs` is a file like any other, so a click in the explorer
    /// opens it — but nothing in the window said so, and the editor was
    /// unfindable for anyone who had not been told.
    pub fn open_menu_bar(&mut self, cx: &mut Context<Self>) {
        let Some(project) = self.project.as_ref() else {
            self.message = Some(crate::tr("message.no_project"));
            cx.notify();
            return;
        };
        if self.menu_file().is_some() {
            // Already open, and nothing else can be covering it now that the
            // middle holds one thing at a time.
            cx.notify();
            return;
        }
        let root = project.root.clone();
        let path = root.join("src/menus.rs");

        // A project made before maxx generated a menu bar — or one whose bar
        // was deleted — gets one now rather than a refusal.
        let added = !path.exists();
        if added {
            if let Err(error) = crate::scaffold::add_menu_bar(&root) {
                self.message = Some(SharedString::from(error.to_string()));
                cx.notify();
                return;
            }
            self.refresh_entries();
        }

        self.select_file(path, cx);
        if added && self.message.is_none() {
            self.message = Some(crate::tr("message.menu_bar_added"));
        }
    }

    /// Takes the menu bar away from the project: `src/menus.rs` to the Trash,
    /// `main.rs` unwired.
    pub fn remove_menu_bar(&mut self, cx: &mut Context<Self>) {
        let Some(project) = self.project.as_ref() else {
            self.message = Some(crate::tr("message.no_project"));
            cx.notify();
            return;
        };
        let path = project.root.join("src/menus.rs");
        if !path.exists() {
            self.message = Some(crate::tr("message.no_menu_bar"));
            cx.notify();
            return;
        }
        // Through the panel's own delete, so the file goes to the Trash and
        // `main.rs` is unwired exactly once, in one place.
        self.selected = Some(path);
        self.delete_selected_entry(cx);
    }

    /// Adds a menu to the bar.
    pub fn add_menu(&mut self, cx: &mut Context<Self>) {
        self.open_menu_bar(cx);
        if let Some(menus) = self.menu_file_mut() {
            menus.add_menu();
            cx.notify();
        }
    }

    /// Adds an entry to the selected menu.
    pub fn add_menu_item(&mut self, separator: bool, cx: &mut Context<Self>) {
        self.open_menu_bar(cx);
        let Some(menus) = self.menu_file_mut() else {
            return;
        };
        if menus.selected.is_none() {
            self.message = Some(crate::tr("message.select_menu_first"));
            cx.notify();
            return;
        }
        let item = if separator {
            ItemDef::Separator
        } else {
            ItemDef::Action {
                label: crate::tr("menu.new_entry_label").to_string(),
                action: "MyAction".into(),
                os_action: None,
                shortcut: None,
            }
        };
        menus.add_item(item);
        cx.notify();
    }

    /// Moves the selected menu or entry one place up, or down.
    pub fn move_menu_selection(&mut self, up: bool, cx: &mut Context<Self>) {
        let Some(menus) = self.menu_file_mut() else {
            return;
        };
        if menus.selected.is_none() {
            self.message = Some(crate::tr("message.select_entry_first_menu"));
            cx.notify();
            return;
        }
        if menus.move_selected(up) {
            // Without this, the "already last" of a blocked move outlived every
            // move that followed.
            self.message = None;
        } else {
            // Already at the end of its list: saying so beats a click that
            // looks broken.
            self.message =
                Some(crate::tr(if up { "message.already_first" } else { "message.already_last" }));
        }
        cx.notify();
    }

    /// Adds a submenu to the selected menu.
    ///
    /// Only inside a menu of the bar: a submenu of a submenu is a place nobody
    /// finds twice, and the model stops at one level on purpose.
    /// Moves a menu or an entry to where it was dropped.
    ///
    /// The one gesture that carries an entry from one menu to another: the two
    /// reorder keys stay inside their list on purpose, and this is where the
    /// boundary is meant to be crossed.
    pub fn drop_menu_row(
        &mut self,
        from: crate::menufile::Selection,
        to: crate::menufile::Drop,
        cx: &mut Context<Self>,
    ) {
        let Some(menus) = self.menu_file_mut() else {
            return;
        };
        if !menus.move_to(from, to) {
            // Nothing moved: put back where it was, or refused by the model —
            // a menu is not an entry, a submenu does not go inside a submenu,
            // and nothing goes into an unreadable menu.
            return;
        }
        // The selection after the drop can name the same rank as before while
        // naming a different entry. `sync_menu_inputs` compares the selection
        // only: without this forced forgetting, the boxes keep the previous
        // entry's text, and the next keystroke writes it onto this one.
        self.menu_synced = None;
        cx.notify();
    }

    pub fn add_submenu(&mut self, cx: &mut Context<Self>) {
        self.open_menu_bar(cx);
        let Some(menus) = self.menu_file_mut() else {
            return;
        };
        // A selected submenu would take the entry inside itself, which is what
        // `add_item` does on purpose for the other entries: here it would give
        // the submenu of a submenu the model cannot show, and which could then
        // be neither selected nor removed.
        let dans_un_sous_menu = matches!(menus.selected, Some(Selection::SubItem(..)))
            || matches!(menus.selected_item(), Some(ItemDef::Submenu(_)));
        if menus.selected.is_none() || dans_un_sous_menu {
            self.message = Some(crate::tr("message.select_menu_not_submenu"));
            cx.notify();
            return;
        }
        menus.add_item(ItemDef::Submenu(crate::menu_model::MenuDef::named("Submenu")));
        cx.notify();
    }

    /// Removes the selected menu or entry.
    pub fn remove_menu_selection(&mut self, cx: &mut Context<Self>) {
        if let Some(menus) = self.menu_file_mut() {
            menus.remove_selected();
            cx.notify();
        }
    }
}
