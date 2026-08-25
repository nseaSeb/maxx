//! Running the project: `cargo run`, its output panel, and stopping it.

use super::*;

impl Workspace {
    /// Builds the dependency tree in the background, so the first run does not
    /// have to.
    pub fn prewarm_project(&mut self, cx: &mut Context<Self>) {
        self.start_cargo(true, cx);
    }

    /// Runs `cargo run` on the open project and streams its output into the
    /// bottom panel.
    pub fn run_project(&mut self, cx: &mut Context<Self>) {
        self.start_cargo(false, cx);
    }

    fn start_cargo(&mut self, prewarm: bool, cx: &mut Context<Self>) {
        let Some(project) = self.project.as_ref() else {
            return;
        };
        if self.run_state == crate::run::State::Running {
            self.message =
                Some(SharedString::from("une exécution est déjà en cours — ⌘. pour l'arrêter"));
            cx.notify();
            return;
        }

        let root = project.root.clone();
        self.run_output.clear();
        self.run_state = crate::run::State::Running;
        self.run_pid = None;
        crate::settings::update_prefs(cx, |preferences| preferences.show_output = true);
        self.message = None;

        let receiver = if prewarm { crate::run::prewarm(root) } else { crate::run::start(root) };
        self.run_task = Some(cx.spawn(async move |workspace, cx| {
            loop {
                let mut lines = Vec::new();
                let mut pid = None;
                let mut finished = None;
                while let Ok(message) = receiver.try_recv() {
                    match message {
                        crate::run::Message::Started(id) => pid = Some(id),
                        crate::run::Message::Line(line) => lines.push(SharedString::from(line)),
                        crate::run::Message::Finished(ok) => finished = Some(ok),
                    }
                }

                if !lines.is_empty() || pid.is_some() || finished.is_some() {
                    let updated = workspace.update(cx, |workspace, cx| {
                        workspace.run_output.extend(lines);
                        // The panel is a log, not a buffer: an application left
                        // running for an hour must not grow the process.
                        let overflow = workspace.run_output.len().saturating_sub(500);
                        workspace.run_output.drain(..overflow);
                        if let Some(pid) = pid {
                            workspace.run_pid = Some(pid);
                        }
                        if let Some(ok) = finished {
                            workspace.run_state = crate::run::State::Finished { ok };
                            workspace.run_pid = None;
                        }
                        // Follow the tail, the way a terminal does.
                        if let Some(last) = workspace.run_output.len().checked_sub(1) {
                            workspace.output_scroll.scroll_to_item(last, ScrollStrategy::Top);
                        }
                        cx.notify();
                    });
                    if updated.is_err() {
                        return;
                    }
                }

                if finished.is_some() {
                    return;
                }
                cx.background_executor().timer(std::time::Duration::from_millis(80)).await;
            }
        }));
        cx.notify();
    }

    /// Stops the running process.
    pub fn stop_project(&mut self, cx: &mut Context<Self>) {
        let Some(pid) = self.run_pid.take() else {
            return;
        };
        crate::run::stop(pid);
        self.message = Some(SharedString::from("exécution interrompue"));
        cx.notify();
    }

    /// Shows or hides the output panel.
    pub fn toggle_output(&mut self, cx: &mut Context<Self>) {
        crate::settings::update_prefs(cx, |preferences| {
            preferences.show_output = !preferences.show_output;
        });
        notify_all(cx);
    }

    /// The output of the last run.
    pub(super) fn render_output(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let (label, colour) = match self.run_state {
            crate::run::State::Idle => ("prêt", theme::TEXT_MUTED),
            crate::run::State::Running => ("exécution…", theme::ACCENT),
            crate::run::State::Finished { ok: true } => ("terminé", theme::TEXT_MUTED),
            crate::run::State::Finished { ok: false } => ("échec", 0xe06c75),
        };
        let lines = self.run_output.clone();
        // What cargo is doing right now is more useful than a bar with no total.
        let current = self
            .run_output
            .iter()
            .rev()
            .find(|line| !line.trim().is_empty())
            .cloned()
            .unwrap_or_default();

        div()
            .flex()
            .flex_col()
            .h(px(200.))
            .flex_none()
            .bg(rgb(theme::PANEL_BG))
            .border_t_1()
            .border_color(rgb(theme::BORDER))
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap_2()
                    .px_3()
                    .py_1()
                    .text_xs()
                    .when(self.run_state == crate::run::State::Running, |this| {
                        this.child(Spinner::new().small())
                    })
                    .child(div().text_color(rgb(colour)).child(label))
                    .child(
                        div()
                            .flex_1()
                            .overflow_hidden()
                            .text_color(rgb(theme::TEXT_MUTED))
                            .child(current),
                    )
                    .when(self.run_state == crate::run::State::Running, |this| {
                        this.child(
                            div()
                                .id("run-stop")
                                .px_2()
                                .rounded_sm()
                                .cursor_pointer()
                                .hover(|this| this.bg(rgb(theme::HOVER_BG)))
                                .child("Arrêter")
                                .on_click(cx.listener(|this, _, _, cx| this.stop_project(cx))),
                        )
                    })
                    .child(
                        div()
                            .id("run-close")
                            .px_2()
                            .rounded_sm()
                            .cursor_pointer()
                            .hover(|this| this.bg(rgb(theme::HOVER_BG)))
                            .child("Fermer")
                            .on_click(cx.listener(|this, _, _, cx| this.toggle_output(cx))),
                    ),
            )
            .child(
                div()
                    .relative()
                    .flex_1()
                    .overflow_hidden()
                    .child(
                        uniform_list(
                            "run-output",
                            lines.len(),
                            cx.processor(
                                move |this, range: std::ops::Range<usize>, _window, _cx| {
                                    range
                                        .filter_map(|index| this.run_output.get(index).cloned())
                                        .map(|line| {
                                            div()
                                                .px_3()
                                                .text_xs()
                                                .font_family("Menlo")
                                                .text_color(rgb(if line.contains("error") {
                                                    0xe06c75
                                                } else if line.contains("warning") {
                                                    0xe5c07b
                                                } else {
                                                    theme::TEXT_MUTED
                                                }))
                                                .child(line)
                                        })
                                        .collect::<Vec<_>>()
                                },
                            ),
                        )
                        .track_scroll(self.output_scroll.clone())
                        .size_full(),
                    )
                    .child(
                        div()
                            .absolute()
                            .top_0()
                            .right_0()
                            .bottom_0()
                            .child(Scrollbar::vertical(&self.output_scroll)),
                    ),
            )
    }
}
