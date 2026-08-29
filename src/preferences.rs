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

use rust_i18n::t;

use gpui::prelude::*;
use gpui::{AnyElement, Context, SharedString, div, px};
use gpui_component::setting::{
    SettingField, SettingGroup, SettingItem, SettingPage, Settings as SettingsView,
};

use gpui::Entity;
use gpui_component::Sizable;
use gpui_component::input::{Input, InputState};

use crate::projectfile::RunField;
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
            .page(run_page(self))
            .page(file_page(cx))
            .into_any_element()
    }
}

/// What the window shows.
fn appearance_page() -> SettingPage {
    SettingPage::new(crate::tr("prefs.appearance"))
        .group(
            SettingGroup::new()
                .title(crate::tr("prefs.panels"))
                .description(crate::tr("prefs.panels_desc"))
                .item(
                    SettingItem::new(
                        crate::tr("prefs.project_panel"),
                        SettingField::switch(
                            |cx| settings::prefs(cx).show_project_panel,
                            |value, cx| {
                                settings::update_prefs(cx, |prefs| {
                                    prefs.show_project_panel = value
                                });
                            },
                        ),
                    )
                    .description(crate::tr("prefs.project_panel_desc")),
                )
                .item(
                    SettingItem::new(
                        crate::tr("prefs.status_bar"),
                        SettingField::switch(
                            |cx| settings::prefs(cx).show_status_bar,
                            |value, cx| {
                                settings::update_prefs(cx, |prefs| prefs.show_status_bar = value);
                            },
                        ),
                    )
                    .description(crate::tr("prefs.status_bar_desc")),
                )
                .item(
                    SettingItem::new(
                        crate::tr("prefs.output_panel"),
                        SettingField::switch(
                            |cx| settings::prefs(cx).show_output,
                            |value, cx| {
                                settings::update_prefs(cx, |prefs| prefs.show_output = value);
                            },
                        ),
                    )
                    .description(crate::tr("prefs.output_panel_desc")),
                ),
        )
        .group(
            SettingGroup::new().title(crate::tr("prefs.theme")).item(
                SettingItem::new(
                    crate::tr("prefs.theme"),
                    SettingField::dropdown(
                        vec![
                            (SharedString::from("system"), crate::tr("prefs.theme_system")),
                            (SharedString::from("light"), crate::tr("prefs.theme_light")),
                            (SharedString::from("dark"), crate::tr("prefs.theme_dark")),
                        ],
                        |cx| SharedString::from(settings::prefs(cx).theme.clone()),
                        |value, cx| {
                            settings::update_prefs(cx, |prefs| prefs.theme = value.to_string());
                            crate::apply_theme(cx);
                            crate::workspace::notify_all(cx);
                        },
                    ),
                )
                .description(crate::tr("prefs.theme_desc")),
            ),
        )
        .group(
            SettingGroup::new().title(crate::tr("prefs.language")).item(
                SettingItem::new(
                    crate::tr("prefs.language"),
                    SettingField::dropdown(
                        language_options(),
                        |cx| SharedString::from(settings::prefs(cx).language.clone()),
                        |value, cx| {
                            settings::update_prefs(cx, |prefs| prefs.language = value.to_string());
                            crate::apply_locale(cx);
                            // Everything translates at render time but the
                            // native menu bar: gpui takes it once and keeps it.
                            cx.set_menus(crate::menus::app_menus(cx));
                            crate::workspace::notify_all(cx);
                        },
                    ),
                )
                // Right away, without restarting: the catalogue's and the
                // inspector's labels are keys resolved at render time, and the
                // native menu bar is the only piece to put back by hand.
                .description(crate::tr("prefs.language_desc")),
            ),
        )
}

