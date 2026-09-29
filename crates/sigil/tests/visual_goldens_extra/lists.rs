use super::*;

fn ten_row_list() -> Vec<String> {
    (0..10).map(|i| format!("item {i}")).collect()
}

fn row_label(label: &str) -> NodeBuilder {
    container()
        .layout(LayoutStyle::flex_row())
        .class("bg-surface text-on-surface")
        .child(text(label.to_owned()))
}

#[test]
fn virtual_list_default() {
    let img = render_widget_to_png_stub(
        || {
            let w = virtual_list(ten_row_list(), |_, s: &String| row_label(s))
                .row_height(20.0)
                .viewport_height(200.0)
                .viewport_width(220.0);
            on_black(w, 232.0, 212.0)
        },
        InteractionMode::Default,
        [232, 212],
        (120.0, 106.0),
    );
    assert_rendered_matches_golden(img, &golden_path!("virtual_list_default"), DELTA_PCT);
}

#[test]
fn virtual_list_focused() {
    let img = render_widget_to_png_stub(
        || {
            let w = virtual_list(ten_row_list(), |_, s: &String| row_label(s))
                .row_height(20.0)
                .viewport_height(200.0)
                .viewport_width(220.0);
            on_black(w, 232.0, 212.0)
        },
        InteractionMode::Focused,
        [232, 212],
        (120.0, 106.0),
    );
    assert_rendered_matches_golden(img, &golden_path!("virtual_list_focused"), DELTA_PCT);
}

#[test]
fn virtual_list_items_bind() {
    let img = render_widget_to_png_stub(
        || {
            let items: Signal<Vec<String>> = signal(ten_row_list());
            let w = virtual_list(items, |_, s: &String| row_label(s))
                .row_height(20.0)
                .viewport_height(200.0)
                .viewport_width(220.0);
            on_black(w, 232.0, 212.0)
        },
        InteractionMode::Default,
        [232, 212],
        (120.0, 106.0),
    );
    assert_rendered_matches_golden(img, &golden_path!("virtual_list_items_bind"), DELTA_PCT);
}

fn splitter_pane(class: &'static str) -> NodeBuilder {
    container()
        .class(class)
        .layout(LayoutStyle::flex_column().fill())
}

#[test]
fn splitter_horizontal_default() {
    let img = render_widget_to_png_stub(
        || {
            let w = splitter(SplitterOrientation::Horizontal)
                .child(splitter_pane("bg-accent"), 120.0)
                .child(splitter_pane("bg-surface-hover"), 120.0)
                .layout(LayoutStyle::flex_row().fixed_size(246.0, 80.0));
            on_black(w, 258.0, 92.0)
        },
        InteractionMode::Default,
        [258, 92],
        (168.0, 46.0),
    );
    assert_rendered_matches_golden(img, &golden_path!("splitter_horizontal_default"), DELTA_PCT);
}

#[test]
fn splitter_vertical_default() {
    let img = render_widget_to_png_stub(
        || {
            let w = splitter(SplitterOrientation::Vertical)
                .child(splitter_pane("bg-accent"), 60.0)
                .child(splitter_pane("bg-surface-hover"), 60.0)
                .layout(LayoutStyle::flex_column().fixed_size(160.0, 126.0));
            on_black(w, 172.0, 138.0)
        },
        InteractionMode::Default,
        [172, 138],
        (86.0, 70.0),
    );
    assert_rendered_matches_golden(img, &golden_path!("splitter_vertical_default"), DELTA_PCT);
}

#[test]
fn splitter_horizontal_with_min_max() {
    let img = render_widget_to_png_stub(
        || {
            let w = splitter(SplitterOrientation::Horizontal)
                .child(splitter_pane("bg-accent"), 120.0)
                .child(splitter_pane("bg-surface-hover"), 120.0)
                .min_size(0, 60.0)
                .max_size(0, 200.0)
                .min_size(1, 60.0)
                .max_size(1, 200.0)
                .layout(LayoutStyle::flex_row().fixed_size(246.0, 80.0));
            on_black(w, 258.0, 92.0)
        },
        InteractionMode::Default,
        [258, 92],
        (168.0, 46.0),
    );
    assert_rendered_matches_golden(
        img,
        &golden_path!("splitter_horizontal_with_min_max"),
        DELTA_PCT,
    );
}
