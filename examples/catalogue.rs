//! Every call the catalogue can write, compiled once here.
//!
//! `registry` writes method names into a project maxx never builds. A name gpui
//! does not have — `font_medium()`, which never existed — or a call a type only
//! offers once another has been made — `overflow_y_scroll`, which lives on a
//! *stateful* element and so needs `id` before it — is a project that stops
//! compiling on a line maxx wrote itself, and the developer finds out, not maxx.
//!
//! So the names are compiled here, in the order the catalogue writes them, and
//! `tests/catalogue.rs` checks that every entry of every table appears in this
//! file. Adding a row without proving it exists fails the suite.
//!
//! Nothing is run: `main` builds elements and drops them. It is the compiler
//! that answers the question.

// The stateful pair below is built by nobody: the compiler is this file's only
// reader, and what it has to answer is whether the calls exist.
#![allow(dead_code)]

use gpui::prelude::*;
use gpui::{FontWeight, Keystroke, ObjectFit, ScrollHandle, SharedString, div, img, px, rgb, svg};
use gpui_component::IndexPath;
use gpui_component::alert::Alert;
use gpui_component::avatar::Avatar;
use gpui_component::badge::Badge;
use gpui_component::breadcrumb::Breadcrumb;
use gpui_component::button::{Button, ButtonVariants};
use gpui_component::calendar::{Calendar, CalendarState};
use gpui_component::checkbox::Checkbox;
use gpui_component::clipboard::Clipboard;
use gpui_component::color_picker::{ColorPicker, ColorPickerState};
use gpui_component::date_picker::{DatePicker, DatePickerState};
use gpui_component::divider::Divider;
use gpui_component::group_box::GroupBox;
use gpui_component::input::{Input, InputState, NumberInput, OtpInput, OtpState};
use gpui_component::kbd::Kbd;
use gpui_component::label::Label;
use gpui_component::link::Link;
use gpui_component::progress::Progress;
use gpui_component::radio::Radio;
use gpui_component::scroll::{Scrollbar, ScrollbarAxis};
use gpui_component::select::{SearchableVec, Select, SelectState};
use gpui_component::skeleton::Skeleton;
use gpui_component::slider::{Slider, SliderState};
use gpui_component::spinner::Spinner;
use gpui_component::switch::Switch;
use gpui_component::tab::{TabBar, TabVariant};
use gpui_component::tag::{Tag, TagVariant};
use gpui_component::tooltip::Tooltip;
use gpui_component::{Disableable, Icon, IconName, Sizable};

