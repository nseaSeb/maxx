//! The code panel: showing, and editing, a file maxx does not know how to
//! design.

use super::*;

/// Beyond this, the file is refused rather than shown.
///
/// tree-sitter parses the whole buffer on the frame the reader opens on, and a
/// multi-megabyte file freezes the window for as long as that takes. A refusal
/// is visible; a frozen window looks like a crash.
const MAX_BYTES: u64 = 2 * 1024 * 1024;

/// A file open in the code panel.
///
/// Read at opening, and written by `⌘S` like anything else maxx holds. The
/// panel stays a small editor and not a rival to yours: no completion, no
/// refactoring, no search across the project — what it is for is the line one
/// does not want to change window for.
pub struct CodeFile {
    /// Absolute path of the file being read.
    pub path: PathBuf,
    /// Its full text, as it was on opening or at the last save.
    ///
    /// What the field holds is the live text; this is the copy the dirty mark
    /// and the disk check are answered from.
    pub text: SharedString,
    /// Whether the field holds something other than `text`.
    ///
    /// Set from the field's own event rather than derived on demand: the tab
    /// strip asks on every repaint, and reading the box would mean holding the
    /// application there.
    pub edited: bool,
    /// The grammar it is coloured with.
    pub language: &'static str,
    /// Whether this is the other side of the view being designed, rather than a
    /// file opened on its own.
    ///
    /// The two look the same and behave differently in the tab strip: a view
    /// seen as code is still the same open document — one tab, two ways of
    /// looking at it — whereas a file opened from the explorer gets a tab of
    /// its own.
    pub of_view: bool,
    /// Whether this file is shown as a picture rather than as text.
    ///
    /// An image has no text to colour and no field to build: the reader draws
    /// it, and the status bar names its weight rather than its lines.
    pub image: bool,
    /// Its size on disk, for the status bar of a picture.
    size: u64,
    /// Its name, and how many lines it holds.
    ///
    /// Counted here rather than in the status bar, which re-runs on every
    /// repaint: at the two-megabyte ceiling, `lines().count()` per frame is
    /// exactly the cost that ceiling exists to avoid.
    name: SharedString,
    lines: usize,
}

/// Fills in what is derived from the text, for both ways of building one.
fn from_text(path: PathBuf, text: String, language: &'static str, of_view: bool) -> CodeFile {
    let name = path.file_name().map(|name| name.to_string_lossy().into_owned()).unwrap_or_default();
    CodeFile {
        lines: text.lines().count(),
        name: SharedString::from(name),
        text: SharedString::from(text),
        path,
        language,
        of_view,
        edited: false,
        image: false,
        size: 0,
    }
}

/// The same, for a picture: nothing is read but its weight.
fn from_image(path: PathBuf, size: u64) -> CodeFile {
    let name = path.file_name().map(|name| name.to_string_lossy().into_owned()).unwrap_or_default();
    CodeFile {
        lines: 0,
        name: SharedString::from(name),
        text: SharedString::default(),
        path,
        language: "text",
        of_view: false,
        edited: false,
        image: true,
        size,
    }
}

impl CodeFile {
    /// Reads `path`, or says why it cannot be shown.
    pub fn load(path: &std::path::Path) -> Result<Self, String> {
        if path.is_dir() {
            return Err(crate::tr("error.not_a_file").to_string());
        }
        let size = std::fs::metadata(path).map(|data| data.len()).unwrap_or(0);
        // A picture is shown as a picture, and never read as text: the UTF-8
        // check below would refuse it, which is right for a binary and wrong
        // for the image the developer has just added to the project.
        if crate::project::is_image(path) {
            if size > crate::project::MAX_IMAGE_BYTES {
                return Err(t!("error.file_too_large", size = size / 1024).into_owned());
            }
            return Ok(from_image(path.to_path_buf(), size));
        }
        if size > MAX_BYTES {
            return Err(t!("error.file_too_large", size = size / 1024).into_owned());
        }
        // A binary file fails here, on the UTF-8 check, which is exactly the
        // test that matters: what cannot be decoded cannot be shown either.
        let text = std::fs::read_to_string(path)
            .map_err(|_| crate::tr("error.file_not_text").to_string())?;
        Ok(from_text(path.to_path_buf(), text, language_for(path), false))
    }

    /// The other side of a view being designed: the Rust `⌘S` would write.
    ///
    /// Not read from the disk, on purpose — the disk is one save behind, and a
    /// canvas and a code panel that disagree would make the reader useless
    /// exactly when it is most wanted.
    pub fn of_view(view: &crate::view::View) -> Result<Self, String> {
        Ok(from_text(view.path.clone(), view.render_source()?, "rust", true))
    }

