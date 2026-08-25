//! The preferences screen.
//!
//! A mode of the workspace rather than a window of its own, the way the menu
//! editor is one: the tab strip stays, and `⌘W` is the way back. Zed makes the
//! same choice — settings open as a tab, not as a modal you have to dismiss
//! before you can look at anything else.
//!
//! Built on `gpui_component::setting`, whose `SettingField` takes a reader and
//! a writer over the application. That is exactly the shape of
//! [`crate::settings`], so no value is copied into this screen: a field reads
//! the settings when it draws and writes them when it is toggled. Nothing here
//! can drift from what is on disk.

use gpui::prelude::*;
use gpui::{AnyElement, Context, SharedString, div, px, rgb};
use gpui_component::setting::{
    SettingField, SettingGroup, SettingItem, SettingPage, Settings as SettingsView,
};

use crate::settings;
use crate::theme;
use crate::workspace::Workspace;

impl Workspace {
    /// The preferences screen, `⌘,`.
    pub(crate) fn render_preferences(&self, cx: &mut Context<Self>) -> AnyElement {
        SettingsView::new("preferences")
            .sidebar_width(px(200.))
            .page(appearance_page())
            .page(projects_page(cx))
            .page(file_page(cx))
            .into_any_element()
    }
}

/// What the window shows.
fn appearance_page() -> SettingPage {
    SettingPage::new("Apparence").group(
        SettingGroup::new()
            .title("Panneaux")
            .description("Ce que la fenêtre affiche autour du canvas.")
            .item(
                SettingItem::new(
                    "Panneau du projet",
                    SettingField::switch(
                        |cx| settings::get(cx).show_project_panel,
                        |value, cx| {
                            settings::update(cx, |settings| settings.show_project_panel = value);
                        },
                    ),
                )
                .description("L'explorateur, à gauche. ⌘B fait la même chose."),
            )
            .item(
                SettingItem::new(
                    "Barre d'état",
                    SettingField::switch(
                        |cx| settings::get(cx).show_status_bar,
                        |value, cx| {
                            settings::update(cx, |settings| settings.show_status_bar = value);
                        },
                    ),
                )
                .description("La ligne du bas : nom de la vue, messages, conflits."),
            )
            .item(
                SettingItem::new(
                    "Panneau de sortie",
                    SettingField::switch(
                        |cx| settings::get(cx).show_output,
                        |value, cx| {
                            settings::update(cx, |settings| settings.show_output = value);
                        },
                    ),
                )
                .description("Ce que `cargo` écrit pendant un lancement. ⌘J le bascule."),
            ),
    )
}

/// The recent projects, listed and clearable.
fn projects_page(cx: &mut Context<Workspace>) -> SettingPage {
    let recent = settings::get(cx).recent_projects.clone();
    let count = recent.len();

    SettingPage::new("Projets").group(
        SettingGroup::new()
            .title("Projets récents")
            .description(match count {
                0 => "Aucun projet ouvert pour l'instant.".to_string(),
                1 => "Un projet retenu. Les dix derniers sont gardés.".to_string(),
                _ => format!("{count} projets retenus. Les dix derniers sont gardés."),
            })
            .item(SettingItem::render(move |_, _, _| {
                let rows = recent.iter().map(|path| {
                    div()
                        .text_xs()
                        .text_color(rgb(theme::TEXT_MUTED))
                        .child(SharedString::from(path.to_string_lossy().into_owned()))
                });
                div().flex().flex_col().gap_1().children(rows)
            }))
            .item(SettingItem::render(|_, _, cx| {
                let empty = settings::get(cx).recent_projects.is_empty();
                action_button("prefs-clear-recent", "Vider la liste", empty, |cx| {
                    settings::update(cx, |settings| settings.recent_projects.clear());
                    cx.set_menus(crate::menus::app_menus(cx));
                })
            })),
    )
}

/// Where the settings live, and how to edit them by hand.
fn file_page(cx: &mut Context<Workspace>) -> SettingPage {
    let path = settings::Settings::path();
    let shown = path
        .as_ref()
        .map(|path| path.to_string_lossy().into_owned())
        .unwrap_or_else(|| "emplacement introuvable sur ce système".into());
    let _ = cx;

    SettingPage::new("Fichier").group(
        SettingGroup::new()
            .title("Fichier de réglages")
            .description(
                "Tout ce qui est ici s'édite aussi à la main. maxx réécrit le \
                 fichier entier quand il enregistre, donc les commentaires que \
                 vous y ajoutez disparaissent.",
            )
            .item(SettingItem::render(move |_, _, _| {
                div()
                    .text_xs()
                    .text_color(rgb(theme::TEXT_MUTED))
                    .child(SharedString::from(shown.clone()))
            }))
            .item(SettingItem::render(move |_, _, _| {
                let path = settings::Settings::path();
                action_button("prefs-open-file", "Ouvrir dans l'éditeur", path.is_none(), {
                    move |_| {
                        if let Some(path) = settings::Settings::path() {
                            // Written before opening: the file may not exist
                            // yet, and an editor opening on nothing is worse
                            // than an editor opening on the defaults.
                            let _ = settings::Settings::load().save();
                            crate::run::open_editor(&path);
                        }
                    }
                })
            })),
    )
}

/// A small button inside a settings group.
fn action_button(
    id: &'static str,
    label: &'static str,
    disabled: bool,
    action: impl Fn(&mut gpui::App) + 'static,
) -> impl IntoElement {
    div()
        .id(id)
        .px_3()
        .py_1()
        .rounded_sm()
        .text_xs()
        .bg(rgb(theme::BG))
        .when(disabled, |this| this.opacity(0.4))
        .when(!disabled, |this| {
            this.cursor_pointer()
                .hover(|this| this.bg(rgb(theme::HOVER_BG)))
                .on_click(move |_, _window, cx| action(cx))
        })
        .child(label)
}
