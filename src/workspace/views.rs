//! Open views: the tab strip, and reading a view from disk or writing it back.

use super::*;

impl Workspace {
    /// The view being designed.
    pub fn view(&self) -> Option<&View> {
        self.views.get(self.active?)
    }

    /// The view being designed, mutably.
    pub fn view_mut(&mut self) -> Option<&mut View> {
        self.views.get_mut(self.active?)
    }

    /// Every open view, in tab order.
    pub fn open_views(&self) -> &[View] {
        &self.views
    }

    /// Index of the view being designed.
    pub fn active_index(&self) -> Option<usize> {
        self.active
    }

    /// Brings the view at `index` to the front.
    pub fn activate_view(&mut self, index: usize, cx: &mut Context<Self>) {
        self.edit_snapshot = None;
        if self.discard_menu_edits(cx) {
            return;
        }
        // The menu editor and the preferences are modes: clicking a tab has
        // to leave them, or the tab strip stays without effect.
        self.menu_file = None;
        self.preferences = false;
        if index < self.views.len() {
            self.active = Some(index);
            self.selected = Some(self.views[index].path.clone());
            self.revision += 1;
            self.message = None;
            cx.notify();
        }
    }

    /// Closes the view at `index`. A view with unsaved edits is kept, with a
    /// message, rather than discarded.
    pub fn close_view(&mut self, index: usize, cx: &mut Context<Self>) {
        let Some(view) = self.views.get(index) else {
            return;
        };
        if view.dirty() {
            self.message = Some(SharedString::from(format!(
                "{} n'est pas enregistré — ⌘S avant de fermer",
                view.name()
            )));
            cx.notify();
            return;
        }
        self.views.remove(index);
        self.active = match self.active {
            Some(_) if self.views.is_empty() => None,
            Some(active) if active >= index && active > 0 => Some(active - 1),
            Some(active) => Some(active.min(self.views.len() - 1)),
            None => None,
        };
        self.revision += 1;
        cx.notify();
    }

    pub(super) fn select_file(&mut self, path: PathBuf, cx: &mut Context<Self>) {
        self.message = None;
        // Opening anything leaves the preferences, for the same reason.
        self.preferences = false;
        if MenuFile::is_menu_file(&path) {
            // Already open and edited: reloading would drop those edits.
            if self.menu_file.as_ref().is_some_and(|menus| menus.path == path) {
                self.selected = Some(path);
                cx.notify();
                return;
            }
            if self.discard_menu_edits(cx) {
                return;
            }
            self.menu_file = None;
            match MenuFile::load(&path) {
                Ok(menus) => {
                    self.menu_file = Some(menus);
                    self.menu_synced = None;
                }
                Err(error) => self.message = Some(SharedString::from(error)),
            }
            self.selected = Some(path);
            cx.notify();
            return;
        }
        if self.discard_menu_edits(cx) {
            return;
        }
        self.menu_file = None;
        if path.extension().is_some_and(|extension| extension == "rs") {
            // Already open: just bring its tab forward.
            if let Some(index) = self.views.iter().position(|view| view.path == path) {
                self.active = Some(index);
                self.revision += 1;
            } else {
                match View::load(&path) {
                    Ok(view) => {
                        self.views.push(view);
                        self.active = Some(self.views.len() - 1);
                        self.revision += 1;
                    }
                    Err(error) => self.message = Some(SharedString::from(error)),
                }
            }
        }
        self.selected = Some(path);
        cx.notify();
    }

    /// Adds a view to the open project and opens it.
    ///
    /// The name is generated rather than asked for: a modal text prompt lands
    /// with the editor, and `view_2` is renamable in Zed in two seconds.
    pub fn new_view(&mut self, cx: &mut Context<Self>) {
        let Some(project) = self.project.as_ref() else {
            return;
        };
        let root = project.root.clone();

        let mut index = 1;
        let module = loop {
            let candidate = format!("view_{index}");
            if !root.join(format!("src/ui/{candidate}.rs")).exists() {
                break candidate;
            }
            index += 1;
        };

        match crate::scaffold::create_view(&root, &module) {
            Ok(()) => {
                self.refresh_entries();
                let path = root.join(format!("src/ui/{module}.rs"));
                self.select_file(path, cx);
                // `select_file` reports its own failure; only claim success when
                // it stayed quiet.
                if self.message.is_none() {
                    self.message = Some(SharedString::from(format!("{module}.rs créé")));
                }
            }
            Err(error) => self.message = Some(SharedString::from(error.to_string())),
        }
        cx.notify();
    }

    /// Writes the view back to its file, refusing when the file changed
    /// underneath.
    pub fn save_view(&mut self, cx: &mut Context<Self>) {
        self.write_view(false, cx);
    }

    /// Writes the view even though the file changed on disk, losing what was
    /// written there.
    pub fn overwrite_view(&mut self, cx: &mut Context<Self>) {
        self.write_view(true, cx);
    }