    /// The file's name, for the tab and the status bar.
    pub fn name(&self) -> SharedString {
        self.name.clone()
    }

    /// How many lines it holds.
    pub fn lines(&self) -> usize {
        self.lines
    }

    /// Its weight in kilobytes, for a picture.
    pub fn kilobytes(&self) -> u64 {
        self.size / 1024
    }

    /// Writes `text` to the file, refusing when the file changed underneath.
    ///
    /// The same bargain a view is saved on: what maxx last read is compared
    /// with what is on disk, and a file written by someone else in the
    /// meantime is not overwritten without being asked. `force` is the answer
    /// to that question.
    ///
    /// A picture is refused outright: there is no text to write, and the field
    /// the panel would take it from does not exist.
    pub fn write(&mut self, text: &str, force: bool) -> Result<(), String> {
        if self.image {
            return Err(crate::tr("error.not_a_file").to_string());
        }
        if !force && self.disk_changed() {
            return Err(crate::tr("error.changed_on_disk").to_string());
        }
        std::fs::write(&self.path, text).map_err(|error| error.to_string())?;
        self.adopt(text);
        Ok(())
    }

    /// Takes `text` as what is now both on screen and on disk.
    pub fn adopt(&mut self, text: &str) {
        self.lines = text.lines().count();
        self.text = SharedString::from(text.to_string());
        self.edited = false;
    }

    /// Whether the file on disk differs from what maxx last read or wrote.
    ///
    /// Unreadable is not "changed", as for a view: refusing to save a file
    /// nobody can read would not help anyone.
    pub fn disk_changed(&self) -> bool {
        match std::fs::read_to_string(&self.path) {
            Ok(text) => text != self.text.as_ref(),
            Err(_) => false,
        }
    }
}

/// The grammar a file is coloured with, from its extension.
///
/// The names are those of `gpui_component::highlighter::Language`; `text` is
/// its no-op grammar, and the fallback for everything unlisted — a file with no
/// extension, a `LICENSE`, a `.gitignore`. Colouring those with the nearest
/// grammar would be inventing structure they do not have.
pub fn language_for(path: &std::path::Path) -> &'static str {
    let extension = path
        .extension()
        .map(|extension| extension.to_string_lossy().to_ascii_lowercase())
        .unwrap_or_default();
    match extension.as_str() {
        "rs" => "rust",
        "toml" | "lock" => "toml",
        "md" | "markdown" => "markdown",
        "json" => "json",
        "yml" | "yaml" => "yaml",
        "sh" | "bash" | "zsh" => "bash",
        "c" | "h" => "c",
        "cc" | "cpp" | "hpp" => "cpp",
        "cs" => "csharp",
        "css" => "css",
        "diff" | "patch" => "diff",
        "ex" | "exs" => "elixir",
        "go" => "go",
        "graphql" | "gql" => "graphql",
        "htm" | "html" => "html",
        "java" => "java",
        "js" | "mjs" | "cjs" => "javascript",
        "proto" => "proto",
        "py" => "python",
        "rb" => "ruby",
        "scala" | "sc" => "scala",
        "sql" => "sql",
        "swift" => "swift",
        "ts" => "typescript",
        "tsx" | "jsx" => "tsx",
        "zig" => "zig",
        _ => "text",
    }
}

impl Workspace {
    /// Shows `path` in the code reader, or says why it will not.
    ///
    /// Like the menu editor, the reader is a mode of the main area: opening it
    /// leaves the preferences and the menu editor, and the open views stay in
    /// the tab strip behind it.
    pub(crate) fn open_code(&mut self, path: PathBuf, cx: &mut Context<Self>) {
        // The context menu reaches here without passing through `select_file`,
        // so the menu editor's unsaved edits have to be defended here too.
        if self.discard_menu_edits(cx) {
            return;
        }
        // And the panel's own, now that it writes: opening another file drops
        // the field it holds.
        if self.discard_code_edits(cx) {
            return;
        }
        match CodeFile::load(&path) {
            Ok(file) => self.show_code(file),
            // The mode does not change: a refusal must not blank the area the
            // reader was not able to fill.
            Err(error) => self.message = Some(SharedString::from(error)),
        }
        self.selected = Some(path);
        cx.notify();
    }

