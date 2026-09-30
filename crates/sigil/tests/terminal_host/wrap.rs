use super::*;

#[test]
fn wrapped_label_fills_the_container_width() {
    use sigil::layout::LayoutStyle;
    let body = "x".repeat(80);
    let out = present(
        move || {
            container().layout(LayoutStyle::block().fill_width()).child(
                text(body.clone())
                    .wrap()
                    .layout(LayoutStyle::block().fill_width()),
            )
        },
        60,
        10,
        &[],
    );
    let count = out.lines().filter(|l| l.contains('x')).count();
    assert!(
        count >= 2,
        "80 x's in a 52-column container must wrap to two lines, got:\n{out}"
    );
}

#[test]
fn wrapped_label_inside_modal_band_wraps() {
    use sigil::layout::LayoutStyle;
    use sigil::reactive::signal;
    use sigil::widgets::modal;
    sigil::reactive::Runtime::scope("tui-modal-wrap", || {
        let body = "x".repeat(80);
        let open = signal(true);
        let out = present(
            move || {
                container().child(
                    modal(open).title("models").trap(true).child(
                        container()
                            .layout(LayoutStyle::flex_column().fill_width())
                            .child(
                                container()
                                    .layout(
                                        LayoutStyle::flex_row()
                                            .fill_width()
                                            .min_size(0.0, 0.0)
                                            .padding_x(8.0),
                                    )
                                    .child(
                                        text(body.clone())
                                            .wrap()
                                            .font_size(12)
                                            .layout(LayoutStyle::block().fill_width()),
                                    ),
                            ),
                    ),
                )
            },
            60,
            10,
            &[],
        );
        let count = out.lines().filter(|l| l.contains('x')).count();
        assert!(count >= 2, "modal band must wrap the label, got:\n{out}");
    });
}

#[test]
fn wrapped_label_inside_flex_row_band_wraps() {
    use sigil::layout::LayoutStyle;
    let body = "x".repeat(80);
    let out = present(
        move || {
            container().layout(LayoutStyle::block().fill_width()).child(
                container()
                    .layout(
                        LayoutStyle::flex_row()
                            .fill_width()
                            .min_size(0.0, 0.0)
                            .padding_x(8.0),
                    )
                    .child(
                        text(body.clone())
                            .wrap()
                            .layout(LayoutStyle::block().fill_width()),
                    ),
            )
        },
        60,
        10,
        &[],
    );
    let count = out.lines().filter(|l| l.contains('x')).count();
    assert!(count >= 2, "flex_row band must wrap the label, got:\n{out}");
}

#[test]
fn grow_fill_does_not_clip_row_labels() {
    use sigil::kernel::{container, node};
    use sigil::prelude::{AlignItems, LayoutStyle, divider_horizontal};
    let grid = present(
        || {
            container()
                .layout(
                    LayoutStyle::flex_column()
                        .fill_width()
                        .min_size(0.0, 0.0)
                        .padding_x(24.0)
                        .gap(0.0),
                )
                .child(
                    container()
                        .layout(
                            LayoutStyle::flex_row()
                                .fill_width()
                                .shrink(0.0)
                                .align_items(AlignItems::Center)
                                .gap(8.0),
                        )
                        .children_reactive(|| {
                            vec![
                                node(container().child(text("L_L_L_L").nowrap())),
                                node(
                                    container()
                                        .layout(
                                            LayoutStyle::flex_row().grow(1.0).min_size(0.0, 0.0),
                                        )
                                        .child(divider_horizontal()),
                                ),
                                node(container().child(text("RRRRRRRRRRRRRRRR").nowrap())),
                            ]
                        }),
                )
        },
        40,
        8,
        &[],
    );
    assert!(grid.contains("L_L_L_L"), "left label clipped in:\n{grid}");
    assert!(
        grid.contains("RRRRRRRRRRRRRRRR"),
        "right label clipped in:\n{grid}"
    );
}