    fn write_view(&mut self, force: bool, cx: &mut Context<Self>) {
        if let Some(menus) = self.menu_file.as_mut() {
            let path = menus.path.clone();
            self.message = match menus.save(force) {
                Ok(()) => Some(SharedString::from(format!("{} enregistré", menus.name()))),
                Err(error) => Some(SharedString::from(error)),
            };
            self.format_after_save(&path, cx);
            cx.notify();
            return;
        }
        let Some(view) = self.view() else {
            return;
        };
        let path = view.path.clone();

        if !force && view.disk_changed() {
            if !view.dirty() {
                // Nothing to lose on this side: take what is on disk.
                self.reload_view(cx);
                return;
            }
            self.conflicts.insert(path);
            self.message = Some(SharedString::from(
                "fichier modifié en dehors de maxx — Fichier > Recharger, ou Écraser",
            ));
            cx.notify();
            return;
        }

        let view = self.view_mut().expect("just borrowed");
        self.message = match view.save() {
            Ok(()) => Some(SharedString::from(format!("{} enregistré", view.name()))),
            Err(error) => Some(SharedString::from(error)),
        };
        self.format_after_save(&path, cx);
        self.conflicts.remove(&path);
        self.revision += 1;
        cx.notify();
    }

    /// Passes the freshly written file through `rustfmt`, when asked to.
    ///
    /// The re-read afterwards is not optional: maxx holds a copy of the file
    /// and compares it with the disk to notice edits made elsewhere. Leaving
    /// that copy behind would make the very next save believe someone had
    /// changed the file underneath — maxx accusing itself.
    fn format_after_save(&mut self, path: &std::path::Path, cx: &mut Context<Self>) {
        if !crate::settings::prefs(cx).format_on_save {
            return;
        }
        // Nothing to format if the save itself failed.
        if self.message.as_deref().is_some_and(|message| !message.ends_with("enregistré")) {
            return;
        }

        match crate::run::format_rust(path) {
            Ok(false) => {}
            Ok(true) => {
                let reloaded = match self.menu_file.as_mut() {
                    Some(menus) if menus.path == path => menus.reload().err(),
                    _ => self.view_mut().and_then(|view| view.reload().err()),
                };
                if let Some(error) = reloaded {
                    self.message = Some(SharedString::from(error));
                } else {
                    self.menu_synced = None;
                    self.revision += 1;
                }
            }
            Err(error) => self.message = Some(SharedString::from(error)),
        }
    }

    /// Drops what the designer holds and re-reads the file.
    pub fn reload_view(&mut self, cx: &mut Context<Self>) {
        self.edit_snapshot = None;
        if let Some(menus) = self.menu_file.as_mut() {
            self.message = match menus.reload() {
                Ok(()) => Some(SharedString::from("menus rechargés")),
                Err(error) => Some(SharedString::from(error)),
            };
            self.menu_synced = None;
            cx.notify();
            return;
        }
        let Some(view) = self.view_mut() else {
            return;
        };
        let path = view.path.clone();
        let name = view.name();
        self.message = match view.reload() {
            Ok(()) => Some(SharedString::from(format!("{name} rechargé"))),
            Err(error) => Some(SharedString::from(error)),
        };
        self.conflicts.remove(&path);
        self.revision += 1;
        cx.notify();
    }

    /// Notices files changed outside maxx.
    ///
    /// A view the designer has not touched is reloaded without asking — the
    /// habit every editor gives you for an unmodified buffer. One changed on
    /// both sides is a real conflict and waits for a decision.
    pub(super) fn check_disk(&mut self, cx: &mut Context<Self>) {
        let mut reloaded = Vec::new();
        let mut conflicted = Vec::new();

        for index in 0..self.views.len() {
            let view = &self.views[index];
            if !view.disk_changed() {
                continue;
            }
            if view.dirty() {
                conflicted.push(view.path.clone());
            } else {
                reloaded.push(index);
            }
        }

        // Only a view that actually moved invalidates the snapshot; clearing it
        // on every return to the window would swallow the undo step for a text
        // edit interrupted by an alt-tab.
        if !reloaded.is_empty() || !conflicted.is_empty() {
            self.edit_snapshot = None;
        }

        for index in reloaded {
            let view = &mut self.views[index];
            let name = view.name();
            if view.reload().is_ok() {
                self.message = Some(SharedString::from(format!(
                    "{name} rechargé — modifié en dehors de maxx"
                )));
                self.revision += 1;
            }
        }
        for path in conflicted {
            if self.conflicts.insert(path) {
                self.message = Some(SharedString::from(
                    "modifié des deux côtés — Fichier > Recharger, ou Écraser",
                ));
                self.revision += 1;
            }
        }
        cx.notify();
    }

    /// Puts maxx's markers around the expression a hand-written `render`
    /// returns, then opens the view.
    pub fn adopt_view(&mut self, cx: &mut Context<Self>) {
        let Some(path) = self
            .selected
            .clone()
            .filter(|path| path.extension().is_some_and(|extension| extension == "rs"))
        else {
            self.message =
                Some(SharedString::from("sélectionnez un fichier .rs dans l'explorateur"));
            cx.notify();
            return;
        };

        let source = match std::fs::read_to_string(&path) {
            Ok(source) => source,
            Err(error) => {
                self.message = Some(SharedString::from(error.to_string()));
                cx.notify();
                return;
            }
        };

        match crate::parser::adopt(&source) {
            Ok(adopted) => match std::fs::write(&path, &adopted) {
                Ok(()) => {
                    self.message = None;
                    self.select_file(path, cx);
                    if self.message.is_none() {
                        self.message = Some(SharedString::from("vue adoptée"));
                    }
                }
                Err(error) => self.message = Some(SharedString::from(error.to_string())),
            },
            Err(error) => self.message = Some(SharedString::from(error.to_string())),
        }
        cx.notify();
    }
}