    /// Flips the view being designed between its canvas and its code.
    ///
    /// The target is always the active view, never the file the reader happens
    /// to hold. A file opened from the explorer is therefore closed by the
    /// first press, the way clicking a view's tab already closes it: the reader
    /// holds one document, and there is nowhere to put a second.
    pub(crate) fn toggle_code(&mut self, cx: &mut Context<Self>) {
        if self.discard_menu_edits(cx) {
            return;
        }
        self.message = None;
        // Shown, not merely held. Asked the other way, `⌘E` was a one-shot: the
        // second press left the middle on the canvas with the document still
        // there, so the third took the same branch and did nothing — and the
        // read tab is only drawn for a file that is *not* a view's, so there
        // was nothing to click either.
        if self.showing_code() && self.code().is_some_and(|file| file.of_view) {
            if self.discard_code_edits(cx) {
                return;
            }
            self.show_designer();
            cx.notify();
            return;
        }
        // Held but covered: a view's code has no tab of its own, so `⌘E` is the
        // only way back to it — and rendering a fresh one here would drop what
        // was typed into the one already open.
        if self.code().is_some_and(|file| file.of_view && file.edited) {
            self.show(Center::Code);
            cx.notify();
            return;
        }
        let Some(view) = self.view() else {
            self.message = Some(crate::tr("designer.open_a_view"));
            cx.notify();
            return;
        };
        match CodeFile::of_view(view) {
            Ok(file) => self.show_code(file),
            // A comment inside the managed region: `render_source` refuses for
            // the same reason `save` does, and says so rather than showing a
            // rendering that would lose it.
            Err(error) => self.message = Some(SharedString::from(error)),
        }
        cx.notify();
    }

    /// Puts `file` in the reader, leaving whatever mode was up.
    ///
    /// The synchronisation key is cleared rather than compared: the same path
    /// can come back with different text — a view rendered again after an edit
    /// — and keying on the path alone would keep the stale field.
    fn show_code(&mut self, file: CodeFile) {
        // The context menu reaches the reader without passing through
        // `select_file`, which is where a stale message is normally dropped —
        // and the reader's status line yields to `message`, so one left behind
        // would hide the file that did open.
        self.message = None;
        self.code = Some(file);
        self.show(Center::Code);
        self.code_synced = None;
        self.code_revision = self.revision;
    }

    /// Brings the code reader back to the front, from a tab click.
    pub(crate) fn activate_code(&mut self, cx: &mut Context<Self>) {
        if self.code().is_none() {
            return;
        }
        if self.discard_menu_edits(cx) {
            return;
        }
        // Brings it forward — the file was already there, under whatever was
        // covering it. Written as `show_designer` this closed the reader
        // instead, which is the opposite of what the tab is for.
        self.show(Center::Code);
        self.message = None;
        cx.notify();
    }

    /// Closes the code reader.
    pub(crate) fn close_code(&mut self, cx: &mut Context<Self>) {
        if self.discard_code_edits(cx) {
            return;
        }
        self.code = None;
        self.show_designer();
        self.code_input = None;
        self.code_synced = None;
        cx.notify();
    }

    /// Drops the reader when the file it holds is `gone`.
    pub(super) fn forget_code(&mut self, gone: impl Fn(&std::path::Path) -> bool) {
        if self.code().is_some_and(|file| gone(&file.path)) {
            self.code = None;
            self.show_designer();
            self.code_input = None;
            self.code_synced = None;
        }
    }

    /// Whether the panel holds something not yet written.
    pub(crate) fn code_dirty(&self) -> bool {
        self.code().is_some_and(|file| file.edited)
    }

    /// Refuses a mode change that would drop what the panel holds.
    ///
    /// The same guard the menu editor has, for the same reason: leaving the
    /// panel throws its field away, and a file edited by hand has nowhere else
    /// to be. `⌘S` writes it, `Reload` throws it away on purpose.
    pub(super) fn discard_code_edits(&mut self, cx: &mut Context<Self>) -> bool {
        if self.code_dirty() {
            self.message = Some(crate::tr("message.code_unsaved"));
            cx.notify();
            return true;
        }
        false
    }

