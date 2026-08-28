//! Running the project: `cargo run`, its output panel, and stopping it.

use super::*;

use crate::projectfile::RunField;

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
            self.message = Some(crate::tr("message.run_in_progress"));
            cx.notify();
            return;
        }

        let root = project.root.clone();
        // A box still holding the caret has not been written yet, and ⌘R is a
        // global shortcut: without this, editing the profile and launching
        // without leaving the field would run the previous one.
        self.save_run(cx);
        self.run_output.clear();
        // What is actually being launched, before its first line of output:
        // the project says the profile, the features and the arguments, and
        // they are otherwise invisible.
        let subcommand = if prewarm { "build" } else { "run" };
        // A file that does not parse is read as an empty one, so the line below
        // would show a plain `cargo run` with no hint that the project asked
        // for anything else.
        if let Some(error) = crate::run::project_file_error(&root) {
            self.run_output.push(SharedString::from(error));
        }
        self.run_output
            .push(SharedString::from(format!("$ {}", crate::run::command_line(&root, subcommand))));
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
        self.message = Some(crate::tr("message.run_stopped"));
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
            crate::run::State::Idle => (crate::tr("run.idle"), theme::text_muted()),
            crate::run::State::Running => (crate::tr("run.running"), theme::accent()),
            crate::run::State::Finished { ok: true } => {
                (crate::tr("run.finished"), theme::text_muted())
            }
            crate::run::State::Finished { ok: false } => (crate::tr("run.failed"), theme::danger()),
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
            .bg(theme::panel_bg())
            .border_t_1()
            .border_color(theme::border())
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
                    .child(div().text_color(colour).child(label))
                    .child(
                        div()
                            .flex_1()
                            .overflow_hidden()
                            .text_color(theme::text_muted())
                            .child(current),
                    )
                    .when(self.run_state == crate::run::State::Running, |this| {
                        this.child(
                            div()
                                .id("run-stop")
                                .px_2()
                                .rounded_sm()
                                .cursor_pointer()
                                .hover(|this| this.bg(theme::hover_bg()))
                                .child(crate::tr("run.stop"))
                                .on_click(cx.listener(|this, _, _, cx| this.stop_project(cx))),
                        )
                    })
                    .child(
                        div()
                            .id("run-close")
                            .px_2()
                            .rounded_sm()
                            .cursor_pointer()
                            .hover(|this| this.bg(theme::hover_bg()))
                            .child(crate::tr("run.close"))
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
                                                .text_color(if line.contains("error") {
                                                    theme::danger()
                                                } else if line.contains("warning") {
                                                    gpui::rgb(0xe5c07b)
                                                } else {
                                                    theme::text_muted()
                                                })
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

/// Editing the project's `[run]` section, on the preferences screen.
impl Workspace {
    /// Builds the run page's text boxes, once per project.
    ///
    /// Built here rather than where they are drawn, for the reason every other
    /// panel in maxx has: `SettingItem::render` runs on every repaint, and an
    /// entity created there would be a new one each frame — a field that loses
    /// the caret as soon as it is typed in.
    pub(super) fn sync_run_inputs(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let root = self.project.as_ref().map(|project| project.root.clone());
        if self.run_synced == root {
            return;
        }
        self.run_synced = root.clone();
        self.run_inputs.clear();
        let Some(root) = root else {
            self.run_config = crate::projectfile::Run::default();
            return;
        };

        self.run_config = crate::projectfile::load(&root).run;
        for field in [RunField::Profile, RunField::Features, RunField::Args] {
            let value = self.run_config.get(field);
            let state = cx.new(|cx| InputState::new(window, cx).default_value(value));
            cx.subscribe(&state, move |this, state, event: &InputEvent, cx| match event {
                // The typing is kept in memory; the file is written when the
                // field is left. A `maxx.toml` rewritten on every keystroke
                // would be a disk write per character, and — worse — every
                // half-typed feature name would be a state the project was
                // briefly in.
                InputEvent::Change => {
                    let value = state.read(cx).value().to_string();
                    this.run_config.set(field, &value);
                }
                InputEvent::Blur | InputEvent::PressEnter { .. } => this.save_run(cx),
                _ => {}
            })
            .detach();
            self.run_inputs.push((field, state));
        }
    }

    /// Turns the switch, and writes: one file write per toggle is not a burden
    /// the way one per keystroke would be.
    pub(crate) fn toggle_default_features(&mut self, on: bool, cx: &mut Context<Self>) {
        self.run_config.default_features = on;
        self.save_run(cx);
    }

    /// Writes `[run]` back to `maxx.toml`.
    ///
    /// The whole file is loaded and put back rather than patched: `[run]` is
    /// maxx's own section, where `[modules]` and `[project]` are written by
    /// other hands, and reading them back is what keeps them.
    fn save_run(&mut self, cx: &mut Context<Self>) {
        let Some(root) = self.project.as_ref().map(|project| project.root.clone()) else {
            return;
        };
        let mut file = crate::projectfile::load(&root);
        if file.run == self.run_config {
            return;
        }
        file.run = self.run_config.clone();
        if let Err(error) = crate::projectfile::save(&root, &file) {
            self.message = Some(SharedString::from(error.to_string()));
        }
        cx.notify();
    }

    /// What the run page holds, for the screen that draws it.
    pub(crate) fn run_config(&self) -> &crate::projectfile::Run {
        &self.run_config
    }

    /// The boxes of the run page, in the order they were built.
    pub(crate) fn run_inputs(&self) -> &[(RunField, Entity<InputState>)] {
        &self.run_inputs
    }
}
