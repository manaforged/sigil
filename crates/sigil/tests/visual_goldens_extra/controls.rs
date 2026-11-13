use super::*;
use sigil::style::FontFamily;

#[test]
fn text_edit_multiline() {
    let img = Harness::render(|| {
        let buffer = signal(String::from("first line\nsecond line\nthird line"));
        text_field(buffer).multiline().width(360.0).height(160.0)
    })
    .stub()
    .mode(InteractionMode::Default)
    .size([220, 88])
    .hit((60.0, 18.0))
    .image();
    Golden::new("text_edit_multiline").assert(img);
}

fn chip_in(label: &'static str, tone: Tone) -> NodeBuilder {
    container()
        .layout(LayoutStyle::flex_row())
        .child(chip(label, tone))
}

#[test]
fn chip_neutral() {
    let img = Harness::render(|| chip_in("READY", Tone::Neutral))
        .stub()
        .mode(InteractionMode::Default)
        .size([50, 35])
        .hit((42.0, 29.4))
        .image();
    Golden::new("chip_neutral").assert(img);
}

#[test]
fn chip_info() {
    let img = Harness::render(|| chip_in("INFO", Tone::Info))
        .stub()
        .mode(InteractionMode::Default)
        .size([50, 35])
        .hit((42.0, 29.4))
        .image();
    Golden::new("chip_info").assert(img);
}

#[test]
fn chip_success() {
    let img = Harness::render(|| chip_in("OK", Tone::Success))
        .stub()
        .mode(InteractionMode::Default)
        .size([50, 35])
        .hit((42.0, 29.4))
        .image();
    Golden::new("chip_success").assert(img);
}

#[test]
fn chip_warn() {
    let img = Harness::render(|| chip_in("WARN", Tone::Warning))
        .stub()
        .mode(InteractionMode::Default)
        .size([50, 35])
        .hit((42.0, 29.4))
        .image();
    Golden::new("chip_warn").assert(img);
}

#[test]
fn chip_error() {
    let img = Harness::render(|| chip_in("ERROR", Tone::Error))
        .stub()
        .mode(InteractionMode::Default)
        .size([50, 35])
        .hit((42.0, 29.4))
        .image();
    Golden::new("chip_error").assert(img);
}

#[test]
fn divider_horizontal_default() {
    let img = Harness::render(|| {
        container()
            .layout(LayoutStyle::flex_column().fixed_size(180.0, 60.0))
            .child(
                container()
                    .background(Token::Accent)
                    .layout(LayoutStyle::flex_row().fill_width().height(20.0)),
            )
            .child(divider(Orientation::Horizontal))
            .child(
                container()
                    .background(Token::SurfaceHover)
                    .layout(LayoutStyle::flex_row().fill_width().height(20.0)),
            )
    })
    .stub()
    .mode(InteractionMode::Default)
    .size([192, 72])
    .hit((90.0, 30.0))
    .image();
    Golden::new("divider_horizontal_default").assert(img);
}

#[test]
fn divider_vertical_default() {
    let img = Harness::render(|| {
        container()
            .layout(LayoutStyle::flex_row().fixed_size(120.0, 60.0))
            .child(
                container()
                    .background(Token::Accent)
                    .layout(LayoutStyle::flex_column().fill_height().width(40.0)),
            )
            .child(divider(Orientation::Vertical))
            .child(
                container()
                    .background(Token::SurfaceHover)
                    .layout(LayoutStyle::flex_column().fill_height().width(40.0)),
            )
    })
    .stub()
    .mode(InteractionMode::Default)
    .size([136, 72])
    .hit((80.0, 32.0))
    .image();
    Golden::new("divider_vertical_default").assert(img);
}

#[test]
fn kv_row_default() {
    let img = Harness::render(|| {
        let row = container()
            .layout(LayoutStyle::flex_column().fixed_size(220.0, 40.0))
            .background(Token::Surface)
            .child(kv_row(
                "Status",
                text("200 OK").font_family(FontFamily::Mono),
            ));
        on_black(row, 232.0, 52.0)
    })
    .stub()
    .mode(InteractionMode::Default)
    .size([232, 52])
    .hit((116.0, 26.0))
    .image();
    Golden::new("kv_row_default").assert(img);
}

#[test]
fn status_bar_default() {
    let img = Harness::render(|| {
        let bar = container()
            .layout(LayoutStyle::flex_column().fixed_size(280.0, 48.0))
            .background(Token::Surface)
            .child(status_bar([
                chip("READY", Tone::Success),
                chip("3 ITEMS", Tone::Neutral),
            ]));
        on_black(bar, 292.0, 60.0)
    })
    .stub()
    .mode(InteractionMode::Default)
    .size([292, 60])
    .hit((146.0, 30.0))
    .image();
    Golden::new("status_bar_default").assert(img);
}

#[test]
fn number_field_default() {
    let img = Harness::render(|| {
        let value = signal(42i32);
        number_field(value, 0..=100).text_field()
    })
    .stub()
    .mode(InteractionMode::Default)
    .size([192, 44])
    .hit((40.0, 22.0))
    .image();
    Golden::new("number_field_default").assert(img);
}

#[test]
fn number_field_focused() {
    let img = Harness::render(|| {
        let value = signal(42i32);
        number_field(value, 0..=100).text_field()
    })
    .stub()
    .mode(InteractionMode::Focused)
    .size([192, 44])
    .hit((40.0, 22.0))
    .image();
    Golden::new("number_field_focused").assert(img);
}
