use gpui::{ClickEvent, Context, Entity, SharedString, Window, prelude::*};
use gpui_component::button::Button;
use gpui_component::checkbox::Checkbox;
use gpui_component::divider::Divider;
use gpui_component::group_box::GroupBox;
use gpui_component::input::{Input, InputState};
use gpui_component::label::Label;
use gpui_component::switch::Switch;
use gpui_component::{h_flex, v_flex};

pub struct Accueil {
    nom: Entity<InputState>,
    resume: SharedString,
    ouvertures: usize,
}

impl Accueil {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        Self {
            nom: cx.new(|cx| InputState::new(window, cx).placeholder("Votre nom")),
            resume: "L'inspecteur n'a pas encore été ouvert.".into(),
            ouvertures: 0,
        }
    }

    /// Ouvre l'inspecteur, comme l'entrée de menu du même nom.
    ///
    /// maxx pose le gestionnaire vide et vous laissez le corps. Sans
    /// `cx.notify()`, le compteur changerait sans que l'écran bouge — c'est
    /// l'oubli classique.
    pub fn on_ouvrir(
        &mut self,
        _event: &ClickEvent,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.ouvertures += 1;
        self.resume = match self.ouvertures {
            1 => "L'inspecteur a été ouvert une fois.".into(),
            count => format!("L'inspecteur a été ouvert {count} fois.").into(),
        };
        cx.notify();
        crate::menus::ouvrir_inspecteur(cx);
    }
}

impl Render for Accueil {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        // maxx:begin
        v_flex()
            .gap_4()
            .p_4()
            .child(Label::new("Démonstration de maxx"))
            .child(
                GroupBox::new()
                    .title("Composants")
                    .child(
                        v_flex()
                            .gap_2()
                            .child(Label::new("Chaque élément ci-dessous vient du catalogue."))
                            .child(Input::new(&self.nom))
                            .child(
                                h_flex()
                                    .gap_4()
                                    .child(
                                        Checkbox::new("relire")
                                            .label("Relire avant d'écrire"),
                                    )
                                    .child(
                                        Switch::new("veille")
                                            .label("Surveiller le disque"),
                                    ),
                            )
                            .child(Divider::horizontal())
                            .child(Label::new(self.resume.clone())),
                    ),
            )
            .child(
                h_flex()
                    .gap_2()
                    .child(
                        Button::new("ouvrir")
                            .label("Ouvrir l'inspecteur")
                            .tooltip("Une seconde fenêtre, comme le fait l'entrée de menu")
                            .on_click(cx.listener(Self::on_ouvrir)),
                    )
                    .child(Label::new("ou ⌘I")),
            )
        // maxx:end
    }
}
