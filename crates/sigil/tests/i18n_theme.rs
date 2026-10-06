#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]
#![allow(clippy::unwrap_used, clippy::expect_used)]

use sigil::events::Key;
use sigil::kernel::Role;
use sigil::prelude::*;
use sigil::style::{Direction, Locale, StringKey, Strings, Theme};
use sigil_kittest::{Harness, Query};

fn first_button_x(direction: Direction) -> f32 {
    let mut ui = Harness::new(move || {
        sigil::app::set_theme(Theme::light().with_direction(direction));
        column()
            .layout(LayoutStyle::flex_column().fill())
            .child(row().child(button("Back")).child(button("Next")))
    })
    .pixels(600, 120);
    ui.settle();
    ui.get_by_role(Role::Button, "Back")
        .bounds()
        .expect("bounds")
        .origin
        .x
}

#[test]
fn right_to_left_moves_a_row_to_the_right_edge() {
    let ltr = first_button_x(Direction::Ltr);
    let rtl = first_button_x(Direction::Rtl);
    assert!(rtl > ltr + 300.0, "ltr {ltr}, rtl {rtl}");
}

#[test]
fn a_number_field_reads_the_theme_locale() {
    let value = Rc::new(Cell::new(None::<Signal<f64>>));
    let export = Rc::clone(&value);
    let mut ui = Harness::new(move || {
        sigil::app::set_theme(Theme::light().with_locale(Locale::de()));
        let amount = signal(0.0_f64);
        export.set(Some(amount));
        number_field(amount, 0.0..=1_000_000.0)
    })
    .pixels(400, 80);
    ui.settle();
    let input = ui.get(&Query::role(Role::TextInput));
    ui.click(&input)
        .type_text("1.234,56")
        .press(Key::Enter)
        .settle();
    let read = value.get().expect("value signal").peek();
    assert!((read - 1234.56).abs() < 0.01, "read {read}");
}

#[test]
fn built_in_labels_come_from_the_theme_strings() {
    let mut ui = Harness::new(|| {
        sigil::app::set_theme(
            Theme::light().with_strings(Strings::english().with(StringKey::Close, "Schließen")),
        );
        modal(signal(true)).title("Dialog").child(text("body"))
    })
    .pixels(400, 300);
    ui.settle();
    let names: Vec<String> = ui
        .find_all(&Query::role(Role::Button))
        .into_iter()
        .filter_map(|node| node.name().map(str::to_owned))
        .collect();
    assert!(names.iter().any(|name| name == "Schließen"), "{names:?}");
    assert!(!names.iter().any(|name| name == "Close"), "{names:?}");
}

use std::cell::Cell;
use std::rc::Rc;
