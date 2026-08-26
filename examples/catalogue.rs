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

use gpui::prelude::*;
use gpui::{FontWeight, ObjectFit, div, img, px, rgb};
use gpui_component::Disableable;
use gpui_component::alert::Alert;
use gpui_component::button::{Button, ButtonVariants};
use gpui_component::checkbox::Checkbox;
use gpui_component::divider::Divider;
use gpui_component::group_box::GroupBox;
use gpui_component::label::Label;
use gpui_component::link::Link;
use gpui_component::progress::Progress;
use gpui_component::radio::Radio;
use gpui_component::switch::Switch;
use gpui_component::tag::Tag;

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
    let _ = Tag::new().outline().rounded_full();
    let _ = Progress::new().value(50.);
}
