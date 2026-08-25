use gpui::{ClickEvent, Context, Entity, SharedString, Window, prelude::*};
use gpui_component::button::Button;
use gpui_component::checkbox::Checkbox;
use gpui_component::divider::Divider;
use gpui_component::group_box::GroupBox;
use gpui_component::input::{Input, InputState};
use gpui_component::label::Label;
use gpui_component::switch::Switch;
use gpui_component::{h_flex, v_flex};

pub struct Home {
    name: Entity<InputState>,
    summary: SharedString,
    openings: usize,
}

impl Home {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        Self {
            name: cx.new(|cx| InputState::new(window, cx).placeholder("Your name")),
            summary: "The inspector has not been opened yet.".into(),
            openings: 0,
        }
    }

    /// Opens the inspector, like the menu entry of the same name.
    ///
    /// maxx writes the empty handler and leaves you the body. Without
    /// `cx.notify()`, the counter would change without the screen moving —
    /// that is the classic slip.
    pub fn on_open(&mut self, _event: &ClickEvent, _window: &mut Window, cx: &mut Context<Self>) {
        self.openings += 1;
        self.summary = match self.openings {
            1 => "The inspector has been opened once.".into(),
            count => format!("The inspector has been opened {count} times.").into(),
        };
        cx.notify();
        crate::menus::open_inspector(cx);
    }
}

impl Render for Home {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        // maxx:begin
        v_flex()
            .gap_4()
            .p_4()
            .child(Label::new("A maxx demonstration"))
            .child(
                GroupBox::new().title("Components").child(
                    v_flex()
                        .gap_2()
                        .child(Label::new("Every element below comes from the catalogue."))
                        .child(Input::new(&self.name))
                        .child(
                            h_flex()
                                .gap_4()
                                .child(Checkbox::new("reread").label("Read back before writing"))
                                .child(Switch::new("watch").label("Watch the disk")),
                        )
                        .child(Divider::horizontal())
                        .child(Label::new(self.summary.clone())),
                ),
            )
            .child(
                h_flex()
                    .gap_2()
                    .child(
                        Button::new("open")
                            .label("Open the inspector")
                            .tooltip("A second window, the way the menu entry does it")
                            .on_click(cx.listener(Self::on_open)),
                    )
                    .child(Label::new("or ⌘I")),
            )
        // maxx:end
    }
}
