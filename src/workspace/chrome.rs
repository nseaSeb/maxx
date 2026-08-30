//! The window's own furniture: titlebar, welcome screen, status bar, [`Render`].

use gpui_component::input::Input;
use gpui_component::resizable::{resizable_panel, v_resizable};
use gpui_component::{h_flex, v_flex};

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
        if self.preferences() {
            self.show_designer();
        } else {
            // Asked before leaving, like every other move between modes: the
            // menu editor's unsaved work now lives *inside* `Center::Menus`, so
            // showing something else drops it rather than covering it.
            if self.discard_menu_edits(cx) {
                return;
            }
            self.show(Center::Preferences);
        }
        cx.notify();
    }

    /// Leaves the preferences screen.
    pub fn close_preferences(&mut self, cx: &mut Context<Self>) {
        if self.preferences() {
            self.show_designer();
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
            .bg(theme::titlebar_bg())
            .border_b_1()
            .border_color(theme::border())
            .child(
                div()
                    .flex()
                    .flex_1()
                    .items_center()
                    .justify_center()
                    .gap_2()
                    .child(div().child(title))
                    .child(div().text_xs().text_color(theme::text_muted()).child(subtitle)),
            )
    }

    fn render_main(&self, cx: &mut Context<Self>) -> AnyElement {
        // Before the welcome screen: both are reachable with no project open,
        // which is exactly when someone is setting maxx up — and the palette
        // reached from the preferences is a file of maxx's own, not a project's.
        // Hidden behind the welcome screen, the button that opens it did
        // nothing and took the preferences away with it.
        if self.preferences() || self.palette().is_some() {
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
            .child(div().text_color(theme::text_muted()).child(crate::tr("welcome.hint")))
            .child(
                div()
                    .id("welcome-open-folder")
                    .px_4()
                    .py_2()
                    .rounded_md()
                    .cursor_pointer()
                    .bg(theme::accent())
                    .text_color(theme::on_accent())
                    .hover(|this| this.opacity(0.85))
                    .child(crate::tr("welcome.open_folder"))
                    .on_click(cx.listener(|_, _, window, cx| {
                        window.dispatch_action(Box::new(OpenFolder), cx);
                    })),
            )
            .child(div().text_xs().text_color(theme::text_muted()).child("⌘O"))
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
                .hover(|this| this.bg(theme::hover_bg()))
                .child(div().child(name))
                .child(div().text_xs().text_color(theme::text_muted()).child(parent))
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
                        .text_color(theme::text_muted())
                        .child(crate::tr("welcome.recent")),
                )
                .children(rows)
                .into_any_element(),
        )
    }

    fn render_status_bar(&self) -> impl IntoElement {
        let conflict = self.view().is_some_and(|view| self.conflicts.contains(&view.path));
        // Only while the reader is what the middle shows. The document outlives
        // what covers it, so this arm otherwise went on naming a file nobody was
        // looking at — and swallowed the open view's own dirty and conflict
        // marks, which is what the line is for.
        if let Some(file) = self.code().filter(|_| self.showing_code()) {
            let label = match &self.message {
                Some(message) => message.clone(),
                // A view seen as code says so: what is on screen is what `⌘S`
                // would write, which on a modified view is not what the disk
                // holds — and the dot is the only thing that says it.
                None if file.of_view => SharedString::from(
                    t!("status.view_code", name = file.name(), lines = file.lines()).into_owned(),
                ),
                None if file.image => SharedString::from(
                    t!("status.image", name = file.name(), size = file.kilobytes()).into_owned(),
                ),
                None => SharedString::from(
                    t!("status.code", name = file.name(), lines = file.lines()).into_owned(),
                ),
            };
            return div()
                .flex()
                .items_center()
                .h(px(24.))
                .px_3()
                .flex_none()
                .bg(theme::panel_bg())
                .border_t_1()
                .border_color(theme::border())
                .text_xs()
                .text_color(theme::text_muted())
                .gap_2()
                .child(label)
                // The same warning as on the canvas side, in the same colour:
                // the code shown is what ⌘S *would* write, so on a modified
                // view it is precisely not what the file holds.
                .children(self.view().filter(|view| file.of_view && view.dirty()).map(|_| {
                    div().text_color(theme::warning()).child(crate::tr("status.unsaved"))
                }));
        }
        if let Some(menus) = self.menu_file() {
            let label = match &self.message {
                Some(message) => message.clone(),
                None => SharedString::from(
                    t!(
                        "status.menus",
                        name = menus.name(),
                        dirty = if menus.dirty() { " •" } else { "" },
                        count = menus.menus.len()
                    )
                    .into_owned(),
                ),
            };
            return div()
                .flex()
                .items_center()
                .h(px(24.))
                .px_3()
                .flex_none()
                .bg(theme::panel_bg())
                .border_t_1()
                .border_color(theme::border())
                .text_xs()
                .text_color(theme::text_muted())
                .child(label);
        }
        // Said in words and in colour rather than marked with a dot: what is on
        // the canvas and what is in the file differ until ⌘S, and a bullet is
        // not enough to explain that to someone reading their own `.rs` beside
        // maxx and finding it behind. Carried apart from the label because only
        // this part of the line is a warning — colouring the whole of it would
        // make the file name shout too.
        let mut warnings: Vec<SharedString> = Vec::new();
        if self.view().is_some_and(|view| view.dirty()) {
            warnings.push(crate::tr("status.unsaved"));
        }
        if conflict {
            warnings.push(crate::tr("status.changed_outside"));
        }

        let label = match (&self.message, &self.view(), &self.project) {
            (Some(message), _, _) => message.clone(),
            (None, Some(view), _) => SharedString::from(
                t!("status.nodes", name = view.name(), count = view.root.count()).into_owned(),
            ),
            (None, None, Some(project)) => SharedString::from(
                t!("status.items", name = project.name, count = self.entries.len()).into_owned(),
            ),
            (None, None, None) => crate::tr("status.no_project"),
        };

        div()
            .flex()
            .items_center()
            .h(px(24.))
            .px_3()
            .flex_none()
            .bg(theme::panel_bg())
            .border_t_1()
            .border_color(theme::border())
            .text_xs()
            .text_color(theme::text_muted())
            .gap_2()
            .child(label)
            .children(
                warnings
                    .into_iter()
                    .map(|warning| div().text_color(theme::warning()).child(warning)),
            )
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
        self.sync_run_inputs(window, cx);
        self.sync_palette_inputs(window, cx);
        self.sync_code_input(window, cx);
        let visible = crate::settings::prefs(cx).clone();
        // The column shows as soon as there is a project: the files are what
        // `⌘B` turns off, and the palette stays either way.
        let show_files = visible.show_project_panel;
        let show_panel = self.project.is_some();
        let panel_width = crate::settings::state(cx).panel_width.unwrap_or(240.);

        // The handle moves the split inside gpui-component's entity; this is
        // where it is read back to be remembered. In memory only, like the
        // window geometry: a file per frame of a drag would be absurd, and
        // `settings::flush` writes it at quit.
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
            .bg(theme::bg())
            .text_color(theme::text())
            .text_sm()
            .child(self.render_titlebar())
            .child(
                // No panel, no handle: a resizable group with a single pane
                // would cost a state for nothing.
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
                                        // Below this the tree becomes
                                        // unreadable; beyond it, it eats
                                        // the canvas.
                                        .size_range(px(160.)..px(520.))
                                        .child(fillable(self.render_left_column(show_files, cx))),
                                )
                                .child(resizable_panel().child(fillable(self.render_main(cx)))),
                        )
                    }),
            )
            .when(visible.show_output, |this| this.child(self.render_output(cx)))
            .when(visible.show_status_bar, |this| this.child(self.render_status_bar()))
            .children(self.render_command_palette(cx))
    }
}

