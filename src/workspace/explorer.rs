//! The project panel: the file tree, its selection and its deletions.

use super::*;

impl Workspace {
    fn toggle_expanded(&mut self, path: PathBuf, cx: &mut Context<Self>) {
        if !self.expanded.remove(&path) {
            self.expanded.insert(path);
        }
        self.refresh_entries();
        cx.notify();
    }

    /// Highlights an entry of the project panel without opening it.
    ///
    /// A right click has to land on the entry it is about — the delete and
    /// reveal actions all read `selected` — but it must not open the file the
    /// way a left click does.
    pub fn select_entry(&mut self, path: PathBuf, cx: &mut Context<Self>) {
        self.selected = Some(path);
        cx.notify();
    }

    /// The entry the project panel is on, falling back to the project root.
    pub fn selected_entry(&self) -> Option<PathBuf> {
        self.selected.clone().or_else(|| self.project.as_ref().map(|project| project.root.clone()))
    }

    /// Moves the selected entry to the Trash, unregistering it when it is a
    /// view.
    ///
    /// Nothing is erased: the file lands in `~/.Trash`, so a wrong click costs
    /// a trip to the Finder and not the afternoon.
    pub fn delete_selected_entry(&mut self, cx: &mut Context<Self>) {
        let Some(project) = self.project.as_ref() else {
            return;
        };
        let root = project.root.clone();
        let Some(path) = self.selected.clone() else {
            self.message = Some(crate::tr("message.select_entry_first"));
            cx.notify();
            return;
        };

        if let Some(reason) = protected_entry(&root, &path) {
            self.message = Some(SharedString::from(reason));
            cx.notify();
            return;
        }

        let name = path
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_else(|| path.to_string_lossy().into_owned());

        // A directory takes everything under it: the modules have to be read
        // off the disk while it is still there.
        let is_dir = path.is_dir();
        let modules: Vec<String> = if is_dir {
            std::fs::read_dir(&path)
                .map(|entries| {
                    entries
                        .flatten()
                        .filter_map(|entry| view_module(&root, &entry.path()))
                        .collect()
                })
                .unwrap_or_default()
        } else {
            view_module(&root, &path).into_iter().collect()
        };

        if let Err(error) = crate::run::move_to_trash(&path) {
            self.message = Some(SharedString::from(error));
            cx.notify();
            return;
        }

        // A view carries a `pub mod` line in `src/ui/mod.rs`; leaving it there
        // breaks the build the file was deleted to keep clean. A directory
        // inside `src/ui/` is itself a module, on top of the views it held.
        for module in &modules {
            unregister_view(&root, module);
        }
        if is_dir
            && let Some(module) = path
                .strip_prefix(root.join("src/ui"))
                .ok()
                .and_then(|relative| relative.to_str())
                .filter(|relative| !relative.contains('/'))
        {
            unregister_view(&root, module);
        }
        // Same for the menu bar: the file is gone, so `main.rs` must stop
        // calling into it.
        if path == root.join("src/menus.rs") {
            let _ = crate::scaffold::remove_menu_bar(&root);
        } else if let Some(module) = top_level_module(&root, &path) {
            // Any other `src/<module>.rs`: its `mod` line would now name a
            // file that is gone, and the project would stop compiling — which
            // is the opposite of what deleting a file is for.
            let _ = crate::scaffold::remove_module(&root, &module);
        }

        // Every tab under it, the menu editor and the code reader are now
        // looking at a file that is gone.
        let gone = |candidate: &std::path::Path| {
            candidate == path || (is_dir && candidate.starts_with(&path))
        };
        while let Some(index) = self.views.iter().position(|view| gone(&view.path)) {
            self.views.remove(index);
            self.active = match self.active {
                Some(_) if self.views.is_empty() => None,
                Some(active) if active >= index && active > 0 => Some(active - 1),
                Some(active) => Some(active.min(self.views.len() - 1)),
                None => None,
            };
            self.revision += 1;
        }
        if self.menu_file.as_ref().is_some_and(|menus| gone(&menus.path)) {
            self.menu_file = None;
            self.menu_synced = None;
            self.palette = None;
        }
        self.forget_code(gone);

        self.selected = None;
        self.expanded.retain(|expanded| !gone(expanded));
        self.refresh_entries();
        self.message = Some(SharedString::from(t!("message.trashed", name = name).into_owned()));
        cx.notify();
    }

    pub(super) fn refresh_entries(&mut self) {
        self.entries = match &self.project {
            Some(project) => flatten(&project.root, &self.expanded),
            None => Vec::new(),
        };
        self.entry_view = self.project.as_ref().and_then(|project| {
            crate::projectfile::entry(&project.root)
                .map(|module| project.root.join(format!("src/ui/{module}.rs")))
        });
    }

