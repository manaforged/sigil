#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]
#![allow(clippy::unwrap_used, clippy::panic)]

#[path = "design_support/mod.rs"]
mod design_support;

use design_support::{Ink, frame, ink, page};
use sigil::prelude::*;
use sigil_kittest::Harness;

fn field_of(value: &str, font: f32, height: Option<f32>) -> impl IntoNode + use<> {
    let mut field = text_field(signal(value.to_owned())).font_size(font);
    field.caret_visible().set(false);
    let field = match height {
        Some(height) => field.height(height),
        None => field,
    };
    page(field)
}

fn plain_rows(value: &'static str, font: f32) -> u32 {
    let shot = frame(move || page(text(value).font_size(font).text_color(Token::OnSurface)));
    ink(&shot).rows
}

fn field_ink(font: f32, height: Option<f32>) -> Ink {
    ink(&frame(move || field_of("Hg", font, height)))
}

#[test]
fn a_field_shows_its_text_whole_at_every_font_size() {
    for font in [12.0, 14.0, 20.0, 28.0] {
        let whole = plain_rows("Hg", font);
        for height in [None, Some(56.0)] {
            let shown = field_ink(font, height);
            assert!(
                shown.rows.abs_diff(whole) <= 1,
                "font {font} height {height:?}: the field shows {} rows of a {whole}-row text",
                shown.rows
            );
        }
    }
}

#[test]
fn a_field_centres_its_text_vertically() {
    for font in [12.0, 14.0, 20.0, 28.0] {
        let shown = field_ink(font, Some(56.0));
        let above = shown.top - shown.box_top;
        let below = shown.box_bottom - shown.bottom;
        assert!(
            above.abs_diff(below) <= 5,
            "font {font}: {above} rows above the text and {below} below"
        );
    }
}

#[test]
fn a_field_and_a_button_in_a_row_are_the_same_height() {
    let mut ui = Harness::new(|| {
        page(
            row()
                .child(text_field(signal(String::new())))
                .child(button("Save")),
        )
    })
    .pixels(360, 160);
    let field = ui.get_by_role(Role::TextInput, "Text field");
    let saved = ui.get_by_role(Role::Button, "Save");
    let height = |node: &sigil_kittest::Node| node.bounds().unwrap().height;
    assert!(height(&field) > 0.0);
    assert!((height(&field) - height(&saved)).abs() < 0.5);
}

#[test]
fn an_empty_field_is_as_tall_as_a_filled_one_at_every_font_size() {
    for font in [14.0, 28.0] {
        let empty = ink(&frame(move || field_of("", font, None)));
        let filled = ink(&frame(move || field_of("Hg", font, None)));
        assert_eq!(
            empty.box_bottom - empty.box_top,
            filled.box_bottom - filled.box_top,
            "font {font}: typing changes the field height"
        );
    }
}

#[test]
fn an_empty_focused_field_shows_a_caret() {
    let empty = ink(&frame(|| {
        page(text_field(signal(String::new())).autofocus())
    }));
    assert!(empty.rows >= 10, "caret shows {} rows", empty.rows);
}
