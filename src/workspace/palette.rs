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
        self.command_input = Some(input);
        self.command_index = 0;
        cx.notify();
    }

    /// Closes the palette without running anything.
    pub fn close_palette(&mut self, cx: &mut Context<Self>) {
        if self.command_input.take().is_some() {
            cx.notify();
        }
    }

    /// Moves the highlight one line down, or up.
    ///
    /// Stops at the ends rather than wrapping: a list that jumps from the last
    /// line to the first loses the reader's place for the sake of a gesture
    /// nobody was making.
    pub fn move_palette(&mut self, down: bool, cx: &mut Context<Self>) {
        let count = self.palette_commands(cx).len();
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
        let mut commands = self.palette_commands(cx);
        if self.command_index >= commands.len() {
            return;
        }
        let action = commands.remove(self.command_index).action;
        self.close_palette(cx);
        cx.defer(move |cx: &mut App| {
            if let Some(handle) = cx.active_window() {
                let _ = handle.update(cx, |_, window, cx| window.dispatch_action(action, cx));
            }
        });
    }

    /// The commands the palette is showing, query applied.
    pub(crate) fn palette_commands(&self, cx: &App) -> Vec<crate::palette::Command> {
        let query = self
            .command_input
            .as_ref()
            .map(|input| input.read(cx).value().to_string())
            .unwrap_or_default();
        crate::palette::filter(crate::palette::commands(cx), &query)
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