    pub(super) fn render_project_panel(&self, cx: &mut Context<Self>) -> impl IntoElement {
        // One menu for the whole panel rather than one per row: `ContextMenu`
        // hard-codes its element id, so a menu per row would have every row
        // sharing the same open/position state.
        //
        // No entry is greyed out from the selection: `ContextMenu` builds the
        // menu from the builder of the frame it was painted with, so anything
        // computed here would be one right click behind. `DeleteFile` reports
        // its own refusal.
        //
        // The editor's name is read here rather than inside the builder, which
        // is `'static` and cannot hold the application. Changing the editor
        // repaints every workspace, so the label follows on the next frame.
        let editor = crate::tools::editor_label(cx);

        div()
            .flex()
            .flex_col()
            // No width here: the resizable panel is what gives it. A fixed width
            // inside left the tree at 240 px in a wider panel, and the empty
            // strip between the two showed.
            .size_full()
            .bg(theme::panel_bg())
            .border_r_1()
            .border_color(theme::border())
            .child(
                div()
                    .flex()
                    .items_center()
                    .h(px(28.))
                    .pl(px(12.))
                    .pr(px(4.))
                    .child(
                        div()
                            .flex_1()
                            .text_xs()
                            .text_color(theme::text_muted())
                            .child(crate::tr("explorer.title")),
                    )
                    .child(panel_icon(
                        "panel-new-view",
                        "＋",
                        "explorer.new_view",
                        cx,
                        |this, cx| this.new_view(cx),
                    ))
                    .child(panel_icon(
                        "panel-delete",
                        "🗑",
                        "explorer.delete",
                        cx,
                        |this, cx| this.delete_selected_entry(cx),
                    )),
            )
            .child(
                uniform_list(
                    "project-entries",
                    self.entries.len(),
                    cx.processor(|this, range: std::ops::Range<usize>, _window, cx| {
                        range
                            .filter_map(|ix| this.entries.get(ix).cloned())
                            .map(|entry| this.render_entry(entry, cx))
                            .collect::<Vec<_>>()
                    }),
                )
                .flex_1(),
            )
            .context_menu(move |menu, _window, _cx| {
                menu.menu(crate::tr("context.new_view"), Box::new(crate::actions::NewView))
                    .menu(crate::tr("context.delete"), Box::new(crate::actions::DeleteFile))
                    .separator()
                    .menu(crate::tr("menu.set_entry_view"), Box::new(crate::actions::SetEntryView))
                    .separator()
                    .menu(crate::tr("context.view_code"), Box::new(crate::actions::ViewCode))
                    .menu(crate::tr("context.reveal"), Box::new(crate::actions::RevealInFinder))
                    .menu(
                        t!("context.open_in", editor = editor).into_owned(),
                        Box::new(crate::actions::OpenInZed),
                    )
            })
    }

    fn render_entry(&self, entry: Entry, cx: &mut Context<Self>) -> AnyElement {
        let is_selected = self.selected.as_deref() == Some(entry.path.as_path());
        let is_entry = self.entry_view.as_deref() == Some(entry.path.as_path());
        let is_expanded = self.expanded.contains(&entry.path);
        let marker = if entry.is_dir { if is_expanded { "▾" } else { "▸" } } else { " " };
        let path = entry.path.clone();
        let is_dir = entry.is_dir;

        let menu_path = entry.path.clone();

        div()
            .id(SharedString::from(entry.path.to_string_lossy().into_owned()))
            .flex()
            .items_center()
            .gap_1()
            .h(px(22.))
            .pr_2()
            .pl(px(8. + 12. * entry.depth as f32))
            .cursor_pointer()
            .when(is_selected, |this| this.bg(theme::selected_bg()))
            .hover(|this| this.bg(theme::hover_bg()))
            .child(
                div()
                    .w(px(12.))
                    .flex_none()
                    .text_xs()
                    .text_color(theme::text_muted())
                    .child(marker),
            )
            .child(
                div()
                    .when(is_dir, |this| this.text_color(theme::accent()))
                    .child(entry.name.clone()),
            )
            // The view the window opens on, marked where the files are rather
            // than on the tab strip: it stays true when the view is closed,
            // which is when the question comes up.
            .when(is_entry, |this| {
                this.child(div().text_xs().text_color(theme::accent()).child("●"))
            })
            .on_click(cx.listener(move |this, _, _window, cx| {
                if is_dir {
                    this.toggle_expanded(path.clone(), cx);
                } else {
                    this.select_file(path.clone(), cx);
                }
            }))
            // The menu acts on `selected`, so the right click has to move the
            // selection before the menu is built — which it does, the menu
            // being deferred to the next frame.
            .on_mouse_down(
                gpui::MouseButton::Right,
                cx.listener(move |this, _, _window, cx| {
                    this.select_entry(menu_path.clone(), cx);
                }),
            )
            .into_any_element()
    }
}