    /// Writes what the panel holds, refusing when the file changed underneath.
    ///
    /// Two files behind one panel. A file opened on its own is written as it
    /// stands. The other side of a view is parsed first and only then written,
    /// so the canvas follows the text — and a text that no longer reads leaves
    /// the file alone and says why.
    pub(super) fn save_code(&mut self, force: bool, cx: &mut Context<Self>) {
        let Some(state) = self.code_input.clone() else {
            return;
        };
        let text = state.read(cx).value().to_string();
        let of_view = self.code().is_some_and(|file| file.of_view);
        let path = match self.code() {
            Some(file) => file.path.clone(),
            None => return,
        };

        if of_view {
            let Some(view) = self.view_mut() else {
                return;
            };
            let name = view.name();
            let saved = match view.adopt_source(&text, force) {
                Ok(()) => {
                    // The tree has been replaced by the one this text parses
                    // to: a box open over the old one is typing into a node
                    // that may no longer exist, and the undo step it was going
                    // to record belongs to a document that is gone.
                    self.edit_snapshot = None;
                    self.canvas_edit = None;
                    self.conflicts.remove(&path);
                    self.message =
                        Some(SharedString::from(t!("message.saved", name = name).into_owned()));
                    self.revision += 1;
                    // Rebuilt here rather than left to `refresh_view_code`,
                    // which steps aside while the panel is dirty — and the
                    // panel is still dirty at this line, because what marks it
                    // clean is precisely this new document. What comes back is
                    // what `⌘S` would write, normalised, and no longer the raw
                    // text typed here.
                    self.rebuild_view_code();
                    // A picture asked for by name needs the assets module, and
                    // a view adopted from text can name one just as a canvas
                    // save can.
                    self.ensure_assets_module();
                    true
                }
                Err(error) => {
                    if error == crate::tr("error.changed_on_disk").as_ref() {
                        self.conflicts.insert(path.clone());
                    }
                    self.message = Some(SharedString::from(error));
                    false
                }
            };
            self.format_code_after_save(&path, saved, cx);
            cx.notify();
            return;
        }

        let Some(file) = self.code_mut() else {
            return;
        };
        let name = file.name();
        let saved = match file.write(&text, force) {
            Ok(()) => {
                self.conflicts.remove(&path);
                self.message =
                    Some(SharedString::from(t!("message.saved", name = name).into_owned()));
                true
            }
            Err(error) => {
                if error == crate::tr("error.changed_on_disk").as_ref() {
                    self.conflicts.insert(path.clone());
                }
                self.message = Some(SharedString::from(error));
                false
            }
        };
        self.format_code_after_save(&path, saved, cx);
        cx.notify();
    }

    /// Renders the view being designed into the panel again, clean.
    ///
    /// The one way an `of_view` panel goes from edited back to written: its
    /// text is not a file maxx reads back but a rendering of the tree, so
    /// "clean" means a rendering made after the tree moved.
    pub(super) fn rebuild_view_code(&mut self) {
        let Some(view) = self.view() else {
            return;
        };
        let Ok(file) = CodeFile::of_view(view) else {
            return;
        };
        self.code = Some(file);
        self.code_synced = None;
        self.code_revision = self.revision;
    }

    /// Runs the formatter over the file just written, when the preference asks
    /// for it and the file is Rust.
    ///
    /// `saved` and not "the panel was open": a write refused because the file
    /// changed underneath must not be followed by a formatter run over that
    /// other person's text, whose reread would then replace the edit the
    /// refusal was protecting.
    ///
    /// `rustfmt` is handed a path and nothing else, so a `Cargo.toml` or a
    /// `README.md` would come back untouched at best — the extension is the
    /// gate. What it reformats has to be read again, which costs the caret and
    /// the scroll position: the field is rebuilt around the new text.
    fn format_code_after_save(
        &mut self,
        path: &std::path::Path,
        saved: bool,
        cx: &mut Context<Self>,
    ) {
        if !saved || !crate::settings::prefs(cx).format_on_save {
            return;
        }
        if path.extension().is_none_or(|extension| extension != "rs") {
            return;
        }
        match crate::run::format_rust(path) {
            Ok(false) => {}
            Ok(true) => {
                // The view holds a copy of the text it wrote, and the formatter
                // has just changed the file under it: left alone, the next save
                // would see a file that "changed on disk" and accuse the person
                // typing of a conflict maxx made itself.
                if self.code().is_some_and(|file| file.of_view) {
                    if let Some(view) = self.view_mut()
                        && let Err(error) = view.reload()
                    {
                        self.message = Some(SharedString::from(error));
                        return;
                    }
                    self.revision += 1;
                    self.rebuild_view_code();
                } else {
                    self.reread_code();
                }
            }
            Err(error) => self.message = Some(SharedString::from(error)),
        }
    }

    /// Takes the file back from the disk, into the panel.
    ///
    /// For a view, the panel is rendered from the tree instead — the view
    /// itself was re-read by whoever called this.
    pub(super) fn reread_code(&mut self) -> bool {
        let Some(file) = self.code() else {
            return false;
        };
        if file.of_view {
            self.code_synced = None;
            return true;
        }
        // A file that has grown past the ceiling, or stopped being text, is not
        // re-read — and the panel is then one version behind, which is what the
        // caller has to be able to say rather than claim a reload.
        let Ok(reread) = CodeFile::load(&file.path) else {
            return false;
        };
        self.code = Some(reread);
        self.code_synced = None;
        true
    }

