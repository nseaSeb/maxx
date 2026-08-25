use gpui::{Context, Window, prelude::*};
use gpui_component::divider::Divider;
use gpui_component::group_box::GroupBox;
use gpui_component::label::Label;
use gpui_component::v_flex;

/// La vue que l'entrée de menu ouvre dans sa propre fenêtre.
pub struct Inspecteur {}

impl Inspecteur {
    pub fn new(_window: &mut Window, _cx: &mut Context<Self>) -> Self {
        Self {}
    }
}

impl Render for Inspecteur {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        // maxx:begin
        v_flex()
            .gap_4()
            .p_4()
            .child(Label::new("Inspecteur"))
            .child(Divider::horizontal())
            .child(
                GroupBox::new()
                    .title("Ce que montre cette fenêtre")
                    .child(
                        v_flex()
                            .gap_2()
                            .child(Label::new("Une seconde fenêtre, ouverte depuis la barre de menus."))
                            .child(Label::new("Elle est enracinée dans gpui_component::Root, sans quoi le moindre composant interrompt le processus."))
                            .child(Label::new("⌘W la referme.")),
                    ),
            )
        // maxx:end
    }
}
