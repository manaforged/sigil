use super::*;

#[test]
fn text_edit_multiline() {
    let img = render_widget_to_png_stub(
        |cx| {
            let buffer = signal(String::from("first line\nsecond line\nthird line"));
            cx.add(text_edit(buffer).width(360.0).height(160.0))
        },
        InteractionMode::Default,
        [220, 88],
        (60.0, 18.0),
    );
    assert_rendered_matches_golden(img, &golden_path!("text_edit_multiline"), DELTA_PCT);
}

fn chip_in(label: &'static str, tone: Tone) -> NodeBuilder {
    container()
        .layout(LayoutStyle::flex_row())
        .child(chip(label, tone))
}

#[test]
fn chip_neutral() {
    let img = render_widget_to_png_stub(
        |cx| cx.add(chip_in("READY", Tone::Neutral)),
        InteractionMode::Default,
        [50, 35],
        (42.0, 29.4),
    );
    assert_rendered_matches_golden(img, &golden_path!("chip_neutral"), DELTA_PCT);
}

#[test]
fn chip_info() {
    let img = render_widget_to_png_stub(
        |cx| cx.add(chip_in("INFO", Tone::Info)),
        InteractionMode::Default,
        [50, 35],
        (42.0, 29.4),
    );
    assert_rendered_matches_golden(img, &golden_path!("chip_info"), DELTA_PCT);
}

#[test]
fn chip_success() {
    let img = render_widget_to_png_stub(
        |cx| cx.add(chip_in("OK", Tone::Success)),
        InteractionMode::Default,
        [50, 35],
        (42.0, 29.4),
    );
    assert_rendered_matches_golden(img, &golden_path!("chip_success"), DELTA_PCT);
}

#[test]
fn chip_warn() {
    let img = render_widget_to_png_stub(
        |cx| cx.add(chip_in("WARN", Tone::Warn)),
        InteractionMode::Default,
        [50, 35],
        (42.0, 29.4),
    );
    assert_rendered_matches_golden(img, &golden_path!("chip_warn"), DELTA_PCT);
}

#[test]
fn chip_error() {
    let img = render_widget_to_png_stub(
        |cx| cx.add(chip_in("ERROR", Tone::Error)),
        InteractionMode::Default,
        [50, 35],
        (42.0, 29.4),
    );
    assert_rendered_matches_golden(img, &golden_path!("chip_error"), DELTA_PCT);
}

#[test]
fn divider_horizontal_default() {
    let img = render_widget_to_png_stub(
        |cx| {
            cx.add(
                container()
                    .layout(LayoutStyle::flex_column().fixed_size(180.0, 60.0))
                    .child(
                        container()
                            .class("bg-accent")
                            .layout(LayoutStyle::flex_row().fill_width().height(20.0)),
                    )
                    .child(divider_horizontal())
                    .child(
                        container()
                            .class("bg-surface-hover")
                            .layout(LayoutStyle::flex_row().fill_width().height(20.0)),
                    ),
            )
        },
        InteractionMode::Default,
        [192, 72],
        (90.0, 30.0),
    );
    assert_rendered_matches_golden(img, &golden_path!("divider_horizontal_default"), DELTA_PCT);
}

#[test]
fn divider_vertical_default() {
    let img = render_widget_to_png_stub(
        |cx| {
            cx.add(
                container()
                    .layout(LayoutStyle::flex_row().fixed_size(120.0, 60.0))
                    .child(
                        container()
                            .class("bg-accent")
                            .layout(LayoutStyle::flex_column().fill_height().width(40.0)),
                    )
                    .child(divider_vertical())
                    .child(
                        container()
                            .class("bg-surface-hover")
                            .layout(LayoutStyle::flex_column().fill_height().width(40.0)),
                    ),
            )
        },
        InteractionMode::Default,
        [136, 72],
        (80.0, 32.0),
    );
    assert_rendered_matches_golden(img, &golden_path!("divider_vertical_default"), DELTA_PCT);
}

#[test]
fn kv_row_default() {
    let img = render_widget_to_png_stub(
        |cx| {
            let row = container()
                .layout(LayoutStyle::flex_column().fixed_size(220.0, 40.0))
                .class("bg-surface")
                .child(kv_row("Status", text("200 OK").class("font-mono")));
            cx.add(on_black(row, 232.0, 52.0))
        },
        InteractionMode::Default,
        [232, 52],
        (116.0, 26.0),
    );
    assert_rendered_matches_golden(img, &golden_path!("kv_row_default"), DELTA_PCT);
}

#[test]
fn status_bar_default() {
    let img = render_widget_to_png_stub(
        |cx| {
            let bar = container()
                .layout(LayoutStyle::flex_column().fixed_size(280.0, 48.0))
                .class("bg-surface")
                .child(status_bar([
                    chip("READY", Tone::Success),
                    chip("3 ITEMS", Tone::Neutral),
                ]));
            cx.add(on_black(bar, 292.0, 60.0))
        },
        InteractionMode::Default,
        [292, 60],
        (146.0, 30.0),
    );
    assert_rendered_matches_golden(img, &golden_path!("status_bar_default"), DELTA_PCT);
}

#[test]
fn number_field_default() {
    let img = render_widget_to_png_stub(
        |cx| {
            let value = signal(42i32);
            cx.add(number_field(value, 0..=100).text_field())
        },
        InteractionMode::Default,
        [192, 44],
        (40.0, 22.0),
    );
    assert_rendered_matches_golden(img, &golden_path!("number_field_default"), DELTA_PCT);
}

#[test]
fn number_field_focused() {
    let img = render_widget_to_png_stub(
        |cx| {
            let value = signal(42i32);
            cx.add(number_field(value, 0..=100).text_field())
        },
        InteractionMode::Focused,
        [192, 44],
        (40.0, 22.0),
    );
    assert_rendered_matches_golden(img, &golden_path!("number_field_focused"), DELTA_PCT);
}
