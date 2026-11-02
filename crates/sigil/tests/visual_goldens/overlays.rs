use super::*;
use sigil::layout::Position;

#[test]
fn modal_open() {
    let img = Harness::render(|| {
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
    })
    .stub()
    .mode(InteractionMode::Default)
    .size([286, 164])
    .hit((143.0, 82.0))
    .image();
    Golden::new("modal_open").delta(DELTA_PCT).assert(img);
}

#[test]
fn modal_dimmed_backdrop() {
    let img = Harness::render(|| {
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
    })
    .stub()
    .mode(InteractionMode::Default)
    .size([412, 292])
    .hit((200.0, 140.0))
    .image();
    Golden::new("modal_dimmed_backdrop")
        .delta(DELTA_PCT)
        .assert(img);
}

#[test]
fn modal_focused() {
    let img = Harness::render(|| {
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
    })
    .stub()
    .focus_first()
    .mode(InteractionMode::Focused)
    .size([286, 164])
    .hit((143.0, 82.0))
    .image();
    Golden::new("modal_focused").delta(DELTA_PCT).assert(img);
}

#[test]
fn modal_focused_differs_from_open() {
    Golden::new("modal_focused").assert_differs(&Golden::new("modal_open"));
}

#[test]
fn modal_dismissing() {
    let img = Harness::render(|| {
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
    })
    .stub()
    .mode(InteractionMode::Default)
    .size([286, 164])
    .hit((143.0, 82.0))
    .image();
    Golden::new("modal_dismissing").delta(DELTA_PCT).assert(img);
}

#[test]
fn menu_open() {
    let img = Harness::render(|| {
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
    })
    .stub()
    .mode(InteractionMode::Default)
    .size([140, 112])
    .hit((70.0, 56.0))
    .image();
    Golden::new("menu_open").delta(DELTA_PCT).assert(img);
}

#[test]
fn menu_hover_item2() {
    let img = Harness::render(|| {
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
    })
    .stub()
    .mode(InteractionMode::Hover)
    .size([140, 112])
    .hit((70.0, 56.0))
    .image();
    Golden::new("menu_hover_item2").delta(DELTA_PCT).assert(img);
}

#[test]
fn menu_disabled_item() {
    let img = Harness::render(|| {
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
    })
    .stub()
    .mode(InteractionMode::Default)
    .size([140, 112])
    .hit((70.0, 56.0))
    .image();
    Golden::new("menu_disabled_item")
        .delta(DELTA_PCT)
        .assert(img);
}

#[test]
fn menu_with_separators() {
    let img = Harness::render(|| {
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
    })
    .stub()
    .mode(InteractionMode::Default)
    .size([140, 124])
    .hit((70.0, 62.0))
    .image();
    Golden::new("menu_with_separators")
        .delta(DELTA_PCT)
        .assert(img);
}

#[test]
fn menu_submenu_open() {
    let img = Harness::render(|| {
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
    })
    .stub()
    .mode(InteractionMode::Default)
    .size([140, 112])
    .hit((70.0, 56.0))
    .image();
    Golden::new("menu_submenu_open")
        .delta(DELTA_PCT)
        .assert(img);
}

#[test]
fn tooltip_visible() {
    let img = Harness::render(|| {
        container()
            .layout(LayoutStyle::flex_column().padding_y(30.0))
            .child(tooltip("Save the file").child(button("Save")))
    })
    .stub()
    .mode(InteractionMode::Hover)
    .size([150, 74])
    .hit((74.0, 50.0))
    .image();
    Golden::new("tooltip_visible").delta(DELTA_PCT).assert(img);
}

#[test]
fn tooltip_offscreen_clamp() {
    let img = Harness::render(|| tooltip("Save the file").clamp_top().child(button("Save")))
        .stub()
        .mode(InteractionMode::Hover)
        .size([150, 44])
        .hit((74.0, 22.0))
        .image();
    Golden::new("tooltip_offscreen_clamp")
        .delta(DELTA_PCT)
        .assert(img);
}

#[test]
fn tooltip_with_keyboard_shortcut() {
    let img = Harness::render(|| {
        container()
            .layout(LayoutStyle::flex_column().padding_y(30.0))
            .child(
                tooltip("Save the file")
                    .hint(Some("Ctrl+S"))
                    .child(button("Save")),
            )
    })
    .stub()
    .mode(InteractionMode::Hover)
    .size([180, 74])
    .hit((90.0, 50.0))
    .image();
    Golden::new("tooltip_with_keyboard_shortcut")
        .delta(DELTA_PCT)
        .assert(img);
}
