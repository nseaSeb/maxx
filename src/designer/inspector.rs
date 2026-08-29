//! The inspector: every property of the selected node, and what typing in one
//! writes back into the tree.

use rust_i18n::t;

use gpui::prelude::*;
use gpui::{AnyElement, Context, SharedString, div, px};
use gpui_component::input::Input;
use gpui_component::{Sizable as _, h_flex, v_flex};

use crate::model::{Call, Node};
use crate::registry::{self, Kind, Prop, Spec};
use crate::theme;

use crate::workspace::Workspace;

use super::canvas::thumbnail;
use super::section_title;

impl Workspace {
    /// One heading of the inspector, and the count it hides when folded.
    ///
    /// The count is what makes a folded heading worth reading: `Appearance 5`
    /// says there is something under it, where a bare title says only that the
    /// heading exists.
    fn render_group_heading(
        &self,
        group: registry::Group,
        count: usize,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let folded = self.is_folded(group);
        h_flex()
            .id(SharedString::from(format!("group-{group:?}")))
            .px_3()
            .py_1p5()
            .gap_2()
            .items_center()
            .border_t_1()
            .border_color(theme::border())
            .cursor_pointer()
            .hover(|this| this.bg(theme::hover_bg()))
            .child(div().w(px(10.)).text_xs().text_color(theme::text_muted()).child(if folded {
                "\u{25b8}"
            } else {
                "\u{25be}"
            }))
            .child(div().font_weight(gpui::FontWeight::SEMIBOLD).child(crate::tr(group.label())))
            .child(
                div()
                    .ml_auto()
                    .text_xs()
                    .text_color(theme::text_muted())
                    .child(SharedString::from(count.to_string())),
            )
            .on_click(cx.listener(move |this, _, _, cx| this.toggle_group(group, cx)))
    }

    /// One builder method of a project component, as a row of the inspector.
    fn render_brick_prop(
        &self,
        node: &Node,
        prop: &crate::bricks::BrickProp,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let label = SharedString::from(prop.method.clone());
        let method = prop.method.clone();
        let on = node.call(&prop.method).is_some();
        let field: AnyElement = if prop.text {
            match self.brick_input(&prop.method) {
                Some(state) => Input::new(state).small().into_any_element(),
                None => div().into_any_element(),
            }
        } else {
            div()
                .id(SharedString::from(format!("brick-flag-{}", prop.method)))
                .px_2()
                .py_1()
                .rounded_sm()
                .cursor_pointer()
                .text_xs()
                .bg(if on { theme::accent() } else { theme::hover_bg() })
                .text_color(if on { theme::on_accent() } else { theme::text_muted() })
                .child(crate::tr(if on { "designer.on" } else { "designer.off" }))
                .on_click(cx.listener(move |this, _, _, cx| {
                    this.toggle_brick_flag(&method, cx);
                }))
                .into_any_element()
        };
        v_flex()
            .px_3()
            .py_1()
            .gap_1()
            .child(div().text_xs().text_color(theme::text_muted()).child(label))
            .child(field)
    }

    /// The inspector's heading and search box, drawn outside the scroll.
    ///
    /// Apart from the rows for the reason the palette's is: a box that scrolls
    /// away from what it filters is a box you lose the moment it works.
    pub(super) fn render_inspector_header(&self, _cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .flex_none()
            .bg(theme::panel_bg())
            .child(section_title("designer.properties"))
            .when_some(self.prop_filter().cloned(), |this, filter| {
                this.child(div().px_3().pb_2().child(Input::new(&filter).small()))
            })
    }

