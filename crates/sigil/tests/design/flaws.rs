use super::design_support::page;
use sigil::layout::audit::{Flaw, FlawKind};
use sigil::prelude::*;
use sigil_kittest::Harness;

fn kinds(ui: &mut Harness) -> Vec<FlawKind> {
    ui.flaws().iter().map(Flaw::kind).collect()
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
    assert!(kinds(&mut ui).contains(&FlawKind::ClippedText));
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
    assert!(kinds(&mut ui).contains(&FlawKind::UnevenControls));
}

#[test]
fn siblings_on_top_of_each_other_are_a_flaw() {
    let mut ui = Harness::new(|| {
        page(column().gap(Space::None).child(button("First")).child(
            button("Second").layout(LayoutStyle::flex_row().margin_top(Dimension::Px(-20.0))),
        ))
    })
    .pixels(360, 160)
    .unaudited();
    assert!(kinds(&mut ui).contains(&FlawKind::Overlap));
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
    assert!(kinds(&mut ui).contains(&FlawKind::Overflow));
}

#[test]
fn faint_text_is_a_flaw_and_theme_text_is_not() {
    let faint = Rgba8 {
        r: 200,
        g: 200,
        b: 200,
        a: 255,
    };
    let mut bad = Harness::new(move || {
        page(text("Faint").text_color(faint)).bg(Rgba8 {
            r: 255,
            g: 255,
            b: 255,
            a: 255,
        })
    })
    .pixels(360, 160)
    .unaudited();
    assert!(kinds(&mut bad).contains(&FlawKind::Contrast));
    let mut good =
        Harness::new(|| page(text("Clear").text_color(Token::OnSurface))).pixels(360, 160);
    assert!(!kinds(&mut good).contains(&FlawKind::Contrast));
}

#[test]
fn a_button_under_the_target_size_is_a_flaw() {
    let mut ui = Harness::new(|| page(row().child(button("Hi").height(16.0))))
        .pixels(360, 160)
        .unaudited();
    assert!(kinds(&mut ui).contains(&FlawKind::SmallTarget));
}

#[test]
fn allow_flaw_opts_a_node_out_of_one_rule() {
    let mut ui = Harness::new(|| {
        page(
            row()
                .child(button("Hi").height(16.0))
                .allow_flaw(FlawKind::SmallTarget, "dense toolbar"),
        )
    })
    .pixels(360, 160);
    assert!(!kinds(&mut ui).contains(&FlawKind::SmallTarget));
}

#[test]
#[should_panic(expected = "design flaws")]
fn a_harness_fails_a_flawed_screen_when_it_is_dropped() {
    let mut ui = Harness::new(|| page(row().child(button("Hi").height(16.0)))).pixels(360, 160);
    ui.settle();
}
