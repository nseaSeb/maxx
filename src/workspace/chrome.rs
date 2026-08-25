//! The window's own furniture: titlebar, welcome screen, status bar, [`Render`].

use super::*;

impl Workspace {
    /// Toggles the project panel (View > Project Panel, `cmd-b`).
    pub fn toggle_project_panel(&mut self, cx: &mut Context<Self>) {
        crate::settings::update_prefs(cx, |preferences| {
            preferences.show_project_panel = !preferences.show_project_panel;
        });
        notify_all(cx);
    }

    /// Toggles the status bar (View > Status Bar).
    pub fn toggle_status_bar(&mut self, cx: &mut Context<Self>) {
        crate::settings::update_prefs(cx, |preferences| {
            preferences.show_status_bar = !preferences.show_status_bar;
        });
        notify_all(cx);
    }

    /// Shows the preferences, or leaves them when they are already up.
    ///
    /// A toggle rather than an open: `⌘,` pressed twice is how you check a
    /// setting and go straight back to what you were drawing.
    pub fn toggle_preferences(&mut self, cx: &mut Context<Self>) {
        self.preferences = !self.preferences;
        cx.notify();
    }

    /// Leaves the preferences screen.
    pub fn close_preferences(&mut self, cx: &mut Context<Self>) {
        if self.preferences {
            self.preferences = false;
            cx.notify();
        }
    }

    fn render_titlebar(&self) -> impl IntoElement {
        let (title, subtitle) = match &self.project {
            Some(project) => (
                project.name.clone(),
                SharedString::from(project.root.to_string_lossy().into_owned()),
            ),
            None => (SharedString::from("maxx"), SharedString::from("")),
        };

        div()
            .flex()
            .items_center()
            .h(px(32.))
            // The system titlebar is transparent, so the traffic lights are
            // drawn on top of this row: keep their corner clear.
            .pl(px(80.))
            .pr(px(12.))
            .gap_2()
            .bg(rgb(theme::TITLEBAR_BG))
            .border_b_1()
            .border_color(rgb(theme::BORDER))
            .child(
                div()
                    .flex()
                    .flex_1()
                    .items_center()
                    .justify_center()
                    .gap_2()
                    .child(div().child(title))
                    .child(div().text_xs().text_color(rgb(theme::TEXT_MUTED)).child(subtitle)),
            )
    }

    fn render_main(&self, cx: &mut Context<Self>) -> AnyElement {
        // Before the welcome screen: the preferences must be reachable when no
        // project is open, which is exactly when someone is setting maxx up.
        if self.preferences {
            return self.render_designer(cx);
        }
        if self.project.is_none() {
            return self.render_welcome(cx);
        }
        self.render_designer(cx)
    }

    fn render_welcome(&self, cx: &mut Context<Self>) -> AnyElement {
        div()
            .flex()
            .flex_1()
            .flex_col()
            .items_center()
            .justify_center()
            .gap_4()
            .child(div().text_2xl().child("maxx"))
            .child(
                div().text_color(rgb(theme::TEXT_MUTED)).child("Ouvrez un dossier pour commencer."),
            )
            .child(
                div()
                    .id("welcome-open-folder")
                    .px_4()
                    .py_2()
                    .rounded_md()
                    .cursor_pointer()
                    .bg(rgb(theme::ACCENT))
                    .text_color(rgb(theme::ON_ACCENT))
                    .hover(|this| this.opacity(0.85))
                    .child("Ouvrir un dossier…")
                    .on_click(cx.listener(|_, _, window, cx| {
                        window.dispatch_action(Box::new(OpenFolder), cx);
                    })),
            )
            .child(div().text_xs().text_color(rgb(theme::TEXT_MUTED)).child("⌘O"))
            .children(self.render_recent_projects(cx))
            .into_any_element()
    }

    /// The recent projects, on the welcome screen.
    ///
    /// The same list as the one in the File menu, put where someone who has
    /// just launched maxx is already looking.
    fn render_recent_projects(&self, cx: &mut Context<Self>) -> Option<AnyElement> {
        let recent = crate::settings::state(cx).recent_projects.clone();
        if recent.is_empty() {
            return None;
        }

        let rows = recent.into_iter().enumerate().map(|(index, path)| {
            let name = path
                .file_name()
                .map(|name| name.to_string_lossy().into_owned())
                .unwrap_or_else(|| path.to_string_lossy().into_owned());
            let parent = path
                .parent()
                .map(|parent| parent.to_string_lossy().into_owned())
                .unwrap_or_default();

            div()
                .id(SharedString::from(format!("recent-{index}")))
                .flex()
                .items_baseline()
                .gap_2()
                .px_3()
                .py_1()
                .rounded_sm()
                .cursor_pointer()
                .hover(|this| this.bg(rgb(theme::HOVER_BG)))
                .child(div().child(name))
                .child(div().text_xs().text_color(rgb(theme::TEXT_MUTED)).child(parent))
                .on_click(cx.listener(move |_, _, window, cx| {
                    window.dispatch_action(Box::new(crate::actions::OpenRecent { index }), cx);
                }))
        });

        Some(
            div()
                .flex()
                .flex_col()
                .mt_4()
                .gap_1()
                .items_start()
                .child(
                    div()
                        .px_3()
                        .text_xs()
                        .text_color(rgb(theme::TEXT_MUTED))
                        .child("Projets récents"),
                )
                .children(rows)
                .into_any_element(),
        )
    }