    /// Property editor for the selected node, driven by the catalogue.
    pub(super) fn render_inspector(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let view = self.view().expect("checked by the caller");
        let node = view.selected();
        let spec = registry::of(node);
        let query = self
            .prop_filter()
            .map(|filter| filter.read(cx).value().to_string())
            .unwrap_or_default();
        let searching = !query.trim().is_empty();

        // Built eagerly: `cx` cannot be reborrowed inside the `FnMut` a
        // `.children(map(..))` would need.
        let mut rows = Vec::new();
        // A component of the project, read out of its own source. Its builder
        // methods are its properties: one taking a string is a field, one
        // taking nothing is a switch.
        if spec.is_none()
            && let Some(brick) = self.brick_of(node)
        {
            for prop in brick
                .props
                .iter()
                .filter(|prop| crate::designer::label_matches(&prop.method, &prop.method, &query))
            {
                rows.push(self.render_brick_prop(node, prop, cx).into_any_element());
            }
        }
        if let Some(spec) = spec {
            // Under headings rather than in one run: twenty rows in the order
            // the catalogue happens to declare them is a list you read from the
            // top every time, and folding what you are not working on is the
            // whole point of naming them.
            for group in registry::Group::ALL {
                let of_group: Vec<_> = registry::props(spec)
                    .into_iter()
                    .filter(|prop| registry::group_of(prop) == group)
                    .filter(|prop| {
                        crate::designer::label_matches(&crate::tr(prop.label), prop.label, &query)
                    })
                    .collect();
                if of_group.is_empty() {
                    continue;
                }
                rows.push(self.render_group_heading(group, of_group.len(), cx).into_any_element());
                // A folded heading opens while a search is running: hiding what
                // was just searched for is the one thing a search must not do.
                if self.is_folded(group) && !searching {
                    continue;
                }
                for prop in of_group {
                    rows.push(self.render_prop(node, spec, prop, cx).into_any_element());
                }
            }

            // Everything the model carries and no property owns. Shown rather
            // than hidden: maxx preserves these faithfully, so it should at
            // least admit they are there.
            let extra: Vec<Call> = node
                .calls
                .iter()
                .filter(|call| {
                    call.name != crate::model::CHILD_SLOT && !registry::covers(spec, &call.name)
                })
                .cloned()
                .collect();
            if !extra.is_empty() {
                rows.push(section_title("designer.other_calls").into_any_element());
                for call in extra {
                    rows.push(self.render_extra_call(&call, cx).into_any_element());
                }
            }
        }

        v_flex()
            .when(node.is_opaque(), |this| {
                this.child(
                    div()
                        .px_3()
                        .py_2()
                        .text_xs()
                        .text_color(theme::text_muted())
                        .child(crate::tr("designer.opaque")),
                )
            })
            .children(rows)
    }

