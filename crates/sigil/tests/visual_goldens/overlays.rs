use super::*;
use sigil::layout::Position;

#[test]
fn modal_open() {
    let img = render_widget_to_png_stub(
        || {
            let open = signal(true);
            let m = container()
                .layout(LayoutStyle::flex_column().fill())
                .bg(Token::Surface)
                .child(
                    modal(open)
                        .title("Confirm action")
                        .width(240.0)
                        .height(128.0)
                        .child(button("OK")),
                );
            on_panel(m, 286.0, 164.0)
        },
        InteractionMode::Default,
        [286, 164],
        (143.0, 82.0),
    );
    assert_rendered_matches_golden(img, &golden_path!("modal_open"), DELTA_PCT);
}

#[test]
fn modal_dimmed_backdrop() {
    let img = render_widget_to_png_stub(
        || {
            let open = signal(true);
            let bg = container().bg(Token::Accent).layout(
                LayoutStyle::block()
                    .position(Position::Absolute)
                    .inset_top(0.0)
                    .inset_left(0.0)
                    .fixed_size(400.0, 280.0),
            );
            let dialog = modal(open)
                .title("Confirm action")
                .width(240.0)
                .height(128.0)
                .child(button("OK"));
            let stack = container()
                .layout(LayoutStyle::block().fixed_size(400.0, 280.0))
                .child(bg)
                .child(dialog);
            on_panel(stack, 412.0, 292.0)
        },
        InteractionMode::Default,
        [412, 292],
        (200.0, 140.0),
    );
    assert_rendered_matches_golden(img, &golden_path!("modal_dimmed_backdrop"), DELTA_PCT);
}

#[test]
fn modal_focused() {
    let img = render_widget_to_png_focus_descendant(
        || {
            let open = signal(true);
            let m = container()
                .layout(LayoutStyle::flex_column().fill())
                .bg(Token::Surface)
                .child(
                    modal(open)
                        .title("Confirm action")
                        .width(240.0)
                        .height(128.0)
                        .child(button("OK")),
                );
            on_panel(m, 286.0, 164.0)
        },
        InteractionMode::Focused,
        [286, 164],
        (143.0, 82.0),
        sigil_kittest::stub_text_measure(),
    );
    assert_rendered_matches_golden(img, &golden_path!("modal_focused"), DELTA_PCT);
}

#[test]
fn modal_focused_differs_from_open() {
    assert_goldens_differ!("modal_focused", "modal_open");
}

#[test]
fn modal_dismissing() {
    let img = render_widget_to_png_stub(
        || {
            let open = signal(true);
            let m = container()
                .layout(LayoutStyle::flex_column().fill())
                .bg(Token::Surface)
                .child(
                    modal(open)
                        .title("Confirm action")
                        .width(240.0)
                        .height(128.0)
                        .child(button("OK"))
                        .opacity(0.5),
                );
            on_panel(m, 286.0, 164.0)
        },
        InteractionMode::Default,
        [286, 164],
        (143.0, 82.0),
    );
    assert_rendered_matches_golden(img, &golden_path!("modal_dismissing"), DELTA_PCT);
}

#[test]
fn menu_open() {
    let img = render_widget_to_png_stub(
        || {
            let open = signal(true);
            let m = container()
                .layout(LayoutStyle::flex_column().fill())
                .bg(Token::Surface)
                .child(
                    menu(open)
                        .width(200.0)
                        .item(MenuItem::new("New file"))
                        .item(MenuItem::new("Open\u{2026}"))
                        .item(MenuItem::new("Save")),
                );
            on_panel(m, 140.0, 112.0)
        },
        InteractionMode::Default,
        [140, 112],
        (70.0, 56.0),
    );
    assert_rendered_matches_golden(img, &golden_path!("menu_open"), DELTA_PCT);
}

