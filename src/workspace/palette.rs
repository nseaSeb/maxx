//! The command palette: opening it, moving through it, running a line.

use gpui::Focusable as _;

use super::*;

impl Workspace {
    /// Opens the command palette, or closes it when it is already open.
    pub fn toggle_palette(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.command_input.is_some() {
            self.close_palette(cx);
            return;
        }
        let input = cx.new(|cx| InputState::new(window, cx).placeholder(crate::tr("palette.hint")));
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
        self.commands = crate::palette::commands(cx);
        self.command_input = Some(input);
        self.command_index = 0;
        cx.notify();
    }

    /// Closes the palette without running anything.
    pub fn close_palette(&mut self, cx: &mut Context<Self>) {
        if self.command_input.take().is_some() {
            self.commands = Vec::new();
            cx.notify();
        }
    }

    /// Moves the highlight one line down, or up.
    ///
    /// Stops at the ends rather than wrapping: a list that jumps from the last
    /// line to the first loses the reader's place for the sake of a gesture
    /// nobody was making.
    pub fn move_palette(&mut self, down: bool, cx: &mut Context<Self>) {
        let count = self.matching_commands(cx).len();
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
        let matching = self.matching_commands(cx);
        let Some(position) = matching.get(self.command_index) else {
            return;
        };
        let action = self.commands[*position].action.boxed_clone();
        self.close_palette(cx);
        cx.defer(move |cx: &mut App| {
            if let Some(handle) = cx.active_window() {
                let _ = handle.update(cx, |_, window, cx| window.dispatch_action(action, cx));
            }
        });
    }

    /// Where in [`Self::commands`] the lines the query keeps are.
    pub(crate) fn matching_commands(&self, cx: &App) -> Vec<usize> {
        let query = self
            .command_input
            .as_ref()
            .map(|input| input.read(cx).value().to_string())
            .unwrap_or_default();
        crate::palette::matching(&self.commands, &query)
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
