//! The modules maxx copies into a project, and bringing them up to date.

use super::*;

impl Workspace {
    /// Copies the system module into the project and points the explorer at it.
    ///
    /// Pointed at, not opened: `system.rs` carries no managed region, so the
    /// designer has nothing to show for it — it is read and edited in the
    /// editor, like any other hand-written module.
    ///
    /// What every desktop application ends up writing on its second day —
    /// where its files go, and what "delete" means — and what nobody wants to
    /// write a third time. Copied source, not a dependency: a generated
    /// project owes nothing to maxx.
    pub fn add_system_module(&mut self, cx: &mut Context<Self>) {
        self.add_module("system", crate::scaffold::add_system_module, "message.system_added", cx);
    }

    /// Copies a module into the project, declares it, and points at it.
    ///
    /// Shared by the modules maxx knows how to add, so they all leave the
    /// window in the same state — no menu editor left in front of a file that
    /// was just written, and no unsaved menu edits dropped on the way.
    fn add_module(
        &mut self,
        module: &str,
        add: fn(&std::path::Path) -> std::io::Result<()>,
        added: &'static str,
        // `added` is a translation key, as everywhere else.
        cx: &mut Context<Self>,
    ) {
        let Some(project) = self.project.as_ref() else {
            self.message = Some(crate::tr("message.no_project"));
            cx.notify();
            return;
        };
        let root = project.root.clone();
        // A library is a directory, everything else a file. Asked here rather
        // than assumed: a wrong path makes `had_file` always false — so adding
        // twice says "added" twice — and leaves the panel selecting something
        // that is not there, which `Open in editor` then hands to Zed.
        let path = if crate::scaffold::module_is_directory(module) {
            root.join(format!("src/{module}"))
        } else {
            root.join(format!("src/{module}.rs"))
        };
        let declaration = format!("mod {module};");
        let had_file = path.exists();
        let had_declaration = std::fs::read_to_string(root.join("src/main.rs"))
            .is_ok_and(|source| source.lines().any(|line| line.trim() == declaration));

        // Unsaved menu edits come first: this leaves the menu editor, and
        // dropping them silently would be the worst way to add a file.
        if self.discard_menu_edits(cx) {
            return;
        }

        if let Err(error) = add(&root) {
            self.message = Some(SharedString::from(error.to_string()));
            cx.notify();
            return;
        }

        self.menu_synced = None;
        self.show_designer();
        self.refresh_entries();
        // The palette page reads `src/theme.rs`, which may have just appeared or
        // moved: the boxes are rebuilt from it on the next frame.
        self.palette_synced = None;
        self.selected = Some(path);
        self.message = Some(SharedString::from(match (had_file, had_declaration) {
            (true, true) => t!("message.module_already_there", module = module).into_owned(),
            // The file was there but nothing declared it — which is exactly
            // the state a half-finished delete leaves behind.
            (true, false) => t!("message.module_now_declared", module = module).into_owned(),
            _ => crate::tr(added).to_string(),
        }));
        cx.notify();
    }

    /// Copies the component library into the project.
    ///
    /// It brings the palette with it: the bricks paint with the project's own
    /// roles, so the roles have to be there before they are.
    pub fn add_components_module(&mut self, cx: &mut Context<Self>) {
        self.add_module(
            "components",
            crate::scaffold::add_components_module,
            "message.components_added",
            cx,
        );
    }

    /// Copies the palette into the project.
    ///
    /// Two modes from the start, because the choice belongs to whoever reads
    /// the screen and not to whoever wrote it — and because retrofitting a
    /// second mode onto colours already scattered through the views is the kind
    /// of work nobody does twice.
    pub fn add_theme_module(&mut self, cx: &mut Context<Self>) {
        self.add_module("theme", crate::scaffold::add_theme_module, "message.theme_added", cx);
    }

    /// Copies the settings module into the project.
    ///
    /// It brings the system module with it, and declares two crates in the
    /// project's `Cargo.toml` — both already compiled in the tree through
    /// gpui, so nothing gets slower.
    pub fn add_settings_module(&mut self, cx: &mut Context<Self>) {
        self.add_module(
            "settings",
            crate::scaffold::add_settings_module,
            "message.settings_added",
            cx,
        );
    }

    /// Copies the assets module into the project.
    ///
    /// It brings a `build.rs` with it, and hands the source to the application
    /// in `main.rs`. Without it a picture drawn by the project shows under
    /// `cargo run` from the root and nowhere else — an image asked for by name
    /// is looked up in an `AssetSource`, and a project that declares none draws
    /// nothing and says so only in the log.
    pub fn add_assets_module(&mut self, cx: &mut Context<Self>) {
        self.add_module("assets", crate::scaffold::add_assets_module, "message.assets_added", cx);
    }

    /// Copies the window module into the project.
    ///
    /// It brings the system module and the same two crates the settings take.
    /// What every desktop application ends up wanting on its second day — the
    /// window back where it was left — and what nobody wants to write twice.
    pub fn add_window_module(&mut self, cx: &mut Context<Self>) {
        self.add_module("window", crate::scaffold::add_window_module, "message.window_added", cx);
    }

    /// Replaces the copied modules a newer maxx has fixed.
    ///
    /// Only those the project has not touched: an edited file belongs to the
    /// developer, and maxx says so rather than deciding for them.
    pub fn update_modules(&mut self, cx: &mut Context<Self>) {
        let Some(project) = self.project.as_ref() else {
            self.message = Some(crate::tr("message.no_project"));
            cx.notify();
            return;
        };
        let root = project.root.clone();
        let outdated = crate::scaffold::outdated_modules(&root);

        if outdated.is_empty() {
            self.message = Some(crate::tr("message.modules_up_to_date"));
            cx.notify();
            return;
        }

        let mut updated = Vec::new();
        let mut failed = Vec::new();
        for module in &outdated {
            match crate::scaffold::update_module(&root, module) {
                Ok(()) => updated.push(module.clone()),
                Err(error) => failed.push(format!("{module} : {error}")),
            }
        }

        self.refresh_entries();
        // `theme` may be among them, and its file has just been rewritten.
        // Re-reading it is the point: clearing the key alone would rebuild the
        // pickers from the copy held here, which is the palette as it stood
        // *before* the update — the old colours, shown as if nothing happened.
        if let Some(palette) = self.palette_mut() {
            palette.reload();
        }
        self.palette_synced = None;
        self.preview = crate::preview::Preview::read(&root);
        self.message = Some(SharedString::from(if failed.is_empty() {
            t!("message.modules_updated", modules = updated.join(", ")).into_owned()
        } else {
            failed.join(" · ")
        }));
        cx.notify();
    }

    /// Says so when the project carries a module maxx has since fixed.
    ///
    /// A message and nothing more: replacing a file because someone opened a
    /// folder would be a poor way to earn trust.
    pub(super) fn announce_outdated_modules(&mut self, root: &std::path::Path) {
        let outdated = crate::scaffold::outdated_modules(root);
        if outdated.is_empty() {
            return;
        }
        self.message = Some(SharedString::from(
            t!("message.modules_outdated", modules = outdated.join(", ")).into_owned(),
        ));
    }
}
