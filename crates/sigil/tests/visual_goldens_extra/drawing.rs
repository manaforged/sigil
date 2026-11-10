use super::*;
use sigil::layout::Space;
use sigil::widgets::Layout;

#[test]
fn vector_path_shapes() {
    let img = Harness::render(|| {
        let accent = sigil::style::Color {
            r: 80,
            g: 130,
            b: 255,
            a: 255,
        };
        let warn = sigil::style::Color {
            r: 240,
            g: 160,
            b: 40,
            a: 255,
        };
        let row = container()
            .layout(
                LayoutStyle::flex_row()
                    .fixed_size(150.0, 76.0)
                    .gap(Space::Md)
                    .padding(Space::Sm),
            )
            .background(Token::Surface)
            .child(
                path(60, 60)
                    .move_to(30.0, 4.0)
                    .line_to(56.0, 56.0)
                    .line_to(4.0, 56.0)
                    .close()
                    .fill(accent),
            )
            .child(
                path(60, 60)
                    .move_to(6.0, 54.0)
                    .cubic_to(22.0, 6.0, 38.0, 6.0, 54.0, 54.0)
                    .stroke(warn, 5.0),
            );
        on_black(row, 162.0, 88.0)
    })
    .stub()
    .mode(InteractionMode::Default)
    .size([162, 88])
    .hit((1.0, 1.0))
    .image();
    Golden::new("vector_path_shapes")
        .delta(DELTA_PCT)
        .assert(img);
}

#[test]
fn vector_gradient_fill() {
    fn square(a: sigil::style::Color, b: sigil::style::Color, radial: bool) -> PathNode {
        let base = path(60, 60)
            .move_to(0.0, 0.0)
            .line_to(60.0, 0.0)
            .line_to(60.0, 60.0)
            .line_to(0.0, 60.0)
            .close();
        if radial {
            base.fill_radial((30.0, 30.0), 30.0, vec![(0.0, a), (1.0, b)])
        } else {
            base.fill_linear((0.0, 0.0), (60.0, 60.0), vec![(0.0, a), (1.0, b)])
        }
    }
    let img = Harness::render(|| {
        let gold = sigil::style::Color {
            r: 255,
            g: 210,
            b: 60,
            a: 255,
        };
        let rose = sigil::style::Color {
            r: 220,
            g: 40,
            b: 120,
            a: 255,
        };
        let row = container()
            .layout(
                LayoutStyle::flex_row()
                    .fixed_size(150.0, 76.0)
                    .gap(Space::Md)
                    .padding(Space::Sm),
            )
            .background(Token::Surface)
            .child(square(gold, rose, true))
            .child(square(gold, rose, false));
        on_black(row, 162.0, 88.0)
    })
    .stub()
    .mode(InteractionMode::Default)
    .size([162, 88])
    .hit((1.0, 1.0))
    .image();
    Golden::new("vector_gradient_fill")
        .delta(DELTA_PCT)
        .assert(img);
}

#[test]
fn emoji_text_line() {
    let img = Harness::render(|| {
        let panel = container()
            .layout(
                LayoutStyle::flex_column()
                    .fixed_size(300.0, 60.0)
                    .padding(Space::Lg),
            )
            .background(Token::Surface)
            .child(text("OK \u{2705} go \u{1F680}").text_color(Token::OnSurface));
        on_black(panel, 312.0, 72.0)
    })
    .measure(|| Box::new(TextEngineMeasure::default()))
    .mode(InteractionMode::Default)
    .size([312, 72])
    .hit((1.0, 1.0))
    .image();
    Golden::new("emoji_text_line").delta(DELTA_PCT).assert(img);
}
