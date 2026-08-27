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
use gpui::{FontWeight, ObjectFit, ScrollHandle, div, img, px, rgb};
use gpui_component::Disableable;
use gpui_component::alert::Alert;
use gpui_component::badge::Badge;
use gpui_component::button::{Button, ButtonVariants};
use gpui_component::checkbox::Checkbox;
use gpui_component::color_picker::{ColorPicker, ColorPickerState};
use gpui_component::divider::Divider;
use gpui_component::group_box::GroupBox;
use gpui_component::label::Label;
use gpui_component::link::Link;
use gpui_component::progress::Progress;
use gpui_component::radio::Radio;
use gpui_component::scroll::{Scrollbar, ScrollbarAxis};
use gpui_component::skeleton::Skeleton;
use gpui_component::slider::{Slider, SliderState};
use gpui_component::spinner::Spinner;
use gpui_component::switch::Switch;
use gpui_component::tag::{Tag, TagVariant};
use gpui_component::tooltip::Tooltip;
use gpui_component::{Icon, IconName};

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
        .flex_1();

    // The shared style properties.
    let _ = div()
        .w(px(1.))
        .h(px(1.))
        .bg(rgb(0x111111))
        .text_color(rgb(0x222222))
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

    // Every icon the inspector offers, each written as the file will carry it.
    for name in [
        IconName::Check,
        IconName::Close,
        IconName::Search,
        IconName::Settings,
        IconName::Plus,
        IconName::Minus,
        IconName::Info,
        IconName::TriangleAlert,
        IconName::CircleCheck,
        IconName::CircleX,
        IconName::Star,
        IconName::Heart,
        IconName::Bell,
        IconName::Calendar,
        IconName::File,
        IconName::Folder,
        IconName::Globe,
        IconName::User,
        IconName::Copy,
        IconName::Delete,
        IconName::Eye,
        IconName::ArrowRight,
    ] {
        let _ = Icon::new(name).text_color(rgb(0x333333));
    }
}

/// The two components that take an entity of their own.
///
/// Apart from `main` because they need a `Context` to build their state in,
/// which is exactly what the generated view's `new` gives them — the shape
/// checked here is the one `StateSpec` writes.
fn stateful(window: &mut gpui::Window, cx: &mut gpui::Context<Holder>) -> Holder {
    let slider = cx.new(|_| SliderState::new().min(0.).max(100.).step(1.).default_value(50.));
    let picker = cx.new(|cx| ColorPickerState::new(window, cx));

    let _ = Slider::new(&slider).horizontal().vertical().disabled(true);
    let _ = ColorPicker::new(&picker).label("Colour");

    Holder { slider, picker }
}

/// The view the two entities hang from, as `view::save` declares it.
struct Holder {
    slider: gpui::Entity<SliderState>,
    picker: gpui::Entity<ColorPickerState>,
}

impl gpui::Render for Holder {
    fn render(
        &mut self,
        window: &mut gpui::Window,
        cx: &mut gpui::Context<Self>,
    ) -> impl IntoElement {
        let _ = stateful(window, cx);
        div().child(Slider::new(&self.slider)).child(ColorPicker::new(&self.picker))
    }
}