impl Workspace {
    /// The left column: the project's files above, the components below.
    ///
    /// The palette used to sit at the bottom of the right panel, under the
    /// inspector — which meant the tool you reach for most while building was
    /// below the fold, past twenty properties. It is a **source**, not a
    /// property of the selection, and its place is beside the other source the
    /// window offers.
    pub(super) fn render_left_column(
        &self,
        files: bool,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        v_resizable("colonne-gauche")
            .with_state(&self.left_split)
            // `⌘B` hides the project's *files*. It hid only those while the
            // palette was on the other side; moving the palette here made one
            // switch turn off two things, and the second is the only way to
            // insert anything at all.
            .when(files, |this| {
                this.child(
                    resizable_panel()
                        .size(px(360.))
                        .size_range(px(120.)..px(900.))
                        .child(fillable(self.render_project_panel(cx))),
                )
            })
            .child(
                resizable_panel().size_range(px(120.)..px(900.)).child(fillable(
                    div()
                        .relative()
                        .flex()
                        .flex_col()
                        .size_full()
                        // The heading and the search box stay put; only the list
                        // moves. Inside the scroll, the field you type in left
                        // the screen as soon as you reached what it had found.
                        .child(self.render_palette_header(cx))
                        .child(
                            div()
                                .id("left-palette")
                                .flex_1()
                                .min_h(px(0.))
                                .overflow_y_scroll()
                                .track_scroll(&self.palette_scroll)
                                .child(self.render_palette(cx)),
                        )
                        .child(
                            div()
                                .absolute()
                                .top_0()
                                .right_0()
                                .bottom_0()
                                .child(Scrollbar::vertical(&self.palette_scroll)),
                        ),
                )),
            )
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

impl Workspace {
    /// The command palette, when it is open.
    ///
    /// Drawn over everything rather than beside it, and anchored near the top:
    /// a palette that pushes the window's contents around costs a reflow every
    /// time it opens, and lands where the eye is not.
    ///
    /// The `key_context` is what makes `escape`, `up` and `down` mean something
    /// here and nothing anywhere else — the keymap binds them to that name.
    pub(crate) fn render_command_palette(&self, cx: &mut Context<Self>) -> Option<AnyElement> {
        let input = self.command_input()?.clone();
        let matching = self.matching_lines(cx);
        let total = matching.len();
        let (offset, matching) = self.palette_window(&matching);
        let drawn = matching.len();
        let selected = self.command_index();

        // Gathered rather than lazy: the closure would carry `cx`, which the
        // palette still needs for its own listeners.
        let rows: Vec<_> = matching
            .into_iter()
            .enumerate()
            // A line is a command or a file, and one list is open at a time.
            .map(|(index, position)| (index + offset, position))
            .filter_map(|(index, position)| {
                let file = self.palette_file(position);
                let command = self.command_at(position);
                match (file, command) {
                    (Some(path), _) => Some((index, path, None)),
                    (None, Some(command)) => {
                        Some((index, command.label.clone(), command.shortcut.clone()))
                    }
                    (None, None) => None,
                }
            })
            .map(|(index, label, shortcut)| {
                h_flex()
                    .id(SharedString::from(format!("command-{index}")))
                    .items_center()
                    .justify_between()
                    .gap_4()
                    .px_3()
                    .py_1()
                    .rounded_sm()
                    .cursor_pointer()
                    .when(index == selected, |this| this.bg(theme::selected_bg()))
                    .hover(|this| this.bg(theme::hover_bg()))
                    .child(div().flex_1().child(label))
                    .when_some(shortcut, |this, keys| {
                        this.child(div().text_xs().text_color(theme::text_muted()).child(keys))
                    })
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.command_index = index;
                        this.run_palette(cx);
                    }))
                    .into_any_element()
            })
            .collect();

