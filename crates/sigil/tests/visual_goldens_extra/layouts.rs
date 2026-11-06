use super::*;
use sigil::layout::Space;

#[test]
fn hint_overlay_default() {
    let img = Harness::render(|| {
        let overlay = sigil::widgets::hint_overlay(&[
            ("Ctrl+S", "Save"),
            ("Ctrl+Z", "Undo"),
            ("?", "Toggle this overlay"),
        ]);
        let surface = container()
            .layout(LayoutStyle::flex_column().fixed_size(220.0, 84.0))
            .bg(Token::Surface)
            .child(overlay);
        on_black(surface, 232.0, 96.0)
    })
    .measure(|| Box::new(DefaultTextMeasure::new()))
    .mode(InteractionMode::Default)
    .size([232, 96])
    .hit((1.0, 1.0))
    .image();
    Golden::new("hint_overlay_default")
        .delta(DELTA_PCT)
        .assert(img);
}

#[test]
fn pattern_app_shell() {
    let img = Harness::render(|| {
        container()
            .layout(LayoutStyle::flex_column().fixed_size(360.0, 200.0))
            .child(sigil::widgets::app_shell(
                container()
                    .bg(Token::Accent)
                    .layout(LayoutStyle::flex_row().fill().padding(Space::Sm))
                    .child(text("Top bar")),
                container()
                    .bg(Token::SurfaceHover)
                    .layout(LayoutStyle::flex_column().fill().padding(Space::Sm))
                    .child(text("Work")),
                status_bar([chip("READY", Tone::Success)]),
                None::<NodeBuilder>,
            ))
    })
    .stub()
    .mode(InteractionMode::Default)
    .size([376, 216])
    .hit((1.0, 1.0))
    .image();
    Golden::new("pattern_app_shell")
        .delta(DELTA_PCT)
        .assert(img);
}

#[test]
fn pattern_field() {
    let img = Harness::render(|| {
        let buffer = signal(String::from("you@host"));
        let field = container()
            .layout(
                LayoutStyle::flex_column()
                    .fixed_size(220.0, 70.0)
                    .padding(Space::Sm),
            )
            .bg(Token::Surface)
            .child(sigil::widgets::field("Username", text_field(buffer)));
        on_black(field, 232.0, 82.0)
    })
    .stub()
    .mode(InteractionMode::Default)
    .size([232, 82])
    .hit((116.0, 41.0))
    .image();
    Golden::new("pattern_field").delta(DELTA_PCT).assert(img);
}

#[test]
fn pattern_kv_grid() {
    let img = Harness::render(|| {
        let grid = container()
            .layout(
                LayoutStyle::flex_column()
                    .fixed_size(280.0, 96.0)
                    .padding(Space::Sm),
            )
            .bg(Token::Surface)
            .child(sigil::widgets::kv_grid([
                ("Content-Type", "application/json"),
                ("Status", "200 OK"),
                ("Length", "1024"),
            ]));
        on_black(grid, 292.0, 108.0)
    })
    .measure(|| Box::new(DefaultTextMeasure::new()))
    .mode(InteractionMode::Default)
    .size([292, 108])
    .hit((146.0, 54.0))
    .image();
    Golden::new("pattern_kv_grid").delta(DELTA_PCT).assert(img);
}