    /// One call the catalogue does not know about: shown, and removable.
    pub(super) fn render_extra_call(
        &self,
        call: &Call,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let name = call.name.clone();
        let mut text = format!(".{}(", call.name);
        for (index, arg) in call.args.iter().enumerate() {
            if index > 0 {
                text.push_str(", ");
            }
            text.push_str(&arg.to_source());
        }
        text.push(')');

        h_flex()
            .items_center()
            .gap_2()
            .px_3()
            .py_1()
            .child(
                div()
                    .flex_1()
                    .text_xs()
                    .font_family("Menlo")
                    .text_color(theme::text_muted())
                    .child(SharedString::from(text)),
            )
            .child(
                div()
                    .id(SharedString::from(format!("drop-call-{name}")))
                    .px_2()
                    .rounded_sm()
                    .cursor_pointer()
                    .hover(|this| this.bg(theme::hover_bg()))
                    .child("×")
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.remove_call_at_selection(&name, cx);
                    })),
            )
    }

    /// One property row.
    pub(super) fn render_prop(
        &self,
        node: &Node,
        spec: &'static Spec,
        prop: &'static Prop,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let current = registry::read(node, prop).unwrap_or_default();
        let row = h_flex().items_center().gap_2().px_3().py_1().child(
            div()
                .w(px(90.))
                .flex_none()
                .text_xs()
                .text_color(theme::text_muted())
                .child(crate::tr(prop.label)),
        );

        match prop.kind {
            // The state panel knows which fields can back an input; making the
            // name be typed again when the answer is on screen is the tool
            // contradicting itself.
            Kind::Field if !self.state_fields().is_empty() => {
                let current = current.clone();
                row.child(
                    div()
                        .id(SharedString::from(format!("field-{}-{}", spec.id, prop.label)))
                        .flex_1()
                        .px_2()
                        .rounded_sm()
                        .cursor_pointer()
                        .bg(theme::bg())
                        .text_color(theme::accent())
                        .hover(|this| this.bg(theme::hover_bg()))
                        .child(if current.is_empty() {
                            SharedString::from("—")
                        } else {
                            SharedString::from(current)
                        })
                        .on_click(
                            cx.listener(move |this, _, _, cx| this.cycle_input_field(prop, cx)),
                        ),
                )
            }
            Kind::Text if registry::read_binding(node, prop).is_some() => {
                let field = registry::read_binding(node, prop).unwrap_or_default();
                row.child(
                    div()
                        .id(SharedString::from(format!("bind-{}-{}", spec.id, prop.label)))
                        .flex_1()
                        .px_2()
                        .rounded_sm()
                        .cursor_pointer()
                        .bg(theme::bg())
                        .text_color(theme::accent())
                        .hover(|this| this.bg(theme::hover_bg()))
                        .child(SharedString::from(field))
                        .on_click(cx.listener(move |this, _, _, cx| this.cycle_binding(prop, cx))),
                )
                .child(binding_toggle(spec, prop, true, cx))
            }
            Kind::Text
            | Kind::Field
            | Kind::Handler
            | Kind::Number
            | Kind::Color
            | Kind::Ratio
            | Kind::Count
            | Kind::Path => {
                match self.prop_input(prop) {
                    Some(state) if matches!(prop.kind, Kind::Handler) => {
                        row.child(div().flex_1().child(Input::new(state).small()))
                            .child(
                                div()
                                    .id(SharedString::from(format!("goto-{}", prop.label)))
                                    .px_2()
                                    .rounded_sm()
                                    .text_xs()
                                    .cursor_pointer()
                                    .hover(|this| this.bg(theme::hover_bg()))
                                    .child(
                                        t!(
                                            "designer.open_in",
                                            editor = crate::tools::editor_label(cx)
                                        )
                                        .into_owned(),
                                    )
                                    .on_click(cx.listener(move |this, _, _, cx| {
                                        this.open_handler(prop, cx)
                                    })),
                            )
                            // The boxes gpui-component presents imperatively, and
                            // which therefore never appear on the canvas: their
                            // place is the other end of the same gesture — this
                            // button opens that box.
                            .children(crate::scaffold::templates::BOXES.iter().map(
                                |(kind, _, _)| {
                                    div()
                                        .id(SharedString::from(format!("box-{kind}")))
                                        .px_2()
                                        .rounded_sm()
                                        .text_xs()
                                        .cursor_pointer()
                                        .hover(|this| this.bg(theme::hover_bg()))
                                        .tooltip(|window, cx| {
                                            gpui_component::tooltip::Tooltip::new(crate::tr(
                                                "designer.opens_desc",
                                            ))
                                            .build(window, cx)
                                        })
                                        .child(SharedString::from(format!(
                                            "{} {kind}",
                                            crate::tr("designer.opens")
                                        )))
                                        .on_click(cx.listener(move |this, _, _, cx| {
                                            this.fill_handler(prop, kind, cx)
                                        }))
                                },
                            ))
                    }
                    Some(state) if matches!(prop.kind, Kind::Path) => row
                        .child(thumbnail(&current, self.project().map(|p| p.root.as_path())))
                        .child(div().flex_1().child(Input::new(state).small()))
                        // How many pixels the picture really has: a width is
                        // not thinkable without it, and the field says nothing.
                        .children(self.image_size.map(|(width, height)| {
                            div()
                                .flex_none()
                                .text_xs()
                                .text_color(theme::text_muted())
                                .child(SharedString::from(format!("{width} × {height}")))
                        }))
                        .child(
                            div()
                                .id(SharedString::from(format!("pick-{}", prop.label)))
                                .px_2()
                                .rounded_sm()
                                .text_xs()
                                .cursor_pointer()
                                .hover(|this| this.bg(theme::hover_bg()))
                                .child(crate::tr("designer.choose"))
                                .on_click(
                                    cx.listener(move |this, _, _, cx| this.pick_path(prop, cx)),
                                ),
                        ),
                    Some(state) if matches!(prop.kind, Kind::Text) => row
                        .child(div().flex_1().child(Input::new(state).small()))
                        .child(binding_toggle(spec, prop, false, cx)),
                    Some(state) => row.child(div().flex_1().child(Input::new(state).small())),
                    // No input this frame: the sync runs at the top of `render`, so
                    // this only shows for a frame after a selection change.
                    None => row.child(
                        div()
                            .flex_1()
                            .px_2()
                            .rounded_sm()
                            .bg(theme::bg())
                            .child(SharedString::from(current)),
                    ),
                }
            }
            Kind::Bool => {
                let on = current == "true";
                row.child(
                    div()
                        .id(SharedString::from(format!("prop-{}-{}", spec.id, prop.label)))
                        .px_2()
                        .rounded_sm()
                        .cursor_pointer()
                        .bg(if on { theme::accent() } else { theme::bg() })
                        .text_color(if on { theme::on_accent() } else { theme::text() })
                        .child(crate::tr(if on { "designer.yes" } else { "designer.no" }))
                        .on_click(cx.listener(move |this, _, _, cx| {
                            this.edit_prop(prop, if on { "false" } else { "true" }, cx);
                        })),
                )
            }
            Kind::Choice => {
                let names = match prop.target {
                    crate::registry::Target::Family(names) => names,
                    // A list of enum variants cycles exactly like a family of
                    // methods: one of them applies, or none.
                    crate::registry::Target::Variant(_, values) => values,
                    crate::registry::Target::VariantArg(_, values) => values,
                    _ => &[][..],
                };
                // An argument of the constructor has no empty state to cycle
                // through: the component does not compile without it.
                let wraps = !matches!(prop.target, crate::registry::Target::VariantArg(..));
                let next = match (next_in_family(names, &current).as_str(), wraps) {
                    ("", false) => {
                        names.first().map(|name| (*name).to_string()).unwrap_or_default()
                    }
                    (next, _) => next.to_string(),
                };
                row.child(
                    div()
                        .id(SharedString::from(format!("prop-{}-{}", spec.id, prop.label)))
                        .flex_1()
                        .px_2()
                        .rounded_sm()
                        .cursor_pointer()
                        .bg(theme::bg())
                        .hover(|this| this.bg(theme::hover_bg()))
                        .child(if current.is_empty() {
                            crate::tr("designer.default")
                        } else {
                            SharedString::from(current)
                        })
                        .on_click(cx.listener(move |this, _, _, cx| {
                            this.edit_prop(prop, &next, cx);
                        })),
                )
            }
        }
    }

    /// The open view, and the box that renames it.
    ///
    /// A section of its own above the state: every view maxx creates is called
    /// `view_1`, `view_2`, … and naming it is the first thing anyone does.
    pub(super) fn render_view_name(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let module = self
            .project()
            .zip(self.view())
            .and_then(|(project, view)| crate::workspace::view_module(&project.root, &view.path))
            .unwrap_or_default();

        v_flex().child(section_title("designer.view")).child(
            h_flex()
                .gap_2()
                .px_3()
                .py_1()
                .text_xs()
                .child(
                    div()
                        .text_color(theme::text_muted())
                        .font_family("Menlo")
                        .child(SharedString::from(module)),
                )
                .when_some(self.rename_input().cloned(), |this, state| {
                    this.child(div().flex_1().child(Input::new(&state).small()))
                })
                .child(
                    div()
                        .id("view-rename")
                        .px_2()
                        .rounded_sm()
                        .text_xs()
                        .cursor_pointer()
                        .bg(theme::bg())
                        .hover(|this| this.bg(theme::hover_bg()))
                        .child(crate::tr("designer.rename"))
                        .on_click(cx.listener(|this, _, _, cx| this.rename_view(cx))),
                ),
        )
    }

    /// The fields of the view's struct, and a box to add one.
    ///
    /// A property can only read what exists, so declaring the field comes
    /// first; binding a property to it is one click away in the inspector.
    pub(super) fn render_state(&self, cx: &mut Context<Self>) -> impl IntoElement {
        let fields = self.view().map(|view| view.state_fields()).unwrap_or_default();
        let (type_label, _, _) = crate::view::STATE_TYPES[self.state_type()];
        let type_label = crate::tr(type_label);

        v_flex()
            .child(section_title("designer.state"))
            .children(fields.into_iter().map(|field| {
                h_flex()
                    .gap_2()
                    .px_3()
                    .py_1()
                    .text_xs()
                    .child(div().flex_1().child(SharedString::from(field.name)))
                    .child(
                        div()
                            .text_color(theme::text_muted())
                            .font_family("Menlo")
                            .child(SharedString::from(field.ty)),
                    )
            }))
            .child(
                h_flex()
                    .gap_2()
                    .px_3()
                    .py_1()
                    .when_some(self.state_name_input().cloned(), |this, state| {
                        this.child(div().flex_1().child(Input::new(&state).small()))
                    })
                    .child(
                        div()
                            .id("state-type")
                            .px_2()
                            .rounded_sm()
                            .text_xs()
                            .cursor_pointer()
                            .bg(theme::bg())
                            .hover(|this| this.bg(theme::hover_bg()))
                            .child(type_label)
                            .on_click(cx.listener(|this, _, _, cx| this.cycle_state_type(cx))),
                    )
                    .child(
                        div()
                            .id("state-add")
                            .px_2()
                            .rounded_sm()
                            .text_xs()
                            .cursor_pointer()
                            .bg(theme::accent())
                            .text_color(theme::on_accent())
                            .child(crate::tr("designer.add"))
                            .on_click(cx.listener(|this, _, _, cx| this.add_state_field(cx))),
                    ),
            )
    }
}