/// Why `path` must not be deleted from the project panel, if it must not.
///
/// `main.rs` imports one view and opens it, and `menus` is named in both
/// `main.rs` and the menu bar it installs: deleting either leaves a project
/// that no longer compiles, which is a worse outcome than a refusal.
pub fn protected_entry(root: &std::path::Path, path: &std::path::Path) -> Option<String> {
    if path == root {
        return Some("the project root is not deleted from here".into());
    }
    if !path.starts_with(root) {
        return Some("this item is outside the project".into());
    }
    let relative = path.strip_prefix(root).ok()?.to_string_lossy().into_owned();
    let kept = [
        ("Cargo.toml", "Cargo.toml describes the project"),
        ("src/main.rs", "main.rs is the entry point"),
        ("src/ui/mod.rs", "ui/mod.rs declares the views"),
        ("src", "the src folder carries all the code"),
        ("src/ui", "the ui folder carries the views"),
    ];
    if let Some((_, reason)) = kept.iter().find(|(candidate, _)| *candidate == relative) {
        return Some(format!("deletion refused: {reason}"));
    }

    // Read from `main.rs` rather than assumed: the template's entry view is
    // called `home` today and was called something else yesterday, and a
    // project written by an older maxx — or renamed by hand — is exactly the
    // one this refusal has to protect.
    let entry = entry_view(root)?;
    (relative == format!("src/ui/{entry}.rs"))
        .then(|| format!("deletion refused: {entry} is the view main.rs opens"))
}

/// The view `src/main.rs` imports, when it imports one from `src/ui/`.
fn entry_view(root: &std::path::Path) -> Option<String> {
    let source = std::fs::read_to_string(root.join("src/main.rs")).ok()?;
    source.lines().find_map(|line| {
        let rest = line.trim().strip_prefix("use crate::ui::")?;
        let module = rest.split("::").next()?;
        let plausible = !module.is_empty()
            && module.chars().all(|character| character.is_alphanumeric() || character == '_')
            && !module.starts_with(|character: char| character.is_ascii_digit());
        plausible.then(|| module.to_string())
    })
}

/// The module name of `path` when it is a `src/<module>.rs` declared in
/// `main.rs`.
///
/// `main.rs` itself is not a module, and `src/ui/` has its own `mod.rs`.
pub fn top_level_module(root: &std::path::Path, path: &std::path::Path) -> Option<String> {
    let relative = path.strip_prefix(root).ok()?;
    let mut parts = relative.components().map(|part| part.as_os_str());
    if parts.next()? != "src" {
        return None;
    }
    let file = parts.next()?.to_string_lossy().into_owned();
    if parts.next().is_some() {
        return None;
    }
    let module = file.strip_suffix(".rs")?;
    (module != "main").then(|| module.to_string())
}

/// The module name of `path` when it is one of the project's views.
pub fn view_module(root: &std::path::Path, path: &std::path::Path) -> Option<String> {
    let relative = path.strip_prefix(root).ok()?;
    let mut parts = relative.components().map(|part| part.as_os_str());
    if parts.next()? != "src" || parts.next()? != "ui" {
        return None;
    }
    let file = parts.next()?.to_string_lossy().into_owned();
    if parts.next().is_some() {
        return None;
    }
    file.strip_suffix(".rs").map(|module| module.to_string())
}

/// Drops `pub mod <module>;` from `src/ui/mod.rs`.
///
/// Textual, the way `scaffold::create_view` adds it, so the rest of the file
/// keeps whatever the developer wrote in it.
pub fn unregister_view(root: &std::path::Path, module: &str) {
    let mod_path = root.join("src/ui/mod.rs");
    let Ok(source) = std::fs::read_to_string(&mod_path) else {
        return;
    };
    let declaration = format!("pub mod {module};");
    let kept: Vec<&str> = source.lines().filter(|line| line.trim() != declaration).collect();
    let mut out = kept.join("\n");
    if !out.is_empty() {
        out.push('\n');
    }
    let _ = std::fs::write(&mod_path, out);
}

/// A small clickable glyph in the project panel header.
fn panel_icon(
    id: &'static str,
    glyph: &'static str,
    // A translation key, not the text.
    tooltip: &'static str,
    cx: &mut Context<Workspace>,
    action: impl Fn(&mut Workspace, &mut Context<Workspace>) + 'static,
) -> impl IntoElement {
    div()
        .id(id)
        .flex()
        .items_center()
        .justify_center()
        .w(px(22.))
        .h(px(22.))
        .rounded_sm()
        .text_xs()
        .cursor_pointer()
        .text_color(theme::text_muted())
        .hover(|this| this.bg(theme::hover_bg()))
        .tooltip(move |window, cx| {
            gpui_component::tooltip::Tooltip::new(crate::tr(tooltip)).build(window, cx)
        })
        .child(glyph)
        .on_click(cx.listener(move |this, _, _window, cx| action(this, cx)))
}
