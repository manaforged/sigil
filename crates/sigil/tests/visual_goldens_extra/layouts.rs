use super::*;

#[test]
fn hint_overlay_default() {
    let img = render_widget_to_png_with(
        || {
            let overlay = sigil::widgets::hint_overlay(&[
                ("Ctrl+S", "Save"),
                ("Ctrl+Z", "Undo"),
                ("?", "Toggle this overlay"),
            ]);
            let surface = container()
                .layout(LayoutStyle::flex_column().fixed_size(220.0, 84.0))
                .class("bg-surface")
                .child(overlay);
            on_black(surface, 232.0, 96.0)
        },
        InteractionMode::Default,
        [232, 96],
        (1.0, 1.0),
        Box::new(DefaultTextMeasure::new()),
    );
    assert_rendered_matches_golden(img, &golden_path!("hint_overlay_default"), DELTA_PCT);
}

#[test]
fn pattern_app_shell() {
    let img = render_widget_to_png_stub(
        || {
            container()
                .layout(LayoutStyle::flex_column().fixed_size(360.0, 200.0))
                .child(sigil::widgets::app_shell(
                    container()
                        .class("bg-accent")
                        .layout(LayoutStyle::flex_row().fill().padding(8.0))
                        .child(text("Top bar")),
                    container()
                        .class("bg-surface-hover")
                        .layout(LayoutStyle::flex_column().fill().padding(8.0))
                        .child(text("Work")),
                    status_bar([chip("READY", Tone::Success)]),
                    None::<NodeBuilder>,
                ))
        },
        InteractionMode::Default,
        [376, 216],
        (1.0, 1.0),
    );
    assert_rendered_matches_golden(img, &golden_path!("pattern_app_shell"), DELTA_PCT);
}

#[test]
fn pattern_field() {
    let img = render_widget_to_png_stub(
        || {
            let buffer = signal(String::from("you@host"));
            let field = container()
                .layout(
                    LayoutStyle::flex_column()
                        .fixed_size(220.0, 70.0)
                        .padding(8.0),
                )
                .class("bg-surface")
                .child(sigil::widgets::field("Username", text_field(buffer)));
            on_black(field, 232.0, 82.0)
        },
        InteractionMode::Default,
        [232, 82],
        (116.0, 41.0),
    );
    assert_rendered_matches_golden(img, &golden_path!("pattern_field"), DELTA_PCT);
}

#[test]
fn pattern_kv_grid() {
    let img = render_widget_to_png_with(
        || {
            let grid = container()
                .layout(
                    LayoutStyle::flex_column()
                        .fixed_size(280.0, 96.0)
                        .padding(8.0),
                )
                .class("bg-surface")
                .child(sigil::widgets::kv_grid(&[
                    ("Content-Type", "application/json"),
                    ("Status", "200 OK"),
                    ("Length", "1024"),
                ]));
            on_black(grid, 292.0, 108.0)
        },
        InteractionMode::Default,
        [292, 108],
        (146.0, 54.0),
        Box::new(DefaultTextMeasure::new()),
    );
    assert_rendered_matches_golden(img, &golden_path!("pattern_kv_grid"), DELTA_PCT);
}