/// The button that switches a text property between a literal and a field of
/// the view's state.
pub(super) fn binding_toggle(
    spec: &'static Spec,
    prop: &'static Prop,
    bound: bool,
    cx: &mut Context<Workspace>,
) -> impl IntoElement {
    div()
        .id(SharedString::from(format!("toggle-{}-{}", spec.id, prop.label)))
        .px_1()
        .rounded_sm()
        .text_xs()
        .cursor_pointer()
        .font_family("Menlo")
        .text_color(if bound { theme::accent() } else { theme::text_muted() })
        .hover(|this| this.bg(theme::hover_bg()))
        .child(if bound { "{ }" } else { "abc" })
        .on_click(cx.listener(move |this, _, _, cx| this.toggle_binding(prop, cx)))
}

/// The next value of a family, cycling through it and back to "unset".
pub(super) fn next_in_family(names: &'static [&'static str], current: &str) -> String {
    match names.iter().position(|name| *name == current) {
        None => names.first().map(|name| (*name).to_string()).unwrap_or_default(),
        Some(index) if index + 1 < names.len() => names[index + 1].to_string(),
        Some(_) => String::new(),
    }
}

/// A tag in the variant the node carries.
///
/// Read off `with_variant`, like every other property the canvas shows: a
/// variant that reaches the generated file and leaves the canvas identical is
/// the inspector looking broken.
pub(super) fn tag_variant(node: &Node) -> gpui_component::tag::Tag {
    use gpui_component::tag::{Tag, TagVariant};
    let variant = match call_source(node, "with_variant").as_deref() {
        Some("TagVariant::Primary") => TagVariant::Primary,
        Some("TagVariant::Danger") => TagVariant::Danger,
        Some("TagVariant::Success") => TagVariant::Success,
        Some("TagVariant::Warning") => TagVariant::Warning,
        Some("TagVariant::Info") => TagVariant::Info,
        // What `Tag::new` gives, and what an unreadable variant falls back to.
        _ => TagVariant::Secondary,
    };
    Tag::new().with_variant(variant)
}

