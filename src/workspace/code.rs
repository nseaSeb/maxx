//! The code reader: showing a file maxx does not know how to design.

use super::*;

/// Beyond this, the file is refused rather than shown.
///
/// tree-sitter parses the whole buffer on the frame the reader opens on, and a
/// multi-megabyte file freezes the window for as long as that takes. A refusal
/// is visible; a frozen window looks like a crash.
const MAX_BYTES: u64 = 2 * 1024 * 1024;

/// A file open in the code reader.
///
/// Read once, at opening, and never written: the reader is a window onto the
/// disk, not a second writer. The `.rs` files maxx designs already have a
/// source of truth — the canvas — and the rest belong to the editor.
pub struct CodeFile {
    /// Absolute path of the file being read.
    pub path: PathBuf,
    /// Its full text, as it was on opening.
    pub text: SharedString,
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
        if self.code.as_ref().is_some_and(|file| file.of_view) {
            self.code = None;
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
        self.preferences = false;
        self.menu_file = None;
        self.palette = None;
        self.code = Some(file);
        self.code_synced = None;
        self.code_revision = self.revision;
    }

    /// Brings the code reader back to the front, from a tab click.
    pub(crate) fn activate_code(&mut self, cx: &mut Context<Self>) {
        if self.code.is_none() {
            return;
        }
        if self.discard_menu_edits(cx) {
            return;
        }
        self.preferences = false;
        self.menu_file = None;
        self.palette = None;
        self.message = None;
        cx.notify();
    }

    /// Closes the code reader.
    pub(crate) fn close_code(&mut self, cx: &mut Context<Self>) {
        self.code = None;
        self.code_input = None;
        self.code_synced = None;
        cx.notify();
    }

    /// Drops the reader when the file it holds is `gone`.
    pub(super) fn forget_code(&mut self, gone: impl Fn(&std::path::Path) -> bool) {
        if self.code.as_ref().is_some_and(|file| gone(&file.path)) {
            self.code = None;
            self.code_input = None;
            self.code_synced = None;
        }
    }

    /// Renders the view's code again when the tree it comes from has moved.
    ///
    /// The canvas is not on screen while its code is, but `⌘Z`, `⌘⇧Z` and the
    /// node shortcuts still are: without this, an undo would leave a panel
    /// claiming to show what `⌘S` would write, one edit behind. Guarded by the
    /// revision because `render_source` runs `syn` and the code generator,
    /// which is not a thing to do on every frame.
    fn refresh_view_code(&mut self) {
        if !self.code.as_ref().is_some_and(|file| file.of_view) {
            return;
        }
        if self.code_revision == self.revision {
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
        let key = self.code.as_ref().map(|file| file.path.clone());
        if key == self.code_synced {
            return;
        }
        self.code_synced = key;

        let Some(file) = self.code.as_ref() else {
            self.code_input = None;
            return;
        };
        if file.image {
            self.code_input = None;
            return;
        }
        let language = file.language;
        let text = file.text.clone();
        self.code_input = Some(cx.new(|cx| {
            InputState::new(window, cx)
                .code_editor(language)
                .line_number(true)
                // Wrapping a line of code hides where it really ends; the
                // horizontal scroll says the truth about its width.
                .soft_wrap(false)
                .default_value(text)
        }));
    }

    /// The file being read, filling the main area.
    pub(crate) fn render_code(&self, _cx: &mut Context<Self>) -> AnyElement {
        if let Some(file) = self.code.as_ref().filter(|file| file.image) {
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
                        .with_fallback(crate::designer::missing_image),
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
                // `disabled` takes the writing away and leaves the reading:
                // arrows, ⌘A, ⌘C, mouse selection and the wheel are all
                // registered outside the `disabled` guard in gpui-component.
                // `appearance(false)` drops the grey wash it paints on a
                // disabled field — a reader is not a dead form control.
                Input::new(state).h_full().appearance(false).disabled(true),
            )
            .into_any_element()
    }
}