        Some(
            div()
                .key_context("Palette")
                .absolute()
                .top_0()
                .left_0()
                .size_full()
                .flex()
                .justify_center()
                // A click beside it closes, as everywhere else. Without that,
                // the only way out would be `escape`, which is not guessable.
                .on_mouse_down_out(cx.listener(|this, _, _, cx| this.close_palette(cx)))
                .child(
                    v_flex()
                        .mt(px(80.))
                        .w(px(560.))
                        .max_h(px(420.))
                        .overflow_hidden()
                        .rounded_md()
                        .border_1()
                        .border_color(theme::border())
                        .bg(theme::panel_bg())
                        .child(div().p_2().child(Input::new(&input)))
                        .when(rows.is_empty(), |this| {
                            this.child(
                                div()
                                    .px_3()
                                    .pb_2()
                                    .text_xs()
                                    .text_color(theme::text_muted())
                                    .child(self.palette_nothing()),
                            )
                        })
                        .child(
                            v_flex()
                                .id("command-list")
                                .flex_1()
                                .overflow_y_scroll()
                                .pb_2()
                                .children(rows),
                        )
                        // Under the list, because that is where a reader
                        // arrives at its end: written above, it read as a
                        // heading for the lines that follow. And it says
                        // "others", not "more below" — the window follows the
                        // highlight, so what is hidden may be on either side.
                        .when(total > drawn, |this| {
                            this.child(
                                div()
                                    .px_3()
                                    .pb_2()
                                    .text_xs()
                                    .text_color(theme::text_muted())
                                    .child(SharedString::from(
                                        t!("palette.more", count = total - drawn).into_owned(),
                                    )),
                            )
                        }),
                )
                .into_any_element(),
        )
    }
}