/// The source text of a one-argument call, when it has one.
pub(super) fn call_source(node: &Node, name: &str) -> Option<String> {
    node.call(name)?.args.first().map(|arg| arg.to_source())
}

/// The source text of a base argument, or the empty string.
pub(super) fn base_source(node: &Node, index: usize) -> String {
    match &node.base {
        crate::model::Base::Known { args, .. } => {
            args.get(index).map(|arg| arg.to_source()).unwrap_or_default()
        }
        crate::model::Base::Opaque(_) => String::new(),
    }
}

/// The whole-number argument of a one-argument call, when it reads as one.
pub(super) fn call_whole(node: &Node, name: &str) -> Option<usize> {
    node.call(name)?.args.first()?.to_source().parse().ok()
}

/// The string argument of a one-argument call, or `fallback`.
pub(super) fn call_text(node: &Node, name: &str, fallback: &str) -> String {
    node.call(name)
        .and_then(|call| call.args.first())
        .and_then(|arg| arg.as_str())
        .unwrap_or(fallback)
        .to_string()
}

/// The numeric argument of a one-argument call, when it reads as a number.
pub(super) fn call_number(node: &Node, name: &str) -> Option<f32> {
    node.call(name)?.args.first()?.to_source().trim_end_matches('.').parse().ok()
}

/// The boolean argument of a one-argument call, `false` when absent.
pub(super) fn call_bool(node: &Node, name: &str) -> bool {
    node.call(name)
        .and_then(|call| call.args.first())
        .map(|arg| arg.to_source() == "true")
        .unwrap_or(false)
}

