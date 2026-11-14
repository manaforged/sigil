use super::*;

fn ten_row_list() -> Vec<String> {
    (0..10).map(|i| format!("item {i}")).collect()
}

fn row_label(label: &str) -> NodeBuilder {
    container()
        .layout(LayoutStyle::flex_row())
        .background(Token::Surface)
        .text_color(Token::OnSurface)
        .child(text(label.to_owned()))
}

#[test]
fn virtual_list_default() {
    let img = Harness::render(|| {
        let w = virtual_list(ten_row_list(), |_, s: &String| row_label(s))
            .row_height(20.0)
            .viewport_height(200.0)
            .viewport_width(220.0);
        on_black(w, 232.0, 212.0)
    })
    .stub()
    .mode(InteractionMode::Default)
    .size([232, 212])
    .hit((120.0, 106.0))
    .image();
    Golden::new("virtual_list_default").assert(img);
}

#[test]
fn virtual_list_focused() {
    let img = Harness::render(|| {
        let w = virtual_list(ten_row_list(), |_, s: &String| row_label(s))
            .row_height(20.0)
            .viewport_height(200.0)
            .viewport_width(220.0);
        on_black(w, 232.0, 212.0)
    })
    .stub()
    .mode(InteractionMode::Focused)
    .size([232, 212])
    .hit((120.0, 106.0))
    .image();
    Golden::new("virtual_list_focused").assert(img);
}

#[test]
fn virtual_list_items_bind() {
    let img = Harness::render(|| {
        let items: Signal<Vec<String>> = signal(ten_row_list());
        let w = virtual_list(items, |_, s: &String| row_label(s))
            .row_height(20.0)
            .viewport_height(200.0)
            .viewport_width(220.0);
        on_black(w, 232.0, 212.0)
    })
    .stub()
    .mode(InteractionMode::Default)
    .size([232, 212])
    .hit((120.0, 106.0))
    .image();
    Golden::new("virtual_list_items_bind").assert(img);
}

fn splitter_pane(bg: Token) -> NodeBuilder {
    container()
        .background(bg)
        .layout(LayoutStyle::flex_column().fill())
}

#[test]
fn splitter_horizontal_default() {
    let img = Harness::render(|| {
        let w = splitter(Orientation::Horizontal)
            .child(splitter_pane(Token::Accent), 120.0)
            .child(splitter_pane(Token::SurfaceHover), 120.0)
            .layout(LayoutStyle::flex_row().fixed_size(246.0, 80.0));
        on_black(w, 258.0, 92.0)
    })
    .stub()
    .mode(InteractionMode::Default)
    .size([258, 92])
    .hit((168.0, 46.0))
    .image();
    Golden::new("splitter_horizontal_default").assert(img);
}

#[test]
fn splitter_vertical_default() {
    let img = Harness::render(|| {
        let w = splitter(Orientation::Vertical)
            .child(splitter_pane(Token::Accent), 60.0)
            .child(splitter_pane(Token::SurfaceHover), 60.0)
            .layout(LayoutStyle::flex_column().fixed_size(160.0, 126.0));
        on_black(w, 172.0, 138.0)
    })
    .stub()
    .mode(InteractionMode::Default)
    .size([172, 138])
    .hit((86.0, 70.0))
    .image();
    Golden::new("splitter_vertical_default").assert(img);
}

#[test]
fn splitter_horizontal_with_min_max() {
    let img = Harness::render(|| {
        let w = splitter(Orientation::Horizontal)
            .child(splitter_pane(Token::Accent), 120.0)
            .child(splitter_pane(Token::SurfaceHover), 120.0)
            .min_size(0, 60.0)
            .max_size(0, 200.0)
            .min_size(1, 60.0)
            .max_size(1, 200.0)
            .layout(LayoutStyle::flex_row().fixed_size(246.0, 80.0));
        on_black(w, 258.0, 92.0)
    })
    .stub()
    .mode(InteractionMode::Default)
    .size([258, 92])
    .hit((168.0, 46.0))
    .image();
    Golden::new("splitter_horizontal_with_min_max").assert(img);
}