/// The languages offered, each named in itself.
///
/// A language names itself: someone looking for French is looking for
/// "Français", not for "French" written in a language they do not read.
fn language_options() -> Vec<(SharedString, SharedString)> {
    let mut options = vec![(SharedString::from("system"), crate::tr("prefs.language_system"))];
    let mut codes: Vec<&str> = rust_i18n::available_locales!();
    codes.sort_unstable();
    options.extend(codes.into_iter().map(|code| {
        let name = match code {
            "en" => "English",
            "fr" => "Français",
            other => other,
        };
        (SharedString::from(code), SharedString::from(name))
    }));
    options
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

    SettingPage::new(crate::tr("prefs.tools")).group(
        SettingGroup::new()
            .title(crate::tr("prefs.editor_terminal"))
            .description(crate::tr(if bound {
                "prefs.installed_only_bound"
            } else {
                "prefs.installed_only"
            }))
            .item(
                SettingItem::new(
                    crate::tr("prefs.editor"),
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
                .description(crate::tr("prefs.editor_desc")),
            )
            .item(
                SettingItem::new(
                    crate::tr("prefs.terminal"),
                    SettingField::dropdown(
                        terminals,
                        |cx| SharedString::from(settings::prefs(cx).terminal.clone()),
                        |value, cx| {
                            settings::update_prefs(cx, |prefs| prefs.terminal = value.to_string());
                        },
                    ),
                )
                .description(crate::tr("prefs.terminal_desc")),
            )
            .item(
                SettingItem::new(
                    crate::tr("prefs.format_on_save"),
                    SettingField::switch(
                        |cx| settings::prefs(cx).format_on_save,
                        |value, cx| {
                            settings::update_prefs(cx, |prefs| prefs.format_on_save = value);
                        },
                    ),
                )
                .description(crate::tr("prefs.format_on_save_desc")),
            ),
    )
}

/// The recent projects, listed and clearable.
fn projects_page(cx: &mut Context<Workspace>) -> SettingPage {
    let recent = settings::state(cx).recent_projects.clone();
    let count = recent.len();

    SettingPage::new(crate::tr("prefs.projects")).group(
        SettingGroup::new()
            .title(crate::tr("prefs.recent"))
            .description(match count {
                0 => crate::tr("prefs.recent_none").to_string(),
                1 => crate::tr("prefs.recent_one").to_string(),
                _ => t!("prefs.recent_many", count = count).into_owned(),
            })
            .item(SettingItem::render(move |_, _, _| {
                let rows = recent.iter().map(|path| {
                    div()
                        .text_xs()
                        .text_color(theme::text_muted())
                        .child(SharedString::from(path.to_string_lossy().into_owned()))
                });
                div().flex().flex_col().gap_1().children(rows)
            }))
            .item(SettingItem::render(|_, _, cx| {
                let empty = settings::state(cx).recent_projects.is_empty();
                action_button("prefs-clear-recent", "prefs.clear_recent", empty, |cx| {
                    settings::update_state(cx, |state| state.recent_projects.clear());
                    cx.set_menus(crate::menus::app_menus(cx));
                })
            })),
    )
}

/// How the open project is launched: the `[run]` section of its `maxx.toml`.
///
/// A page of this screen rather than a screen of its own, and the distinction
/// it has to carry is stated rather than drawn: these are the project's
/// settings, not the user's, and the group says which file they go to. The
/// alternative was a second sidebar and a second set of pages for four fields.
///
/// The boxes are built by `sync_run_inputs` and only borrowed here: an entity
/// created in a render closure is a new one per frame, and a field rebuilt
/// under the caret loses what is being typed into it.
fn run_page(workspace: &Workspace) -> SettingPage {
    let Some(project) = workspace.project() else {
        return SettingPage::new(crate::tr("prefs.run")).group(
            SettingGroup::new()
                .title(crate::tr("prefs.run_none"))
                .description(crate::tr("prefs.run_none_desc")),
        );
    };

    let name = project.name.clone();
    let config = workspace.run_config().clone();
    let inputs: Vec<_> = workspace.run_inputs().to_vec();
    let boxed = |field: RunField| {
        inputs.iter().find(|(this, _)| *this == field).map(|(_, state)| state.clone())
    };
    let (profile, features, args) =
        (boxed(RunField::Profile), boxed(RunField::Features), boxed(RunField::Args));
    let line = format!("$ cargo {}", config.arguments("run").join(" "));

    SettingPage::new(crate::tr("prefs.run"))
        .group(
            SettingGroup::new()
                .title(name)
                .description(crate::tr("prefs.run_desc"))
                .item(field_row("prefs.run_profile", "prefs.run_profile_desc", profile))
                .item(field_row("prefs.run_features", "prefs.run_features_desc", features))
                .item(
                    SettingItem::new(
                        crate::tr("prefs.run_default_features"),
                        SettingField::switch(
                            {
                                let on = config.default_features;
                                move |_| on
                            },
                            |value, cx| {
                                crate::workspace::defer_active(cx, move |workspace, _, cx| {
                                    workspace.toggle_default_features(value, cx);
                                });
                            },
                        ),
                    )
                    .description(crate::tr("prefs.run_default_features_desc")),
                )
                .item(field_row("prefs.run_args", "prefs.run_args_desc", args)),
        )
        .group(
            SettingGroup::new()
                .title(crate::tr("prefs.run_command"))
                .description(crate::tr("prefs.run_command_desc"))
                .item(SettingItem::render(move |_, _, _| {
                    div()
                        .text_xs()
                        .text_color(theme::text_muted())
                        .child(SharedString::from(line.clone()))
                })),
        )
}

/// One labelled text box of the run page.
fn field_row(label: &str, description: &str, state: Option<Entity<InputState>>) -> SettingItem {
    let label = crate::tr(label);
    let description = crate::tr(description);
    SettingItem::render(move |_, _, _| {
        let field: AnyElement = match &state {
            Some(state) => Input::new(state).small().into_any_element(),
            None => div().into_any_element(),
        };
        div()
            .flex()
            .flex_col()
            .gap_1()
            .child(div().child(label.clone()))
            .child(div().text_xs().text_color(theme::text_muted()).child(description.clone()))
            .child(field)
    })
}

/// Where the settings live, and how to edit them by hand.
fn file_page(cx: &mut Context<Workspace>) -> SettingPage {
    let path = settings::settings_path();
    let shown = path
        .as_ref()
        .map(|path| path.to_string_lossy().into_owned())
        .unwrap_or_else(|| crate::tr("prefs.no_location").to_string());
    let _ = cx;

    SettingPage::new(crate::tr("prefs.file")).group(
        SettingGroup::new()
            .title(crate::tr("prefs.settings_file"))
            .description(crate::tr("prefs.settings_file_desc"))
            .item(SettingItem::render(move |_, _, _| {
                div()
                    .text_xs()
                    .text_color(theme::text_muted())
                    .child(SharedString::from(shown.clone()))
            }))
            .item(SettingItem::render(move |_, _, _| {
                let path = settings::settings_path();
                action_button("prefs-open-file", "prefs.open_in_editor", path.is_none(), {
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
/// A button of the preferences screen, labelled from its translation key.
fn action_button(
    id: &'static str,
    key: &'static str,
    disabled: bool,
    action: impl Fn(&mut gpui::App) + 'static,
) -> impl IntoElement {
    div()
        .id(id)
        .px_3()
        .py_1()
        .rounded_sm()
        .text_xs()
        .bg(theme::bg())
        .when(disabled, |this| this.opacity(0.4))
        .when(!disabled, |this| {
            this.cursor_pointer()
                .hover(|this| this.bg(theme::hover_bg()))
                .on_click(move |_, _window, cx| action(cx))
        })
        .child(crate::tr(key))
}