fn main() {
    // The layout properties of a column and a row.
    let _ = div()
        .gap_0()
        .gap_1()
        .gap_2()
        .gap_3()
        .gap_4()
        .gap_6()
        .gap_8()
        .p_0()
        .p_1()
        .p_2()
        .p_3()
        .p_4()
        .p_6()
        .p_8()
        .items_start()
        .items_center()
        .items_end()
        .justify_start()
        .justify_center()
        .justify_end()
        .justify_between()
        .justify_around()
        .flex_1()
        .flex_wrap();

    // The margins, the three families of them the inspector offers.
    let _ = div()
        .m_0()
        .m_1()
        .m_2()
        .m_3()
        .m_4()
        .m_6()
        .m_8()
        .mx_0()
        .mx_1()
        .mx_2()
        .mx_3()
        .mx_4()
        .mx_6()
        .mx_8()
        .my_0()
        .my_1()
        .my_2()
        .my_3()
        .my_4()
        .my_6()
        .my_8();

    // The box: what `Styled` draws around every component alike.
    let _ = div()
        .min_w(px(1.))
        .max_w(px(1.))
        .min_h(px(1.))
        .max_h(px(1.))
        .border_0()
        .border_1()
        .border_2()
        .border_4()
        .border_color(rgb(0x444444))
        .shadow_sm()
        .shadow_md()
        .shadow_lg()
        .shadow_xl()
        .opacity(0.5)
        .overflow_hidden()
        .cursor_pointer();

    // The shared style properties.
    let _ = div()
        .w(px(1.))
        .h(px(1.))
        .bg(rgb(0x111111))
        .text_color(rgb(0x222222))
        .line_height(px(20.))
        .italic()
        .underline()
        .truncate()
        .whitespace_nowrap()
        .text_xs()
        .text_sm()
        .text_base()
        .text_lg()
        .text_xl()
        .text_2xl()
        .font_weight(FontWeight::NORMAL)
        .font_weight(FontWeight::MEDIUM)
        .font_weight(FontWeight::SEMIBOLD)
        .font_weight(FontWeight::BOLD)
        .rounded_none()
        .rounded_sm()
        .rounded_md()
        .rounded_lg()
        .rounded_full();

    // Scrolling, in the order the property writes it: the id comes first, and
    // this file is where that stops being an opinion.
    let _ = div().id("scroll").overflow_y_scroll().h_full();
    let _ = div().id("scroll").overflow_x_scroll().w_full();

    // The image, its size and its fill mode.
    let _ = img("assets/images/image.png")
        .max_w_full()
        .w_full()
        .size_full()
        .object_fit(ObjectFit::Contain)
        .object_fit(ObjectFit::Cover)
        .object_fit(ObjectFit::Fill)
        .object_fit(ObjectFit::ScaleDown)
        .object_fit(ObjectFit::None);

    // The drawing: a path handed to the application's assets, a size without
    // which it lays out as nothing, and the colour gpui tints it with — an svg
    // with no `text_color` is painted not at all, which is why the property
    // sits on the entry rather than among the shared text ones.
    let _ = svg().path("assets/images/drawing.svg").size_4().text_color(rgb(0x333333));

    // What changes under the pointer, in the shape the property writes it: one
    // closure, and the ordinary style calls inside it. `hover` comes from
    // `InteractiveElement`, so this is a `div` and not a widget — which is
    // exactly why the catalogue offers it on the containers alone.
    let _ = div().hover(|this| {
        this.bg(rgb(0x111111))
            .text_color(rgb(0x222222))
            .border_color(rgb(0x333333))
            .opacity(0.5)
            .rounded_none()
            .rounded_sm()
            .rounded_md()
            .rounded_lg()
            .rounded_full()
            .shadow_sm()
            .shadow_md()
            .shadow_lg()
            .shadow_xl()
    });

    // A tooltip, in the only place gpui offers one: on a stateful element, so
    // after `id` — the same order the scroll needs, and for the same reason.
    let _ = div().id("tip").tooltip(|window, cx| Tooltip::new("Hint").build(window, cx));

    // The visible scrollbar, in the shape the box's property writes: one handle
    // shared by the box that scrolls and the bar drawn over it, inside a
    // positioned parent — `Scrollbar` is not `Styled`, so the `div` is what
    // places it.
    let handle = ScrollHandle::new();
    // The bar is a *sibling* of the box that scrolls, under a `relative`
    // wrapper: `Div::prepaint` moves every child of a scrolling element by the
    // scroll offset, an absolutely positioned one included, so a bar written
    // inside the box travels with the content and leaves the screen. This is
    // the shape `gpui-component` mounts its own with.
    let _ = div()
        .relative()
        .h_full()
        .child(div().id("scroller").overflow_y_scroll().track_scroll(&handle))
        .child(
            div()
                .absolute()
                .top_0()
                .left_0()
                .right_0()
                .bottom_0()
                .child(Scrollbar::new(&handle).id("bar").axis(ScrollbarAxis::Vertical)),
        );
    let _ = Scrollbar::new(&handle).axis(ScrollbarAxis::Horizontal);
    let _ = Scrollbar::new(&handle).axis(ScrollbarAxis::Both);

    // Every component's own calls, on its own type — which is the half a
    // generic `Styled` check cannot answer.
    let _ = Label::new("Label");

    let _ = Button::new("button")
        .label("Button")
        .primary()
        .danger()
        .outline()
        .ghost()
        .link()
        .tooltip("Tooltip")
        .disabled(true)
        .on_click(|_event: &gpui::ClickEvent, _window, _cx| {});

    let _ = Checkbox::new("checkbox")
        .label("Checkbox")
        .checked(true)
        .on_click(|_on: &bool, _window, _cx| {});

    let _ =
        Switch::new("switch").label("Switch").checked(true).on_click(|_on: &bool, _window, _cx| {});

    let _ = Radio::new("radio")
        .label("Radio")
        .checked(true)
        .disabled(true)
        .on_click(|_on: &bool, _window, _cx| {});

    let _ = GroupBox::new().title("Group");
    let _ = Divider::horizontal().label("Divider");
    let _ = Link::new("link").href("https://example.org").disabled(true);
    let _ = Alert::new("alert", "Something happened").title("Note");
    let _ = Tag::new()
        .with_variant(TagVariant::Primary)
        .with_variant(TagVariant::Secondary)
        .with_variant(TagVariant::Danger)
        .with_variant(TagVariant::Success)
        .with_variant(TagVariant::Warning)
        .with_variant(TagVariant::Info)
        .outline()
        .rounded_full();
    let _ = Progress::new().value(50.);

    // A placeholder and a spinner: no argument, and no `Styled` on the second —
    // which is why the catalogue gives it `Common::None`, and why the shared
    // style calls above are not written on it here either.
    let _ = Skeleton::new().h_4().secondary();
    let _ = Spinner::new();

    // The badge is the other one outside `Styled`; what it takes are two whole
    // numbers, which is the whole reason `Kind::Count` exists — `.count(3.)`
    // does not compile.
    let _ = Badge::new().count(3).max(99).child(div());

    // The avatar: a picture read from the project's assets, a name it falls
    // back to, and the three sizes `Sizable` names — the fourth, medium, is
    // what an empty choice leaves.
    let _ =
        Avatar::new().src("assets/images/image.png").name("Ada Lovelace").xsmall().small().large();

    // The two whose items are a type of their own rather than elements: what
    // they take from maxx is an array of literals, which is the only spelling
    // that lets one node hold the whole component.
    let _ = Breadcrumb::new().children(["Home", "Files"]);
    let _ = TabBar::new("tabs")
        .children(["First", "Second"])
        .selected_index(0)
        .with_variant(TabVariant::Tab)
        .with_variant(TabVariant::Outline)
        .with_variant(TabVariant::Pill)
        .with_variant(TabVariant::Segmented)
        .with_variant(TabVariant::Underline);

    // The keystroke, in the shape the property writes it: parsed at runtime
    // because `Keystroke` has no literal, and falling back rather than
    // panicking in an application maxx does not own.
    let _ = Kbd::new(Keystroke::parse("cmd-k").unwrap_or_default());

    // What it copies, and the name gpui keeps its "copied" flag under.
    let _ = Clipboard::new("clipboard").value("Something to copy");

    // The icon, and its colour — which is a text colour, because an icon is an
    // `svg` gpui tints. The eighty-six names themselves are not written out
    // here: `build.rs` generates the match `designer::canvas` draws from, and
    // the compiler reads every variant there.
    let _ = Icon::new(IconName::Check).text_color(rgb(0x333333));
}

