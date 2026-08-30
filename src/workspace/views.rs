//! Open views: the tab strip, and reading a view from disk or writing it back.

use futures::StreamExt;

use super::*;

/// How long the watcher waits for the disk to fall silent before re-reading.
const SETTLE: std::time::Duration = std::time::Duration::from_millis(150);

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
        // The menu editor, the palette and the preferences are modes: clicking
        // a tab has to leave them, or the tab strip stays without effect. The
        // strip is drawn *above* each of them for exactly that reason.
        self.show_designer();
        if index < self.views.len() {
            self.focus_view(index);
            self.selected = Some(self.views[index].path.clone());
            self.revision += 1;
            self.message = None;
            cx.notify();
        }
    }

    /// Brings the view at `index` forward, remembering where one came from.
    ///
    /// Every way in goes through here — the tab strip, the project tree, the
    /// menu, `⌘P` — because `⌃⇥` is only useful between the two files one is
    /// actually working between, and those are rarely reached by clicking
    /// tabs. Written straight to `self.active` anywhere else, the trace is
    /// never taken and the gesture answers "nowhere to go back to".
    fn focus_view(&mut self, index: usize) {
        // Only when it is another view: coming back to the one already in
        // front would make the gesture answer itself.
        if let Some(current) = self.active.filter(|current| *current != index) {
            self.previous_view = self.views.get(current).map(|view| view.path.clone());
        }
        self.active = Some(index);
    }

    /// Brings the next tab forward, or the previous one.
    ///
    /// The strip is a ring: `⌘⌥→` on the last tab goes to the first, because
    /// stopping there would do nothing exactly when there is somewhere to go.
    pub fn step_view(&mut self, forward: bool, cx: &mut Context<Self>) {
        let Some(current) = self.active else {
            return;
        };
        let Some(next) = crate::tabs::step(current, self.views.len(), forward) else {
            return;
        };
        self.activate_view(next, cx);
    }

    /// Goes back to the file one was on before.
    ///
    /// `⌃⇥`, and pressed twice it comes back — which is the whole use of it:
    /// two files one is working between, without looking at the strip.
    pub fn last_view(&mut self, cx: &mut Context<Self>) {
        let paths: Vec<PathBuf> = self.views.iter().map(|view| view.path.clone()).collect();
        let Some(index) = crate::tabs::position_of(&paths, self.previous_view.as_deref()) else {
            self.message = Some(crate::tr("message.no_previous_view"));
            cx.notify();
            return;
        };
        self.activate_view(index, cx);
    }

    /// Closes the view at `index`. A view with unsaved edits is kept, with a
    /// message, rather than discarded.
    pub fn close_view(&mut self, index: usize, cx: &mut Context<Self>) {
        let Some(view) = self.views.get(index) else {
            return;
        };
        if view.dirty() {
            self.message = Some(SharedString::from(
                t!("message.view_unsaved_close", name = view.name()).into_owned(),
            ));
            cx.notify();
            return;
        }
        // The reader may be showing this very view's code, and `render_designer`
        // tests `self.code` before it tests `self.view()`: left behind, the
        // panel would go on rendering a document that is no longer open, with
        // no tab left to leave it by.
        let path = view.path.clone();
        self.forget_code(|candidate| candidate == path);
        // The file one came from is closing: there is nothing to go back to,
        // and holding its path would send `⌃⇥` to a tab that is gone.
        if self.previous_view.as_deref() == Some(path.as_path()) {
            self.previous_view = None;
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
        if crate::themefile::ThemeFile::is_palette_file(&path) {
            // Already open: leave it be rather than reload, which would drop
            // the pickers and shut a popup someone is picking from.
            if self.palette().is_some_and(|palette| palette.path == path) {
                self.selected = Some(path);
                cx.notify();
                return;
            }
            if self.discard_menu_edits(cx) {
                return;
            }
            match crate::themefile::ThemeFile::open(&path) {
                Some(palette) => self.show(Center::Palette(palette)),
                // A `src/theme.rs` this reader cannot make sense of is not a
                // broken palette: it is a file the developer rewrote, and the
                // code reader shows it rather than an empty screen pretending
                // there are no colours in it.
                None => self.open_code(path.clone(), cx),
            }
            self.selected = Some(path);
            cx.notify();
            return;
        }
        if MenuFile::is_menu_file(&path) {
            // Already open and edited: reloading would drop those edits.
            if self.menu_file().is_some_and(|menus| menus.path == path) {
                self.selected = Some(path);
                cx.notify();
                return;
            }
            if self.discard_menu_edits(cx) {
                return;
            }
            match MenuFile::load(&path) {
                Ok(menus) => {
                    self.menu_synced = None;
                    self.show(Center::Menus(menus));
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
        if path.extension().is_some_and(|extension| extension == "rs") {
            // Already open: just bring its tab forward.
            if let Some(index) = self.views.iter().position(|view| view.path == path) {
                self.show_designer();
                self.focus_view(index);
                self.revision += 1;
            } else {
                match View::load(&path) {
                    Ok(view) => {
                        self.show_designer();
                        self.views.push(view);
                        self.focus_view(self.views.len() - 1);
                        self.revision += 1;
                    }
                    // A `.rs` without a managed region is not a broken view: it
                    // is `main.rs`, `ui/mod.rs`, a module written by hand. The
                    // reader shows it rather than the parser refusing it.
                    //
                    // Only that one reason, though. `View::load` also refuses a
                    // syntax error in the region and markers in the wrong order,
                    // and those are a diagnosis: swallowing them would leave a
                    // read-only panel and no clue what maxx choked on. The file
                    // is read a second time here, on the failing path only.
                    Err(error) => {
                        let no_region = std::fs::read_to_string(&path).is_ok_and(|source| {
                            matches!(
                                crate::parser::locate(&source),
                                Err(crate::parser::Error::NoMarkers)
                            )
                        });
                        if no_region {
                            self.open_code(path, cx);
                            return;
                        }
                        self.message = Some(SharedString::from(error));
                    }
                }
            }
        } else {
            self.open_code(path, cx);
            return;
        }
        self.selected = Some(path);
        cx.notify();
    }

    /// Adds a view to the open project and opens it.
    ///
    /// The name is generated rather than asked for: a modal text prompt lands
    /// with the editor, and a generated name is one less thing to invent before
    /// drawing anything.
    ///
    /// Renaming it afterwards is on the developer, in their editor, and it is
    /// four places rather than one — the file, its line in `src/ui/mod.rs`, the
    /// type, and `main.rs` when this is the view it opens. maxx does not do it
    /// yet; the backlog says what it would take.
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
                    self.message = Some(SharedString::from(
                        t!("message.view_created", module = module).into_owned(),
                    ));
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
        if let Some(menus) = self.menu_file_mut() {
            let path = menus.path.clone();
            let saved = match menus.save(force) {
                Ok(()) => {
                    self.message = Some(SharedString::from(
                        t!("message.saved", name = menus.name()).into_owned(),
                    ));
                    true
                }
                Err(error) => {
                    self.message = Some(SharedString::from(error));
                    false
                }
            };
            self.format_after_save(&path, saved, cx);
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
            self.message = Some(crate::tr("error.changed_on_disk"));
            cx.notify();
            return;
        }

        let view = self.view_mut().expect("just borrowed");
        let saved = match view.save() {
            Ok(()) => {
                self.message =
                    Some(SharedString::from(t!("message.saved", name = view.name()).into_owned()));
                true
            }
            Err(error) => {
                self.message = Some(SharedString::from(error));
                false
            }
        };
        // A picture asked for by name needs an `AssetSource`, and a project
        // that declares none draws nothing and says so only in the log. Saving
        // is the one place every image passes through — a drop, a paste, an
        // undo, a hand-written view maxx has adopted.
        if saved {
            self.ensure_assets_module();
        }
        self.format_after_save(&path, saved, cx);
        self.conflicts.remove(&path);
        self.revision += 1;
        cx.notify();
    }

    /// Adds the assets module when the view just written draws one.
    ///
    /// Adding rather than saying it and stopping: a bare string with no source
    /// behind it is the silent failure this module exists to remove, and saving
    /// a view already adds a field, an import and a handler stub to the
    /// developer's file.
    fn ensure_assets_module(&mut self) {
        let Some(project) = self.project.as_ref() else {
            return;
        };
        let root = project.root.clone();
        if root.join("src/assets.rs").exists() || self.assets_refused.contains(&root) {
            return;
        }
        // A module the project once carried and no longer does was deleted on
        // purpose — `maxx.toml` remembers it. Putting it back on the next save
        // would be maxx arguing with the developer, and there would be no way
        // to win the argument.
        if crate::projectfile::load(&root).modules.contains_key("assets") {
            return;
        }
        let Some(view) = self.view() else {
            return;
        };
        if !crate::registry::uses_an_asset(&view.root) {
            return;
        }
        match crate::scaffold::add_assets_module(&root) {
            Ok(()) => {
                self.refresh_entries();
                self.message = Some(crate::tr("message.assets_added_for_image"));
            }
            Err(error) => {
                self.assets_refused.insert(root);
                self.message = Some(SharedString::from(error.to_string()));
            }
        }
    }

    /// Passes the freshly written file through `rustfmt`, when asked to.
    ///
    /// The re-read afterwards is not optional: maxx holds a copy of the file
    /// and compares it with the disk to notice edits made elsewhere. Leaving
    /// that copy behind would make the very next save believe someone had
    /// changed the file underneath — maxx accusing itself.
    /// `saved` says whether the write went through. Told rather than deduced
    /// from the message: reading the outcome back out of a sentence shown to
    /// the user tied the behaviour to its wording, and the wording is
    /// translated.
    fn format_after_save(&mut self, path: &std::path::Path, saved: bool, cx: &mut Context<Self>) {
        if !saved || !crate::settings::prefs(cx).format_on_save {
            return;
        }

        match crate::run::format_rust(path) {
            Ok(false) => {}
            Ok(true) => {
                let reloaded = match self.menu_file_mut() {
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
        if let Some(menus) = self.menu_file_mut() {
            self.message = match menus.reload() {
                Ok(()) => Some(crate::tr("message.menus_reloaded")),
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
            Ok(()) => Some(SharedString::from(t!("message.reloaded", name = name).into_owned())),
            Err(error) => Some(SharedString::from(error)),
        };
        self.conflicts.remove(&path);
        self.revision += 1;
        cx.notify();
    }

    /// Watches the open project, so an edit made elsewhere reaches the canvas
    /// without waiting for the window to come back.
    ///
    /// It ends in the same `check_disk` the return of focus runs: this is a
    /// second trigger for it, not a second way of reloading. Called on both
    /// sides of the project's life — it starts by dropping whatever was
    /// watching before, and returns early when there is no project, so closing
    /// one needs no second method.
    pub(super) fn watch_project(&mut self, cx: &mut Context<Self>) {
        self.watch_task = None;
        self.watcher = None;
        let Some(root) = self.project.as_ref().map(|project| project.root.clone()) else {
            return;
        };
        let Some((mut receiver, watcher)) = crate::watch::start(&root) else {
            // Nothing said here either: `watch::start` has already written to
            // the log, and the check on returning to the window still covers it.
            return;
        };
        self.watcher = Some(watcher);

        self.watch_task = Some(cx.spawn(async move |workspace, cx| {
            loop {
                // Awaited rather than polled: a window left open all afternoon
                // must cost nothing while nothing happens, and a timer ticking
                // ten times a second forever is what keeps a laptop from idling.
                if StreamExt::next(&mut receiver).await.is_none() {
                    return;
                }

                // Then wait for silence. An editor's save is several events — a
                // write, a rename, sometimes a temporary file beside it — and
                // reading between two of them reads a file that is half written:
                // `View::load` fails on it, `check_disk` drops the failure, and
                // if the discarded event was the last one the canvas stays stale
                // until the window is focused again.
                loop {
                    cx.background_executor().timer(SETTLE).await;
                    let mut more = false;
                    while receiver.try_recv().is_ok() {
                        more = true;
                    }
                    if !more {
                        break;
                    }
                }

                let updated = workspace.update(cx, |workspace, cx| {
                    // The tree first: it takes no `cx` and notifies nobody,
                    // where `check_disk` ends on `cx.notify()`. The tree only —
                    // a `git checkout` unlinks files for an instant, and closing
                    // a document because git blinked cannot be undone.
                    workspace.refresh_entries();
                    workspace.check_disk(cx);
                });
                if updated.is_err() {
                    return;
                }
            }
        }));
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
        //
        // A conflict does not move anything on maxx's side, and it is not the
        // one-off the focus edge made it look like: once a view is dirty and
        // changed on disk it stays in `conflicted` on every later pass, so
        // counting it here would drop the snapshot on any unrelated file event
        // — which, with the watcher, means while someone is typing in the
        // inspector, and `close_text_edit` then records no undo step at all.
        if !reloaded.is_empty() {
            self.edit_snapshot = None;
        }

        for index in reloaded {
            let view = &mut self.views[index];
            let name = view.name();
            if view.reload().is_ok() {
                self.message = Some(SharedString::from(
                    t!("message.reloaded_outside", name = name).into_owned(),
                ));
                self.revision += 1;
            }
        }
        // No `revision` bump here, where the reload above has one: nothing moved
        // on maxx's side, and `conflicts` is read in one place — the status bar,
        // which the `cx.notify()` below repaints anyway. The bump was doing
        // nothing until the watcher arrived; now it would rebuild the inspector
        // fields, and the caret with them, while someone is typing in one.
        for path in conflicted {
            if self.conflicts.insert(path) {
                self.message = Some(crate::tr("message.conflict_both"));
            }
        }
        // The palette has nothing to lose on maxx's side — a picker holds no
        // unsaved state, every turn of one is already on disk — so it is taken
        // back from the file without asking, the way an untouched view is.
        let root = self.project.as_ref().map(|project| project.root.clone());
        let reloaded = match self.palette_mut() {
            Some(palette) => palette.reload().then(|| {
                // The project's palette, or nothing: the same editor also serves
                // the user's default one, and painting an open project with a
                // file belonging to no project is describing one by another.
                let of_project = root.as_ref().is_some_and(|root| palette.path.starts_with(root));
                of_project.then(|| crate::preview::Preview::from_file(palette))
            }),
            None => None,
        };
        if let Some(preview) = reloaded {
            self.palette_synced = None;
            if let Some(preview) = preview {
                self.preview = preview;
            }
        }
        // And the canvas follows the file even when nobody has the palette
        // open, which is the ordinary case: it is the preview, not the editor,
        // that has to tell the truth about the project's colours.
        if let Some(root) = root {
            // Read from disk unless the project's OWN palette is open, which is
            // the only case where a fresher copy is already in hand. With the
            // *default* palette open, `palette().is_some()` was true and a real
            // change to the project's `src/theme.rs` went unseen.
            let holding_it = self.palette().is_some_and(|palette| palette.path.starts_with(&root));
            if !holding_it {
                self.preview = crate::preview::Preview::read(&root);
            }
            // And the components. The case this is written for is a developer
            // writing one in Zed while maxx is open — without this it reaches
            // the palette only at the next opening of the project. Worse, a
            // `new` whose arguments changed there would go on being dropped in
            // its old shape, which no longer compiles.
            let bricks = crate::bricks::read(&root);
            if bricks != self.bricks {
                self.bricks = bricks;
                // The inspector's fields are built from these, and its key is
                // `(revision, selection)` — neither of which moved. Without
                // this, a `pub fn subtitle` added in Zed while its node is
                // selected shows a labelled row with no field in it until the
                // selection has been somewhere else and come back.
                self.synced = None;
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
            self.message = Some(crate::tr("message.select_rs_file"));
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
                        self.message = Some(crate::tr("message.view_adopted"));
                    }
                }
                Err(error) => self.message = Some(SharedString::from(error.to_string())),
            },
            Err(error) => self.message = Some(SharedString::from(error.to_string())),
        }
        cx.notify();
    }

    /// Makes the selected view the one the project's window opens on.
    ///
    /// The explorer's selection, not the active tab: this is a property of the
    /// project, and the file it names is the one the pointer is on — the same
    /// gesture as adopting a view.
    pub fn set_entry_view(&mut self, cx: &mut Context<Self>) {
        let Some(project) = self.project.as_ref() else {
            self.message = Some(crate::tr("message.no_project"));
            cx.notify();
            return;
        };
        let root = project.root.clone();

        let Some(path) = self
            .selected
            .clone()
            .filter(|path| path.extension().is_some_and(|extension| extension == "rs"))
        else {
            self.message = Some(crate::tr("message.select_rs_file"));
            cx.notify();
            return;
        };

        // A view lives in `src/ui/`, and `main.rs` reaches it through
        // `crate::ui::`: a file anywhere else could be imported, but the import
        // maxx would write for it would not compile.
        let module = path
            .strip_prefix(root.join("src/ui"))
            .ok()
            .filter(|relative| relative.components().count() == 1)
            .and_then(|relative| relative.file_stem())
            .map(|stem| stem.to_string_lossy().into_owned());
        let Some(module) = module else {
            self.message = Some(crate::tr("message.entry_outside_ui"));
            cx.notify();
            return;
        };

        match crate::scaffold::set_entry_view(&root, &module) {
            Ok(()) => {
                self.message =
                    Some(SharedString::from(t!("message.entry_set", name = module).into_owned()));
                // The reader may be showing the very `main.rs` that just
                // changed under it.
                let main = root.join("src/main.rs");
                self.forget_code(|candidate| candidate == main);
                // The explorer marks the entry view, and it has just moved.
                self.refresh_entries();
            }
            Err(error) => self.message = Some(SharedString::from(error.to_string())),
        }
        cx.notify();
    }
}
