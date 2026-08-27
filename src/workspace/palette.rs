//! The palette: opening it, moving through it, running a line.
//!
//! Two lists, one box. `⌘K` fills it with the menu bar, flattened; `⌘P` fills
//! it with the project's files. Everything else — the keymap, the highlight,
//! the click, the way out — is written once and serves both, which is what
//! keeps them answering the same way.

use gpui::Focusable as _;

use super::*;

impl Workspace {
    /// Opens the command palette, or closes it when it is already open.
    pub fn toggle_palette(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        // The same key closes what it opened; the *other* key switches lists
        // rather than closing, which is what one means by pressing it.
        if self.command_input.is_some() && self.palette_mode == PaletteMode::Commands {
            self.close_palette(cx);
            return;
        }
        self.open_palette(crate::tr("palette.hint"), window, cx);
        self.palette_mode = PaletteMode::Commands;
        self.commands = crate::palette::commands(cx);
    }

    /// Opens the quick-open list, `⌘P`: the project's files, by name.
    ///
    /// The most used gesture of Zed, and the one maxx had nothing for: the tree
    /// is fine for looking around and hopeless for going somewhere known.
    pub fn quick_open(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.command_input.is_some() && self.palette_mode == PaletteMode::Files {
            self.close_palette(cx);
            return;
        }
        let Some(project) = self.project.as_ref() else {
            self.message = Some(crate::tr("message.no_project"));
            cx.notify();
            return;
        };
        // Walked at opening rather than held and refreshed: a list built on
        // every keystroke would walk the disk for nothing, and one kept between
        // openings would offer a file that has since been deleted.
        let (files, capped) = crate::project::walk_files(&project.root);
        self.open_palette(crate::tr("palette.file_hint"), window, cx);
        self.palette_mode = PaletteMode::Files;
        self.palette_files = files;
        // A list that stops without a word is a list that looks complete: the
        // file one is looking for may simply not be in it.
        if capped {
            self.message = Some(SharedString::from(
                t!("message.files_capped", count = crate::project::MAX_QUICK_OPEN_FILES)
                    .into_owned(),
            ));
        }
    }

    /// The half the two share: the box, its listeners, and the focus.
    fn open_palette(
        &mut self,
        placeholder: SharedString,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let input = cx.new(|cx| InputState::new(window, cx).placeholder(placeholder));
        // The box is rebuilt on every opening rather than emptied: a palette
        // that reopens on the last query would answer a question nobody asked
        // twice.
        cx.subscribe(&input, |this, _, event: &InputEvent, cx| match event {
            InputEvent::Change => {
                // The list shrinks under the cursor, so the cursor goes home.
                this.command_index = 0;
                cx.notify();
            }
            InputEvent::PressEnter { .. } => this.run_palette(cx),
            _ => {}
        })
        .detach();
        window.focus(&input.read(cx).focus_handle(cx));
        self.commands = Vec::new();
        self.palette_files = Vec::new();
        self.command_input = Some(input);
        self.command_index = 0;
        cx.notify();
    }

    /// Closes the palette without running anything.
    pub fn close_palette(&mut self, cx: &mut Context<Self>) {
        if self.command_input.take().is_some() {
            self.commands = Vec::new();
            self.palette_files = Vec::new();
            cx.notify();
        }
    }

    /// Moves the highlight one line down, or up.
    ///
    /// Stops at the ends rather than wrapping: a list that jumps from the last
    /// line to the first loses the reader's place for the sake of a gesture
    /// nobody was making.
    pub fn move_palette(&mut self, down: bool, cx: &mut Context<Self>) {
        let count = self.matching_lines(cx).len();
        if count == 0 {
            return;
        }
        self.command_index = if down {
            (self.command_index + 1).min(count - 1)
        } else {
            self.command_index.saturating_sub(1)
        };
        cx.notify();
    }