/// The components that take an entity of their own.
///
/// Apart from `main` because they need a `Context` to build their state in,
/// which is exactly what the generated view's `new` gives them — the shape
/// checked here is the one `StateSpec` writes, initializer included: the
/// multi-line switch and the dropdown's entries are arguments of a constructor
/// called here and nowhere else.
fn stateful(window: &mut gpui::Window, cx: &mut gpui::Context<Holder>) -> Holder {
    let slider = cx.new(|_| SliderState::new().min(0.).max(100.).step(1.).default_value(50.));
    let picker = cx.new(|cx| ColorPickerState::new(window, cx));
    let date = cx.new(|cx| DatePickerState::new(window, cx));
    let month = cx.new(|cx| CalendarState::new(window, cx));
    let number = cx.new(|cx| InputState::new(window, cx));
    let code = cx.new(|cx| OtpState::new(6, window, cx));
    // The other shape the text input's state comes in — a property of the
    // field, since the element has no such call.
    let lines = cx.new(|cx| InputState::new(window, cx).multi_line(true));

    let _ = Slider::new(&slider).horizontal().vertical().disabled(true);
    let _ = ColorPicker::new(&picker).label("Colour");
    let _ = DatePicker::new(&date).placeholder("Date").disabled(true);
    let _ = Calendar::new(&month).number_of_months(2);
    let _ = NumberInput::new(&number).placeholder("0").disabled(true);
    let _ = OtpInput::new(&code).groups(3).disabled(true);
    let _ = Input::new(&lines);

    // The dropdown: the element, and the initializer its Items property writes.
    // The initializer is the part no other file holds — it is not a call on the
    // element but the constructor this view's `new` runs, so this is the only
    // place a compiler ever reads it.
    let choices = cx.new(|cx| {
        SelectState::new(
            SearchableVec::new(vec![SharedString::from("First"), SharedString::from("Second")]),
            Some(IndexPath::new(0)),
            window,
            cx,
        )
    });
    let _ = Select::new(&choices);

    Holder { slider, picker, date, month, number, code, lines, choices }
}

/// The view the entities hang from, as `view::save` declares it.
struct Holder {
    slider: gpui::Entity<SliderState>,
    picker: gpui::Entity<ColorPickerState>,
    date: gpui::Entity<DatePickerState>,
    month: gpui::Entity<CalendarState>,
    number: gpui::Entity<InputState>,
    code: gpui::Entity<OtpState>,
    lines: gpui::Entity<InputState>,
    choices: gpui::Entity<SelectState<SearchableVec<SharedString>>>,
}

impl gpui::Render for Holder {
    fn render(
        &mut self,
        window: &mut gpui::Window,
        cx: &mut gpui::Context<Self>,
    ) -> impl IntoElement {
        let _ = stateful(window, cx);
        div()
            .child(Slider::new(&self.slider))
            .child(ColorPicker::new(&self.picker))
            .child(DatePicker::new(&self.date))
            .child(Calendar::new(&self.month))
            .child(NumberInput::new(&self.number))
            .child(OtpInput::new(&self.code))
            .child(Input::new(&self.lines))
            .child(Select::new(&self.choices))
    }
}
