use gpui::{Context, Window, prelude::*};
use gpui_component::divider::Divider;
use gpui_component::group_box::GroupBox;
use gpui_component::label::Label;
use gpui_component::v_flex;

/// The view the menu entry opens in a window of its own.
pub struct Inspector {}

impl Inspector {
    pub fn new(_window: &mut Window, _cx: &mut Context<Self>) -> Self {
        Self {}
    }
}

impl Render for Inspector {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        // maxx:begin
        v_flex()
            .gap_4()
            .p_4()
            .child(Label::new("Inspector"))
            .child(Divider::horizontal())
            .child(
                GroupBox::new()
                    .title("What this window shows")
                    .child(
                        v_flex()
                            .gap_2()
                            .child(Label::new("A second window, opened from the menu bar."))
                            .child(Label::new("It is rooted in gpui_component::Root, without which the smallest component aborts the process."))
                            .child(Label::new("⌘W closes it.")),
                    ),
            )
        // maxx:end
    }
}
