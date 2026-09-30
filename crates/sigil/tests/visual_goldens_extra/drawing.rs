use super::*;

#[test]
fn vector_path_shapes() {
    let img = render_widget_to_png_stub(
        || {
            let accent = sigil::style::Rgba8 {
                r: 80,
                g: 130,
                b: 255,
                a: 255,
            };
            let warn = sigil::style::Rgba8 {
                r: 240,
                g: 160,
                b: 40,
                a: 255,
            };
            let row = container()
                .layout(
                    LayoutStyle::flex_row()
                        .fixed_size(150.0, 76.0)
                        .gap(12.0)
                        .padding(8.0),
                )
                .bg(Token::Surface)
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
        },
        InteractionMode::Default,
        [162, 88],
        (1.0, 1.0),
    );
    assert_rendered_matches_golden(img, &golden_path!("vector_path_shapes"), DELTA_PCT);
}

#[test]
fn vector_gradient_fill() {
    fn square(a: sigil::style::Rgba8, b: sigil::style::Rgba8, radial: bool) -> PathNode {
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
    let img = render_widget_to_png_stub(
        || {
            let gold = sigil::style::Rgba8 {
                r: 255,
                g: 210,
                b: 60,
                a: 255,
            };
            let rose = sigil::style::Rgba8 {
                r: 220,
                g: 40,
                b: 120,
                a: 255,
            };
            let row = container()
                .layout(
                    LayoutStyle::flex_row()
                        .fixed_size(150.0, 76.0)
                        .gap(12.0)
                        .padding(8.0),
                )
                .bg(Token::Surface)
                .child(square(gold, rose, true))
                .child(square(gold, rose, false));
            on_black(row, 162.0, 88.0)
        },
        InteractionMode::Default,
        [162, 88],
        (1.0, 1.0),
    );
    assert_rendered_matches_golden(img, &golden_path!("vector_gradient_fill"), DELTA_PCT);
}

#[test]
fn emoji_text_line() {
    let img = render_widget_to_png_with(
        || {
            let panel = container()
                .layout(
                    LayoutStyle::flex_column()
                        .fixed_size(300.0, 60.0)
                        .padding(16.0),
                )
                .bg(Token::Surface)
                .child(text("OK \u{2705} go \u{1F680}").text_color(Token::OnSurface));
            on_black(panel, 312.0, 72.0)
        },
        InteractionMode::Default,
        [312, 72],
        (1.0, 1.0),
        Box::new(DefaultTextMeasure::new()),
    );
    assert_rendered_matches_golden(img, &golden_path!("emoji_text_line"), DELTA_PCT);
}