/// Applies the placement calls, so a pinned overlay is drawn pinned.
///
/// Without them the bar maxx writes into the corner of a box shows on the
/// canvas as an ordinary child at the end of the column — the canvas showing a
/// layout the file does not have.
pub(super) fn apply_placement<E: Styled>(element: E, calls: &[crate::model::Call]) -> E {
    let mut element = element;
    for call in calls {
        element = match call.name.as_str() {
            "absolute" => element.absolute(),
            "relative" => element.relative(),
            "top_0" => element.top_0(),
            "left_0" => element.left_0(),
            "right_0" => element.right_0(),
            "bottom_0" => element.bottom_0(),
            _ => element,
        };
    }
    element
}

/// Applies the style calls the preview knows how to show.
///
/// A call that is not listed here is still carried by the model and written to
/// the file; it simply has no effect on the preview.
pub(super) fn apply<T: Styled>(mut element: T, calls: &[Call]) -> T {
    for call in calls {
        let argument = call.args.first().map(|arg| arg.to_source()).unwrap_or_default();
        element = match call.name.as_str() {
            "gap_0" => element.gap_0(),
            "gap_1" => element.gap_1(),
            "gap_2" => element.gap_2(),
            "gap_3" => element.gap_3(),
            "gap_4" => element.gap_4(),
            "gap_6" => element.gap_6(),
            "gap_8" => element.gap_8(),
            "p_0" => element.p_0(),
            "p_1" => element.p_1(),
            "p_2" => element.p_2(),
            "p_3" => element.p_3(),
            "p_4" => element.p_4(),
            "p_6" => element.p_6(),
            "p_8" => element.p_8(),
            "items_start" => element.items_start(),
            "items_center" => element.items_center(),
            "items_end" => element.items_end(),
            "flex_1" => element.flex_1(),
            // The shared properties, which the panel puts first and the board
            // used to ignore: a width typed in the inspector reached the file
            // and changed nothing on screen, which reads as a defect of the
            // field rather than of the preview.
            "w" => match pixels(&argument) {
                Some(value) => element.w(px(value)),
                None => element,
            },
            "h" => match pixels(&argument) {
                Some(value) => element.h(px(value)),
                None => element,
            },
            "w_full" => element.w_full(),
            "h_full" => element.h_full(),
            "size_full" => element.size_full(),
            "max_w_full" => element.max_w_full(),
            "bg" => match colour(&argument) {
                Some(value) => element.bg(value),
                None => element,
            },
            "text_color" => match colour(&argument) {
                Some(value) => element.text_color(value),
                None => element,
            },
            "text_xs" => element.text_xs(),
            "text_sm" => element.text_sm(),
            "text_base" => element.text_base(),
            "text_lg" => element.text_lg(),
            "text_xl" => element.text_xl(),
            "text_2xl" => element.text_2xl(),
            // Weight is written `font_weight(FontWeight::…)` since we found out
            // that `font_medium()` does not exist: what has to be followed here
            // is the name of the call, not the name of the variant.
            "font_weight" => match argument.as_str() {
                "FontWeight::MEDIUM" => element.font_weight(gpui::FontWeight::MEDIUM),
                "FontWeight::SEMIBOLD" => element.font_weight(gpui::FontWeight::SEMIBOLD),
                "FontWeight::BOLD" => element.font_weight(gpui::FontWeight::BOLD),
                _ => element.font_weight(gpui::FontWeight::NORMAL),
            },
            "rounded_none" => element.rounded_none(),
            "rounded_sm" => element.rounded_sm(),
            "rounded_md" => element.rounded_md(),
            "rounded_lg" => element.rounded_lg(),
            "rounded_full" => element.rounded_full(),
            _ => element,
        };
    }
    element
}

/// `px(40.)` read back as a number, for the preview.
pub(super) fn pixels(source: &str) -> Option<f32> {
    source.strip_prefix("px(")?.strip_suffix(')')?.trim_end_matches('.').parse().ok()
}

/// `rgb(0x1e2127)` read back as a colour, for the preview.
pub(super) fn colour(source: &str) -> Option<gpui::Rgba> {
    let hex = source.strip_prefix("rgb(0x")?.strip_suffix(')')?;
    u32::from_str_radix(hex, 16).ok().map(gpui::rgb)
}