    /// Throws away what the panel holds and takes the file as it is on disk.
    pub(super) fn reload_code(&mut self, cx: &mut Context<Self>) {
        let Some(file) = self.code() else {
            return;
        };
        let path = file.path.clone();
        if file.of_view {
            if let Some(view) = self.view_mut()
                && let Err(error) = view.reload()
            {
                self.message = Some(SharedString::from(error));
                cx.notify();
                return;
            }
            self.edit_snapshot = None;
            self.canvas_edit = None;
            self.revision += 1;
            // Not `refresh_view_code`, which steps aside for a dirty panel:
            // this is the gesture that throws that edit away on purpose.
            self.rebuild_view_code();
        } else if !self.reread_code() {
            self.message = Some(crate::tr("error.file_not_text"));
            cx.notify();
            return;
        }
        self.conflicts.remove(&path);
        self.message = Some(crate::tr("message.code_reloaded"));
        cx.notify();
    }

    /// Renders the view's code again when the tree it comes from has moved.
    ///
    /// The canvas is not on screen while its code is, but `⌘Z`, `⌘⇧Z` and the
    /// node shortcuts still are: without this, an undo would leave a panel
    /// claiming to show what `⌘S` would write, one edit behind. Guarded by the
    /// revision because `render_source` runs `syn` and the code generator,
    /// which is not a thing to do on every frame.
    fn refresh_view_code(&mut self) {
        if !self.code().is_some_and(|file| file.of_view) {
            return;
        }
        if self.code_revision == self.revision {
            return;
        }
        // Not over something typed here and not yet written: an undo on the
        // canvas side, or a reload the watcher decided on, would otherwise
        // rebuild the field and take the edit with it.
        if self.code_dirty() {
            return;
        }
        self.code_revision = self.revision;
        let Some(view) = self.view() else {
            return;
        };
        if let Ok(file) = CodeFile::of_view(view) {
            self.code = Some(file);
            self.code_synced = None;
        }
    }

    /// Builds the reader's field, once per file rather than once per frame.
    ///
    /// Rebuilding it on every frame would lose the selection under the mouse,
    /// and re-parse the whole file each time; the guard is the same one the
    /// menu boxes use.
    pub(super) fn sync_code_input(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.refresh_view_code();
        let key = self.code().map(|file| file.path.clone());
        if key == self.code_synced {
            return;
        }
        self.code_synced = key;

        let Some(file) = self.code() else {
            self.code_input = None;
            return;
        };
        if file.image {
            self.code_input = None;
            return;
        }
        let language = file.language;
        let text = file.text.clone();
        let state = cx.new(|cx| {
            InputState::new(window, cx)
                .code_editor(language)
                .line_number(true)
                // Wrapping a line of code hides where it really ends; the
                // horizontal scroll says the truth about its width.
                .soft_wrap(false)
                .default_value(text)
        });
        // Compared with the text the panel opened on rather than flagged: a
        // line typed and then taken back leaves the tab clean, the way undoing
        // back to the saved tree does on the canvas.
        cx.subscribe(&state, move |this, state, event: &InputEvent, cx| {
            if !matches!(event, InputEvent::Change) {
                return;
            }
            let typed = state.read(cx).value().to_string();
            let Some(file) = this.code_mut() else {
                return;
            };
            let edited = typed != file.text.as_ref();
            if file.edited != edited {
                file.edited = edited;
                cx.notify();
            }
        })
        .detach();
        self.code_input = Some(state);
    }

    /// The file being read, filling the main area.
    pub(crate) fn render_code(&self, _cx: &mut Context<Self>) -> AnyElement {
        if let Some(file) = self.code().filter(|file| file.image) {
            return div()
                .flex()
                .flex_1()
                .items_center()
                .justify_center()
                .overflow_hidden()
                .p_6()
                .bg(theme::bg())
                .child(
                    gpui::img(file.path.clone())
                        .max_w_full()
                        .max_h_full()
                        // The reader shows a file and not a node, so there are no
                        // calls to size the frame with.
                        .with_fallback(|| crate::designer::missing_image(&[])),
                )
                .into_any_element();
        }
        let Some(state) = self.code_input.as_ref() else {
            return div().flex_1().into_any_element();
        };

        div()
            .flex()
            .flex_col()
            .flex_1()
            .overflow_hidden()
            .bg(theme::bg())
            .child(
                // Writable, and `appearance(false)` so it is a page of code and
                // not a form control: the panel fills the middle of the window,
                // where a border and a wash would draw a box around nothing.
                Input::new(state).h_full().appearance(false),
            )
            .into_any_element()
    }
}