#[test]
fn menu_hover_item2() {
    let img = render_widget_to_png_stub(
        || {
            let open = signal(true);
            let m = container()
                .layout(LayoutStyle::flex_column().fill())
                .bg(Token::Surface)
                .child(
                    menu(open)
                        .width(200.0)
                        .item(MenuItem::new("New file"))
                        .item(MenuItem::new("Open\u{2026}"))
                        .item(MenuItem::new("Save")),
                );
            on_panel(m, 140.0, 112.0)
        },
        InteractionMode::Hover,
        [140, 112],
        (70.0, 56.0),
    );
    assert_rendered_matches_golden(img, &golden_path!("menu_hover_item2"), DELTA_PCT);
}

#[test]
fn menu_disabled_item() {
    let img = render_widget_to_png_stub(
        || {
            let open = signal(true);
            let m = container()
                .layout(LayoutStyle::flex_column().fill())
                .bg(Token::Surface)
                .child(
                    menu(open)
                        .width(200.0)
                        .item(MenuItem::new("New"))
                        .item(MenuItem::new("Save"))
                        .item(MenuItem::new("Save As\u{2026}").disabled(true)),
                );
            on_panel(m, 140.0, 112.0)
        },
        InteractionMode::Default,
        [140, 112],
        (70.0, 56.0),
    );
    assert_rendered_matches_golden(img, &golden_path!("menu_disabled_item"), DELTA_PCT);
}

#[test]
fn menu_with_separators() {
    let img = render_widget_to_png_stub(
        || {
            let open = signal(true);
            let m = container()
                .layout(LayoutStyle::flex_column().fill())
                .bg(Token::Surface)
                .child(
                    menu(open)
                        .width(200.0)
                        .item(MenuItem::new("Cut"))
                        .item(MenuItem::new("Copy"))
                        .separator()
                        .item(MenuItem::new("Paste")),
                );
            on_panel(m, 140.0, 124.0)
        },
        InteractionMode::Default,
        [140, 124],
        (70.0, 62.0),
    );
    assert_rendered_matches_golden(img, &golden_path!("menu_with_separators"), DELTA_PCT);
}

#[test]
fn menu_submenu_open() {
    let img = render_widget_to_png_stub(
        || {
            let open = signal(true);
            let m = container()
                .layout(LayoutStyle::flex_column().fill())
                .bg(Token::Surface)
                .child(
                    menu(open)
                        .width(200.0)
                        .item(MenuItem::new("Export"))
                        .item(MenuItem::new("Recent").submenu(true))
                        .item(MenuItem::new("Quit")),
                );
            on_panel(m, 140.0, 112.0)
        },
        InteractionMode::Default,
        [140, 112],
        (70.0, 56.0),
    );
    assert_rendered_matches_golden(img, &golden_path!("menu_submenu_open"), DELTA_PCT);
}

#[test]
fn tooltip_visible() {
    let img = render_widget_to_png_stub(
        || {
            container()
                .layout(LayoutStyle::flex_column().padding_y(30.0))
                .child(tooltip("Save the file").child(button("Save")))
        },
        InteractionMode::Hover,
        [150, 74],
        (74.0, 50.0),
    );
    assert_rendered_matches_golden(img, &golden_path!("tooltip_visible"), DELTA_PCT);
}

#[test]
fn tooltip_offscreen_clamp() {
    let img = render_widget_to_png_stub(
        || tooltip("Save the file").clamp_top().child(button("Save")),
        InteractionMode::Hover,
        [150, 44],
        (74.0, 22.0),
    );
    assert_rendered_matches_golden(img, &golden_path!("tooltip_offscreen_clamp"), DELTA_PCT);
}

#[test]
fn tooltip_with_keyboard_shortcut() {
    let img = render_widget_to_png_stub(
        || {
            container()
                .layout(LayoutStyle::flex_column().padding_y(30.0))
                .child(
                    tooltip("Save the file")
                        .shortcut(Some("Ctrl+S"))
                        .child(button("Save")),
                )
        },
        InteractionMode::Hover,
        [180, 74],
        (90.0, 50.0),
    );
    assert_rendered_matches_golden(
        img,
        &golden_path!("tooltip_with_keyboard_shortcut"),
        DELTA_PCT,
    );
}
