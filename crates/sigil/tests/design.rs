#![allow(clippy::unwrap_used, clippy::panic)]

#[path = "design_support/mod.rs"]
mod design_support;

use design_support::{Ink, frame, ink, page, shadow_rows};
use sigil::layout::audit::{Flaw, Issue};
use sigil::prelude::*;
use sigil::widgets::ButtonBuilder;
use sigil_kittest::Harness;

fn field_of(value: &str, font: f32, height: Option<f32>) -> impl IntoNode + use<> {
    let field = text_field(signal(value.to_owned()))
        .font_size(font, "test fixture")
        .caret_visible(signal(false));
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
    let field = ui.get((Role::TextInput, "Text field"));
    let saved = ui.get((Role::Button, "Save"));
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
    let first = ui.get((Role::Button, "First")).bounds().unwrap();
    let second = ui.get((Role::Button, "Second")).bounds().unwrap();
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
    let tall = ui.get((Role::Button, "Tall")).center().unwrap();
    let short = ui.get((Role::Button, "Short")).center().unwrap();
    assert!((tall.y - short.y).abs() < 0.5, "{} and {}", tall.y, short.y);
}

fn kinds(ui: &mut Harness) -> Vec<Flaw> {
    ui.flaws().iter().map(Issue::kind).collect()
}

#[test]
fn text_taller_than_its_clip_is_a_flaw() {
    let mut ui = Harness::new(|| {
        page(
            container()
                .layout(
                    LayoutStyle::block()
                        .fill_width()
                        .height(10.0)
                        .overflow_y(Overflow::Hidden),
                )
                .child(text("Clipped").text_role(TextRole::Display)),
        )
    })
    .pixels(360, 160)
    .unaudited();
    assert!(kinds(&mut ui).contains(&Flaw::ClippedText));
}

#[test]
fn controls_of_two_heights_in_a_row_are_a_flaw() {
    let mut ui = Harness::new(|| {
        page(
            row()
                .child(button("Tall").height(64.0))
                .child(button("Short")),
        )
    })
    .pixels(360, 160)
    .unaudited();
    assert!(kinds(&mut ui).contains(&Flaw::UnevenControls));
}

#[test]
fn siblings_on_top_of_each_other_are_a_flaw() {
    let mut ui =
        Harness::new(|| {
            page(column().gap(Space::None).child(button("First")).child(
                button("Second").layout(LayoutStyle::flex_row().margin_top(Length::Px(-20.0))),
            ))
        })
        .pixels(360, 160)
        .unaudited();
    assert!(kinds(&mut ui).contains(&Flaw::Overlap));
}

#[test]
fn a_child_wider_than_its_parent_is_a_flaw() {
    let mut ui = Harness::new(|| {
        page(
            container()
                .layout(LayoutStyle::block().width(40.0).height(40.0))
                .child(container().layout(LayoutStyle::block().width(200.0).height(20.0))),
        )
    })
    .pixels(360, 160)
    .unaudited();
    assert!(kinds(&mut ui).contains(&Flaw::Overflow));
}

#[test]
fn faint_text_is_a_flaw_and_theme_text_is_not() {
    let faint = Color {
        r: 200,
        g: 200,
        b: 200,
        a: 255,
    };
    let mut bad = Harness::new(move || {
        page(text("Faint").text_color(faint)).background(Color {
            r: 255,
            g: 255,
            b: 255,
            a: 255,
        })
    })
    .pixels(360, 160)
    .unaudited();
    assert!(kinds(&mut bad).contains(&Flaw::Contrast));
    let mut good =
        Harness::new(|| page(text("Clear").text_color(Token::OnSurface))).pixels(360, 160);
    assert!(!kinds(&mut good).contains(&Flaw::Contrast));
}

#[test]
fn a_button_under_the_target_size_is_a_flaw() {
    let mut ui = Harness::new(|| page(row().child(button("Hi").height(16.0))))
        .pixels(360, 160)
        .unaudited();
    assert!(kinds(&mut ui).contains(&Flaw::SmallTarget));
}

#[test]
fn allow_flaw_opts_a_node_out_of_one_rule() {
    let mut ui = Harness::new(|| {
        page(
            row()
                .child(button("Hi").height(16.0))
                .allow_flaw(Flaw::SmallTarget, "dense toolbar"),
        )
    })
    .pixels(360, 160);
    assert!(!kinds(&mut ui).contains(&Flaw::SmallTarget));
}

#[test]
#[should_panic(expected = "design flaws")]
fn a_harness_fails_a_flawed_screen_when_it_is_dropped() {
    let mut ui = Harness::new(|| page(row().child(button("Hi").height(16.0)))).pixels(360, 160);
    ui.settle();
}

#[path = "design/rows.rs"]
mod rows;
