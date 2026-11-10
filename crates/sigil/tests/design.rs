#![allow(clippy::unwrap_used, clippy::panic)]

#[path = "design_support/mod.rs"]
mod design_support;
#[path = "design/flaws.rs"]
mod flaws;

use design_support::{Ink, frame, ink, page, shadow_rows};
use sigil::prelude::*;
use sigil::widgets::ButtonBuilder;
use sigil_kittest::Harness;

fn field_of(value: &str, font: f32, height: Option<f32>) -> impl IntoNode + use<> {
    let mut field = text_field(signal(value.to_owned())).font_size(font, "test fixture");
    field.caret_visible().set(false);
    let field = match height {
        Some(height) => field.height(height),
        None => field,
    };
    page(field)
}

fn blank() -> NodeBuilder {
    container().layout(LayoutStyle::block().fill_width().height(40.0))
}

fn plain_rows(value: &'static str, font: f32) -> u32 {
    let shot = frame(move || {
        page(
            text(value)
                .font_size(font, "test fixture")
                .text_color(Token::OnSurface),
        )
    });
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

#[test]
fn a_card_casts_a_shadow_on_the_page() {
    let flat = frame(|| page(blank()));
    let raised = frame(|| page(card(text("Hg"))));
    assert_eq!(shadow_rows(&flat), 0, "the bare page has no shadow");
    assert!(shadow_rows(&raised) > 0, "a card has no shadow");
}

#[test]
fn a_raised_layer_casts_a_shadow_and_a_page_layer_does_not() {
    let raised = frame(|| page(blank().layer(Layer::Raised)));
    let flat = frame(|| page(blank().layer(Layer::Page)));
    assert!(shadow_rows(&raised) > 0);
    assert_eq!(shadow_rows(&flat), 0);
}

#[test]
fn each_panel_elevation_casts_a_deeper_shadow() {
    let depth = |tier| {
        shadow_rows(&frame(move || {
            page(panel().elevation(tier).child(text("Hg")))
        }))
    };
    let (flat, low, mid, high) = (depth(0), depth(1), depth(2), depth(3));
    assert_eq!(flat, 0);
    assert!(
        low > 0 && low < mid && mid < high,
        "shadow rows by tier: {low}, {mid}, {high}"
    );
}

#[test]
fn a_card_on_the_terminal_keeps_its_border_and_draws_no_shadow() {
    let mut ui = Harness::new(|| card(text("Hg")));
    let screen = ui.text();
    assert!(screen.contains('╭') && screen.contains('╰'), "{screen}");
    assert!(!screen.contains(['░', '▒', '▓']), "{screen}");
}

#[test]
fn a_gap_step_is_the_distance_between_children() {
    let mut ui = Harness::new(|| {
        page(
            column()
                .gap(Space::Xl)
                .child(button("First"))
                .child(button("Second")),
        )
    })
    .pixels(360, 160);
    let first = ui.get_by_role(Role::Button, "First").bounds().unwrap();
    let second = ui.get_by_role(Role::Button, "Second").bounds().unwrap();
    let between = second.origin.y - (first.origin.y + first.height);
    assert!((between - 24.0).abs() < 0.5, "gap is {between}");
}

#[test]
fn each_text_role_is_larger_than_the_one_below_it() {
    let height = |role| {
        ink(&frame(move || {
            page(text("Hg").text_role(role).text_color(Token::OnSurface))
        }))
        .rows
    };
    let heights = [
        height(TextRole::Caption),
        height(TextRole::Body),
        height(TextRole::Subtitle),
        height(TextRole::Title),
        height(TextRole::Heading),
        height(TextRole::Display),
    ];
    assert!(heights.is_sorted_by(|low, high| low < high), "{heights:?}");
}

#[test]
fn a_row_centres_short_children_against_tall_ones() {
    let mut ui = Harness::new(|| {
        page(
            row()
                .child(button("Tall").height(64.0))
                .child(button("Short")),
        )
    })
    .pixels(360, 160)
    .unaudited();
    let tall = ui.get_by_role(Role::Button, "Tall").center().unwrap();
    let short = ui.get_by_role(Role::Button, "Short").center().unwrap();
    assert!((tall.y - short.y).abs() < 0.5, "{} and {}", tall.y, short.y);
}

const LONG: &str = "A sentence long enough to need more than one line in a narrow window";

#[test]
fn long_text_in_a_row_wraps_inside_the_window_and_pushes_later_widgets_down() {
    let mut ui = Harness::new(|| {
        column()
            .child(
                row()
                    .child(button("A"))
                    .child(text(LONG))
                    .child(button("B")),
            )
            .child(button("Below"))
    })
    .pixels(200, 400);
    let last = ui.get_by_role(Role::Button, "B").bounds().unwrap();
    let below = ui.get_by_role(Role::Button, "Below").bounds().unwrap();
    assert!(last.origin.x + last.width <= 200.5, "B ends at {last:?}");
    assert!(below.width <= 200.5, "the column is {} wide", below.width);
    assert!(
        below.origin.y >= last.origin.y + last.height,
        "{below:?} under {last:?}"
    );
    assert!(
        below.origin.y >= 3.0 * 16.0,
        "three lines of text end above {below:?}"
    );
}

#[test]
fn a_text_field_keeps_a_usable_width_inside_a_nested_row() {
    let mut ui = Harness::new(|| {
        row()
            .child(
                row()
                    .child(text_field(signal(String::new())))
                    .child(button("Go")),
            )
            .child(text("Words"))
    })
    .pixels(1280, 200);
    let field = ui
        .get_by_role(Role::TextInput, "Text field")
        .bounds()
        .unwrap();
    assert!(field.width >= 96.0, "the field is {} wide", field.width);
}

#[test]
fn a_button_keeps_its_label_when_a_field_fills_its_row() {
    let mut ui = Harness::new(|| {
        row()
            .child(text_field(signal(String::new())))
            .child(button("Save changes"))
    })
    .pixels(360, 200);
    let alone = {
        let mut lone = Harness::new(|| row().child(button("Save changes"))).pixels(360, 200);
        lone.get_by_role(Role::Button, "Save changes")
            .bounds()
            .unwrap()
            .width
    };
    let saved = ui
        .get_by_role(Role::Button, "Save changes")
        .bounds()
        .unwrap();
    assert!(
        (saved.width - alone).abs() < 0.5,
        "{} and {alone}",
        saved.width
    );
}

#[test]
fn every_button_fill_has_the_same_width() {
    let width = |fill: fn(ButtonBuilder) -> ButtonBuilder| {
        let mut ui = Harness::new(move || fill(button("Go"))).pixels(200, 80);
        ui.get_by_role(Role::Button, "Go").bounds().unwrap().width
    };
    let plain = width(|b| b);
    for (name, fill) in [
        (
            "primary",
            ButtonBuilder::primary as fn(ButtonBuilder) -> ButtonBuilder,
        ),
        ("outline", ButtonBuilder::outline),
        ("ghost", ButtonBuilder::ghost),
    ] {
        assert!((width(fill) - plain).abs() < 0.5, "{name}");
    }
}
