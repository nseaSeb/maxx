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
            .page(tools_page())
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
                        |cx| settings::prefs(cx).show_project_panel,
                        |value, cx| {
                            settings::update_prefs(cx, |prefs| prefs.show_project_panel = value);
                        },
                    ),
                )
                .description("L'explorateur, à gauche. ⌘B fait la même chose."),
            )
            .item(
                SettingItem::new(
                    "Barre d'état",
                    SettingField::switch(
                        |cx| settings::prefs(cx).show_status_bar,
                        |value, cx| {
                            settings::update_prefs(cx, |prefs| prefs.show_status_bar = value);
                        },
                    ),
                )
                .description("La ligne du bas : nom de la vue, messages, conflits."),
            )
            .item(
                SettingItem::new(
                    "Panneau de sortie",
                    SettingField::switch(
                        |cx| settings::prefs(cx).show_output,
                        |value, cx| {
                            settings::update_prefs(cx, |prefs| prefs.show_output = value);
                        },
                    ),
                )
                .description("Ce que `cargo` écrit pendant un lancement. ⌘J le bascule."),
            ),
    )
}

/// Which editor and which terminal maxx hands things to.
fn tools_page() -> SettingPage {
    let editors: Vec<(SharedString, SharedString)> = crate::tools::editor_options()
        .into_iter()
        .map(|(value, label)| (SharedString::from(value), SharedString::from(label)))
        .collect();
    let terminals: Vec<(SharedString, SharedString)> = crate::tools::terminal_options()
        .into_iter()
        .map(|(value, label)| (SharedString::from(value), SharedString::from(label)))
        .collect();

    // A terminal editor is driven through the chosen terminal, so the two
    // choices are not independent — worth saying rather than leaving to be
    // discovered on a click that does nothing.
    let bound =
        crate::tools::EDITORS.iter().any(|editor| editor.terminal_bound && editor.installed());

    SettingPage::new("Outils").group(
        SettingGroup::new()
            .title("Éditeur et terminal")
            .description(if bound {
                "Seul ce qui est installé est proposé. Un éditeur de terminal — \
                 Helix, Neovim, Vim — est lancé dans le terminal choisi ci-dessous, \
                 qui doit donc savoir recevoir une commande."
            } else {
                "Seul ce qui est installé est proposé."
            })
            .item(
                SettingItem::new(
                    "Éditeur",
                    SettingField::dropdown(
                        editors,
                        |cx| SharedString::from(settings::prefs(cx).editor.clone()),
                        |value, cx| {
                            settings::update_prefs(cx, |prefs| prefs.editor = value.to_string());
                            // The menu bar names the editor, so it has to be
                            // handed over again.
                            cx.set_menus(crate::menus::app_menus(cx));
                            crate::workspace::notify_all(cx);
                        },
                    ),
                )
                .description("Ce qu'ouvrent ⌘⌥Z et le bouton → de l'inspecteur."),
            )
            .item(
                SettingItem::new(
                    "Terminal",
                    SettingField::dropdown(
                        terminals,
                        |cx| SharedString::from(settings::prefs(cx).terminal.clone()),
                        |value, cx| {
                            settings::update_prefs(cx, |prefs| prefs.terminal = value.to_string());
                        },
                    ),
                )
                .description("Ce qu'ouvre ⌘⌥T, sur le dossier du projet."),
            )
            .item(
                SettingItem::new(
                    "Mettre en forme à l'enregistrement",
                    SettingField::switch(
                        |cx| settings::prefs(cx).format_on_save,
                        |value, cx| {
                            settings::update_prefs(cx, |prefs| prefs.format_on_save = value);
                        },
                    ),
                )
                .description(
                    "Passe rustfmt sur le fichier après chaque écriture, pour que ce que \
                     maxx écrit suive les conventions du projet — son rustfmt.toml compris. \
                     Attention : rustfmt met en forme le fichier entier, pas seulement la \
                     zone que maxx gère. Sur un projet déjà formaté, cela ne change rien \
                     ailleurs.",
                ),
            ),
    )
}

/// The recent projects, listed and clearable.
fn projects_page(cx: &mut Context<Workspace>) -> SettingPage {
    let recent = settings::state(cx).recent_projects.clone();
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
                let empty = settings::state(cx).recent_projects.is_empty();
                action_button("prefs-clear-recent", "Vider la liste", empty, |cx| {
                    settings::update_state(cx, |state| state.recent_projects.clear());
                    cx.set_menus(crate::menus::app_menus(cx));
                })
            })),
    )
}

/// Where the settings live, and how to edit them by hand.
fn file_page(cx: &mut Context<Workspace>) -> SettingPage {
    let path = settings::settings_path();
    let shown = path
        .as_ref()
        .map(|path| path.to_string_lossy().into_owned())
        .unwrap_or_else(|| "emplacement introuvable sur ce système".into());
    let _ = cx;

    SettingPage::new("Fichier").group(
        SettingGroup::new()
            .title("Fichier de réglages")
            .description(
                "Tout ce qui est ici s'édite aussi à la main, commentaires \
                 compris : maxx ne réécrit que la clé qu'il change. Les projets \
                 récents et la position de la fenêtre vivent à côté, dans \
                 state.json, que maxx tient seul.",
            )
            .item(SettingItem::render(move |_, _, _| {
                div()
                    .text_xs()
                    .text_color(rgb(theme::TEXT_MUTED))
                    .child(SharedString::from(shown.clone()))
            }))
            .item(SettingItem::render(move |_, _, _| {
                let path = settings::settings_path();
                action_button("prefs-open-file", "Ouvrir dans l'éditeur", path.is_none(), {
                    move |cx| {
                        if let Some(path) = settings::settings_path() {
                            crate::tools::open_in_editor(cx, &path, None);
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