    fn render_status_bar(&self) -> impl IntoElement {
        let conflict = self.view().is_some_and(|view| self.conflicts.contains(&view.path));
        if let Some(menus) = self.menu_file.as_ref() {
            let label = match &self.message {
                Some(message) => message.clone(),
                None => SharedString::from(format!(
                    "{}{} · {} menus",
                    menus.name(),
                    if menus.dirty() { " •" } else { "" },
                    menus.menus.len()
                )),
            };
            return div()
                .flex()
                .items_center()
                .h(px(24.))
                .px_3()
                .flex_none()
                .bg(rgb(theme::PANEL_BG))
                .border_t_1()
                .border_color(rgb(theme::BORDER))
                .text_xs()
                .text_color(rgb(theme::TEXT_MUTED))
                .child(label);
        }
        let label = match (&self.message, &self.view(), &self.project) {
            (Some(message), _, _) => message.clone(),
            (None, Some(view), _) => SharedString::from(format!(
                "{}{}{} · {} nœuds",
                view.name(),
                if view.dirty() { " •" } else { "" },
                if conflict { " ⚠ modifié en dehors de maxx" } else { "" },
                view.root.count()
            )),
            (None, None, Some(project)) => {
                SharedString::from(format!("{} · {} éléments", project.name, self.entries.len()))
            }
            (None, None, None) => SharedString::from("Aucun projet"),
        };

        div()
            .flex()
            .items_center()
            .h(px(24.))
            .px_3()
            .flex_none()
            .bg(rgb(theme::PANEL_BG))
            .border_t_1()
            .border_color(rgb(theme::BORDER))
            .text_xs()
            .text_color(rgb(theme::TEXT_MUTED))
            .child(label)
    }
}

impl Render for Workspace {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        // Coming back from Zed is the moment to notice what changed there.
        let active = window.is_window_active();
        if active && !self.was_active {
            self.check_disk(cx);
        }
        self.was_active = active;

        // Only the active window, and only in memory: there is a single saved
        // geometry, so letting every window stage its own would persist
        // whichever repainted last. Writing the file on every frame of a drag
        // would be absurd anyway — `settings::flush` at quit puts it away.
        let bounds = window.bounds();
        if active {
            crate::settings::stage_state(cx, |state| {
                state.window = Some(crate::settings::WindowGeometry {
                    x: bounds.origin.x.into(),
                    y: bounds.origin.y.into(),
                    width: bounds.size.width.into(),
                    height: bounds.size.height.into(),
                });
            });
        }

        self.sync_prop_inputs(window, cx);
        self.sync_menu_inputs(window, cx);
        let visible = crate::settings::prefs(cx).clone();
        let show_panel = visible.show_project_panel && self.project.is_some();
        let panel_width = crate::settings::state(cx).panel_width.unwrap_or(240.);

        // La poignée déplace la découpe dans l'entité de gpui-component ; c'est
        // ici qu'on la relit pour la retenir. En mémoire seulement, comme la
        // géométrie de la fenêtre : un fichier par image de glissement serait
        // absurde, et `settings::flush` l'écrit à l'extinction.
        if show_panel && let Some(largeur) = self.panel_split.read(cx).sizes().first().copied() {
            let largeur = f32::from(largeur);
            if largeur > 0. {
                crate::settings::stage_state(cx, |state| state.panel_width = Some(largeur));
            }
        }

        div()
            .flex()
            .flex_col()
            .size_full()
            .bg(rgb(theme::BG))
            .text_color(rgb(theme::TEXT))
            .text_sm()
            .child(self.render_titlebar())
            .child(
                // Sans le panneau, pas de poignée : un groupe redimensionnable
                // à un seul volet coûterait un état pour rien.
                div()
                    .flex()
                    .flex_1()
                    .overflow_hidden()
                    .when(!show_panel, |this| this.child(self.render_main(cx)))
                    .when(show_panel, |this| {
                        this.child(
                            h_resizable("panneaux")
                                .with_state(&self.panel_split)
                                .child(
                                    resizable_panel()
                                        .size(px(panel_width))
                                        // En dessous, l'arborescence devient
                                        // illisible ; au-delà, elle mange le
                                        // canvas.
                                        .size_range(px(160.)..px(520.))
                                        .child(fillable(self.render_project_panel(cx))),
                                )
                                .child(resizable_panel().child(fillable(self.render_main(cx)))),
                        )
                    }),
            )
            .when(visible.show_output, |this| this.child(self.render_output(cx)))
            .when(visible.show_status_bar, |this| this.child(self.render_status_bar()))
    }
}

/// Wraps the content of a resizable panel so it can actually shrink.
///
/// Without this, dragging the handle only *pushes* what is beside it: a flex
/// item defaults to `min-width: auto`, so it refuses to go below the width of
/// its own content and overflows instead of compressing. A width of zero as a
/// floor, and the content follows the handle.
pub fn fillable(content: impl IntoElement) -> impl IntoElement {
    div().flex().size_full().min_w(px(0.)).overflow_hidden().child(content)
}