    /// Runs the highlighted command.
    ///
    /// Deferred, and the palette closed first: an action handler runs inside
    /// the window's own update, and several of maxx's commands open a window or
    /// borrow the workspace again — the same rule `defer_active` exists for.
    pub fn run_palette(&mut self, cx: &mut Context<Self>) {
        let matching = self.matching_lines(cx);
        let Some(position) = matching.get(self.command_index).copied() else {
            return;
        };

        // A file is opened here and now: there is no action to dispatch, and
        // nothing to defer — `select_file` is the workspace's own.
        if self.palette_mode == PaletteMode::Files
            && let Some(relative) = self.palette_files.get(position).cloned()
        {
            let root = self.project.as_ref().map(|project| project.root.clone());
            self.close_palette(cx);
            if let Some(root) = root {
                self.select_file(root.join(relative), cx);
            }
            return;
        }

        let Some(command) = self.commands.get(position) else {
            return;
        };
        let action = command.action.boxed_clone();
        self.close_palette(cx);
        cx.defer(move |cx: &mut App| {
            if let Some(handle) = cx.active_window() {
                let _ = handle.update(cx, |_, window, cx| window.dispatch_action(action, cx));
            }
        });
    }

    /// How many lines the palette draws at once.
    ///
    /// `⌘P` on a large project matches thousands of files, and an element per
    /// match is built on opening and again on every keystroke — the height of
    /// the box only clips them, it does not spare the work. A window that
    /// follows the highlight costs the same whatever the project holds.
    pub(crate) const PALETTE_WINDOW: usize = 60;

    /// The slice of the matching lines the palette draws, and where it starts.
    ///
    /// Around the highlight rather than from the top: a highlight moved past
    /// the window would otherwise be a highlight nobody can see.
    pub(crate) fn palette_window(&self, matching: &[usize]) -> (usize, Vec<usize>) {
        if matching.len() <= Self::PALETTE_WINDOW {
            return (0, matching.to_vec());
        }
        let half = Self::PALETTE_WINDOW / 2;
        let last = matching.len() - Self::PALETTE_WINDOW;
        let start = self.command_index.saturating_sub(half).min(last);
        (start, matching[start..start + Self::PALETTE_WINDOW].to_vec())
    }

    /// Where in the open list the lines the query keeps are.
    pub(crate) fn matching_lines(&self, cx: &App) -> Vec<usize> {
        let query = self
            .command_input
            .as_ref()
            .map(|input| input.read(cx).value().to_string())
            .unwrap_or_default();
        if self.palette_mode == PaletteMode::Commands {
            return crate::palette::matching(&self.commands, &query);
        }
        // The whole relative path is searched, not just the name: `ui home`
        // finds `src/ui/home.rs`, which is the question one actually has.
        let labels: Vec<String> =
            self.palette_files.iter().map(|path| path.to_string_lossy().into_owned()).collect();
        crate::palette::matching_labels(labels.iter().map(String::as_str), &query)
    }

    /// What the palette says when nothing answers the query.
    ///
    /// Two lists, two sentences: told there is "no command by that name" while
    /// looking for a file, one starts wondering what one typed.
    pub(crate) fn palette_nothing(&self) -> SharedString {
        match self.palette_mode {
            PaletteMode::Files => crate::tr("palette.no_file"),
            PaletteMode::Commands => crate::tr("palette.nothing"),
        }
    }

    /// The file at `position` of the quick-open list, when that is what is
    /// open.
    pub(crate) fn palette_file(&self, position: usize) -> Option<SharedString> {
        if self.palette_mode != PaletteMode::Files {
            return None;
        }
        self.palette_files
            .get(position)
            .map(|path| SharedString::from(path.to_string_lossy().into_owned()))
    }

    /// The command at `position` of the list the palette was opened on.
    pub(crate) fn command_at(&self, position: usize) -> Option<&crate::palette::Command> {
        self.commands.get(position)
    }

    /// The palette's box, while it is open.
    pub(crate) fn command_input(&self) -> Option<&Entity<InputState>> {
        self.command_input.as_ref()
    }

    /// Which line is highlighted.
    pub(crate) fn command_index(&self) -> usize {
        self.command_index
    }
}
