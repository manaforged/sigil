#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]
#![allow(missing_docs)]
#![allow(clippy::unwrap_used, clippy::panic, clippy::indexing_slicing)]
#![allow(clippy::cast_possible_truncation, clippy::cast_precision_loss)]
#![allow(clippy::cast_sign_loss, clippy::doc_markdown)]
#![allow(clippy::print_stdout, clippy::too_many_lines)]

use sigil::prelude::*;

use sigil_kittest::{
    InteractionMode, assert_goldens_differ, assert_png_matches_golden, golden_path,
    render_widget_to_png, render_widget_to_png_focus_descendant,
};

const DELTA_PCT: f32 = 0.5;

fn check(stem: &str, img: Option<image::RgbaImage>) {
    let Some(img) = img else {
        println!("no GPU adapter — skipping {stem}");
        return;
    };
    assert_png_matches_golden(&img, &golden_path!(stem), DELTA_PCT);
}

fn on_panel(inner: impl IntoNode + 'static, w: f32, h: f32) -> NodeBuilder {
    container()
        .layout(
            LayoutStyle::flex_column()
                .fixed_size(w, h)
                .padding(6.0)
                .align_items(AlignItems::Stretch),
        )
        .child(inner)
}

#[test]
fn button_default() {
    let img = render_widget_to_png(
        |cx| cx.add(button("Save")),
        InteractionMode::Default,
        [160, 56],
        (24.0, 16.0),
    );
    check("button_default", img);
}

#[test]
fn button_solid_default() {
    let img = render_widget_to_png(
        |cx| cx.add(button("Save").primary()),
        InteractionMode::Default,
        [160, 56],
        (24.0, 16.0),
    );
    check("button_solid_default", img);
}

#[test]
fn button_hover() {
    let img = render_widget_to_png(
        |cx| cx.add(button("Save")),
        InteractionMode::Hover,
        [160, 56],
        (24.0, 16.0),
    );
    check("button_hover", img);
}

#[test]
fn button_pressed() {
    let img = render_widget_to_png(
        |cx| cx.add(button("Save")),
        InteractionMode::Pressed,
        [160, 56],
        (24.0, 16.0),
    );
    check("button_pressed", img);
}

#[test]
fn button_focused() {
    let img = render_widget_to_png(
        |cx| cx.add(button("Save")),
        InteractionMode::Focused,
        [160, 56],
        (24.0, 16.0),
    );
    check("button_focused", img);
}

#[test]
fn button_disabled() {
    let img = render_widget_to_png(
        |cx| cx.add(button("Save").enabled(false)),
        InteractionMode::Disabled,
        [160, 56],
        (24.0, 16.0),
    );
    check("button_disabled", img);
}

#[test]
fn checkbox_checked() {
    let img = render_widget_to_png(
        |cx| {
            let checked = signal(true);
            cx.add(checkbox(checked).class("bg-surface rounded-sm"))
        },
        InteractionMode::Default,
        [36, 36],
        (16.0, 16.0),
    );
    check("checkbox_checked", img);
}

#[test]
fn checkbox_default() {
    let img = render_widget_to_png(
        |cx| {
            let checked = signal(false);
            cx.add(checkbox(checked).class("bg-surface rounded-sm"))
        },
        InteractionMode::Default,
        [36, 36],
        (16.0, 16.0),
    );
    check("checkbox_default", img);
}

#[test]
fn checkbox_hover() {
    let img = render_widget_to_png(
        |cx| {
            let checked = signal(false);
            cx.add(checkbox(checked).class("bg-surface rounded-sm"))
        },
        InteractionMode::Hover,
        [36, 36],
        (16.0, 16.0),
    );
    check("checkbox_hover", img);
}

#[test]
fn checkbox_pressed() {
    let img = render_widget_to_png(
        |cx| {
            let checked = signal(false);
            cx.add(checkbox(checked).class("bg-surface rounded-sm"))
        },
        InteractionMode::Pressed,
        [36, 36],
        (16.0, 16.0),
    );
    check("checkbox_pressed", img);
}

#[test]
fn checkbox_focused() {
    let img = render_widget_to_png(
        |cx| {
            let checked = signal(false);
            cx.add(checkbox(checked).class("bg-surface rounded-sm"))
        },
        InteractionMode::Focused,
        [36, 36],
        (16.0, 16.0),
    );
    check("checkbox_focused", img);
}

#[test]
fn checkbox_disabled() {
    let img = render_widget_to_png(
        |cx| {
            let checked = signal(false);
            let dis = signal(true);
            cx.add(
                checkbox(checked)
                    .class("bg-surface rounded-sm")
                    .disabled(dis),
            )
        },
        InteractionMode::Disabled,
        [36, 36],
        (16.0, 16.0),
    );
    check("checkbox_disabled", img);
}

#[test]
fn slider_default() {
    let img = render_widget_to_png(
        |cx| {
            let v = signal(50.0_f32);
            cx.add(slider(v, 0.0..=100.0))
        },
        InteractionMode::Default,
        [192, 36],
        (90.0, 14.0),
    );
    check("slider_default", img);
}

#[test]
fn slider_hover() {
    let img = render_widget_to_png(
        |cx| {
            let v = signal(50.0_f32);
            cx.add(slider(v, 0.0..=100.0))
        },
        InteractionMode::Hover,
        [192, 36],
        (90.0, 14.0),
    );
    check("slider_hover", img);
}

#[test]
fn slider_pressed() {
    let img = render_widget_to_png(
        |cx| {
            let v = signal(50.0_f32);
            cx.add(slider(v, 0.0..=100.0))
        },
        InteractionMode::Pressed,
        [192, 36],
        (90.0, 14.0),
    );
    check("slider_pressed", img);
}

#[test]
fn slider_focused() {
    let img = render_widget_to_png(
        |cx| {
            let v = signal(50.0_f32);
            cx.add(slider(v, 0.0..=100.0))
        },
        InteractionMode::Focused,
        [192, 36],
        (90.0, 14.0),
    );
    check("slider_focused", img);
}

#[test]
fn slider_disabled() {
    let img = render_widget_to_png(
        |cx| {
            let v = signal(50.0_f32);
            let dis = signal(true);
            cx.add(slider(v, 0.0..=100.0).disabled(dis))
        },
        InteractionMode::Disabled,
        [192, 36],
        (90.0, 14.0),
    );
    check("slider_disabled", img);
}

#[test]
fn panel_default() {
    let img = render_widget_to_png(
        |cx| cx.add(panel().title("Settings")),
        InteractionMode::Default,
        [340, 92],
        (1.0, 1.0),
    );
    check("panel_default", img);
}

#[test]
fn progress_empty() {
    let img = render_widget_to_png(
        |cx| {
            let v = signal(0.0_f32);
            cx.add(progress(v, 100.0).height(24.0))
        },
        InteractionMode::Default,
        [244, 32],
        (1.0, 1.0),
    );
    check("progress_empty", img);
}

#[test]
fn progress_half() {
    let img = render_widget_to_png(
        |cx| {
            let v = signal(50.0_f32);
            cx.add(progress(v, 100.0).height(24.0))
        },
        InteractionMode::Default,
        [244, 32],
        (1.0, 1.0),
    );
    check("progress_half", img);
}

#[test]
fn progress_full() {
    let img = render_widget_to_png(
        |cx| {
            let v = signal(100.0_f32);
            cx.add(progress(v, 100.0).height(24.0))
        },
        InteractionMode::Default,
        [244, 32],
        (1.0, 1.0),
    );
    check("progress_full", img);
}

fn dropdown_options() -> Vec<String> {
    vec!["Easy".into(), "Normal".into(), "Hard".into()]
}

#[test]
fn dropdown_open() {
    let img = render_widget_to_png(
        |cx| {
            let sel = signal(0_usize);
            cx.add(dropdown(sel, dropdown_options()))
        },
        InteractionMode::Clicked,
        [212, 160],
        (90.0, 22.0),
    );
    check("dropdown_open", img);
}

#[test]
fn dropdown_default() {
    let img = render_widget_to_png(
        |cx| {
            let sel = signal(0_usize);
            cx.add(dropdown(sel, dropdown_options()))
        },
        InteractionMode::Default,
        [212, 52],
        (90.0, 22.0),
    );
    check("dropdown_default", img);
}

#[test]
fn dropdown_hover() {
    let img = render_widget_to_png(
        |cx| {
            let sel = signal(0_usize);
            cx.add(dropdown(sel, dropdown_options()))
        },
        InteractionMode::Hover,
        [212, 52],
        (90.0, 22.0),
    );
    check("dropdown_hover", img);
}

#[test]
fn dropdown_pressed() {
    let img = render_widget_to_png(
        |cx| {
            let sel = signal(0_usize);
            cx.add(dropdown(sel, dropdown_options()))
        },
        InteractionMode::Pressed,
        [212, 52],
        (90.0, 22.0),
    );
    check("dropdown_pressed", img);
}

#[test]
fn dropdown_focused() {
    let img = render_widget_to_png(
        |cx| {
            let sel = signal(0_usize);
            cx.add(dropdown(sel, dropdown_options()))
        },
        InteractionMode::Focused,
        [212, 52],
        (90.0, 22.0),
    );
    check("dropdown_focused", img);
}

#[test]
fn dropdown_disabled() {
    let img = render_widget_to_png(
        |cx| {
            let sel = signal(0_usize);
            let dis = signal(true);
            cx.add(dropdown(sel, dropdown_options()).disabled(dis))
        },
        InteractionMode::Disabled,
        [212, 52],
        (90.0, 22.0),
    );
    check("dropdown_disabled", img);
}

#[test]
fn scroll_default() {
    let img = render_widget_to_png(
        |cx| {
            let body = container()
                .class("bg-accent rounded-sm")
                .layout(LayoutStyle::block().fixed_size(312.0, 400.0));
            cx.add(scroll(body, 400.0).class("rounded-sm"))
        },
        InteractionMode::Default,
        [330, 252],
        (1.0, 1.0),
    );
    check("scroll_default", img);
}

#[test]
fn text_field_default() {
    let img = render_widget_to_png(
        |cx| {
            let v = signal(String::from("Hello"));
            cx.add(text_field(v))
        },
        InteractionMode::Default,
        [192, 44],
        (40.0, 22.0),
    );
    check("text_field_default", img);
}

#[test]
fn text_field_hover() {
    let img = render_widget_to_png(
        |cx| {
            let v = signal(String::from("Hello"));
            cx.add(text_field(v))
        },
        InteractionMode::Hover,
        [192, 44],
        (40.0, 22.0),
    );
    check("text_field_hover", img);
}

#[test]
fn text_field_pressed() {
    let img = render_widget_to_png(
        |cx| {
            let v = signal(String::from("Hello"));
            cx.add(text_field(v))
        },
        InteractionMode::Pressed,
        [192, 44],
        (40.0, 22.0),
    );
    check("text_field_pressed", img);
}

#[test]
fn text_field_focused() {
    let img = render_widget_to_png(
        |cx| {
            let v = signal(String::from("Hello"));
            cx.add(text_field(v))
        },
        InteractionMode::Focused,
        [192, 44],
        (40.0, 22.0),
    );
    check("text_field_focused", img);
}

#[test]
fn text_field_disabled() {
    let img = render_widget_to_png(
        |cx| {
            let v = signal(String::from("Hello"));
            let dis = signal(true);
            cx.add(text_field(v).disabled(dis))
        },
        InteractionMode::Disabled,
        [192, 44],
        (40.0, 22.0),
    );
    check("text_field_disabled", img);
}

fn tab_labels() -> Vec<String> {
    vec!["First".into(), "Second".into(), "Third".into()]
}

#[test]
fn tabs_default() {
    let img = render_widget_to_png(
        |cx| {
            let active = signal(0_usize);
            cx.add(tabs(active, tab_labels()))
        },
        InteractionMode::Default,
        [260, 56],
        (40.0, 17.0),
    );
    check("tabs_default", img);
}

#[test]
fn tabs_hover_tab2() {
    let img = render_widget_to_png(
        |cx| {
            let active = signal(0_usize);
            cx.add(tabs(active, tab_labels()))
        },
        InteractionMode::Hover,
        [260, 56],
        (120.0, 17.0),
    );
    check("tabs_hover_tab2", img);
}

#[test]
fn tabs_active_tab2() {
    let img = render_widget_to_png(
        |cx| {
            let active = signal(1_usize);
            cx.add(tabs(active, tab_labels()))
        },
        InteractionMode::Default,
        [260, 56],
        (120.0, 17.0),
    );
    check("tabs_active_tab2", img);
}

#[test]
fn tabs_focused() {
    let img = render_widget_to_png_focus_descendant(
        |cx| {
            let active = signal(0_usize);
            cx.add(tabs(active, tab_labels()))
        },
        InteractionMode::Focused,
        [260, 56],
        (40.0, 17.0),
    );
    check("tabs_focused", img);
}

#[test]
fn tabs_focused_differs_from_default() {
    assert_goldens_differ!("tabs_focused", "tabs_default");
}

#[test]
fn tabs_disabled() {
    let img = render_widget_to_png(
        |cx| {
            let active = signal(0_usize);
            cx.add(tabs(active, tab_labels()).disabled(&[1]))
        },
        InteractionMode::Default,
        [260, 56],
        (40.0, 17.0),
    );
    check("tabs_disabled", img);
}

#[test]
fn modal_open() {
    let img = render_widget_to_png(
        |cx| {
            let open = signal(true);
            let m = container()
                .layout(LayoutStyle::flex_column().fill())
                .class("bg-surface")
                .child(
                    modal(open)
                        .title("Confirm action")
                        .width(240.0)
                        .height(128.0)
                        .child(button("OK")),
                );
            cx.add(on_panel(m, 286.0, 164.0))
        },
        InteractionMode::Default,
        [286, 164],
        (143.0, 82.0),
    );
    check("modal_open", img);
}

#[test]
fn modal_dimmed_backdrop() {
    let img = render_widget_to_png(
        |cx| {
            let open = signal(true);
            let bg = container().class("bg-accent").layout(
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
            cx.add(on_panel(stack, 412.0, 292.0))
        },
        InteractionMode::Default,
        [412, 292],
        (200.0, 140.0),
    );
    check("modal_dimmed_backdrop", img);
}

#[test]
fn modal_focused() {
    let img = render_widget_to_png_focus_descendant(
        |cx| {
            let open = signal(true);
            let m = container()
                .layout(LayoutStyle::flex_column().fill())
                .class("bg-surface")
                .child(
                    modal(open)
                        .title("Confirm action")
                        .width(240.0)
                        .height(128.0)
                        .child(button("OK")),
                );
            cx.add(on_panel(m, 286.0, 164.0))
        },
        InteractionMode::Focused,
        [286, 164],
        (143.0, 82.0),
    );
    check("modal_focused", img);
}

#[test]
fn modal_focused_differs_from_open() {
    assert_goldens_differ!("modal_focused", "modal_open");
}

#[test]
fn modal_dismissing() {
    let img = render_widget_to_png(
        |cx| {
            let open = signal(true);
            let m = container()
                .layout(LayoutStyle::flex_column().fill())
                .class("bg-surface")
                .child(
                    modal(open)
                        .title("Confirm action")
                        .width(240.0)
                        .height(128.0)
                        .child(button("OK"))
                        .opacity(0.5),
                );
            cx.add(on_panel(m, 286.0, 164.0))
        },
        InteractionMode::Default,
        [286, 164],
        (143.0, 82.0),
    );
    check("modal_dismissing", img);
}

#[test]
fn menu_open() {
    let img = render_widget_to_png(
        |cx| {
            let open = signal(true);
            let m = container()
                .layout(LayoutStyle::flex_column().fill())
                .class("bg-surface")
                .child(
                    menu(open)
                        .width(200.0)
                        .item(MenuItem::new("New file"))
                        .item(MenuItem::new("Open\u{2026}"))
                        .item(MenuItem::new("Save")),
                );
            cx.add(on_panel(m, 140.0, 112.0))
        },
        InteractionMode::Default,
        [140, 112],
        (70.0, 56.0),
    );
    check("menu_open", img);
}

#[test]
fn menu_hover_item2() {
    let img = render_widget_to_png(
        |cx| {
            let open = signal(true);
            let m = container()
                .layout(LayoutStyle::flex_column().fill())
                .class("bg-surface")
                .child(
                    menu(open)
                        .width(200.0)
                        .item(MenuItem::new("New file"))
                        .item(MenuItem::new("Open\u{2026}"))
                        .item(MenuItem::new("Save")),
                );
            cx.add(on_panel(m, 140.0, 112.0))
        },
        InteractionMode::Hover,
        [140, 112],
        (70.0, 56.0),
    );
    check("menu_hover_item2", img);
}

#[test]
fn menu_disabled_item() {
    let img = render_widget_to_png(
        |cx| {
            let open = signal(true);
            let m = container()
                .layout(LayoutStyle::flex_column().fill())
                .class("bg-surface")
                .child(
                    menu(open)
                        .width(200.0)
                        .item(MenuItem::new("New"))
                        .item(MenuItem::new("Save"))
                        .item(MenuItem::new("Save As\u{2026}").disabled(true)),
                );
            cx.add(on_panel(m, 140.0, 112.0))
        },
        InteractionMode::Default,
        [140, 112],
        (70.0, 56.0),
    );
    check("menu_disabled_item", img);
}

#[test]
fn menu_with_separators() {
    let img = render_widget_to_png(
        |cx| {
            let open = signal(true);
            let m = container()
                .layout(LayoutStyle::flex_column().fill())
                .class("bg-surface")
                .child(
                    menu(open)
                        .width(200.0)
                        .item(MenuItem::new("Cut"))
                        .item(MenuItem::new("Copy"))
                        .separator()
                        .item(MenuItem::new("Paste")),
                );
            cx.add(on_panel(m, 140.0, 124.0))
        },
        InteractionMode::Default,
        [140, 124],
        (70.0, 62.0),
    );
    check("menu_with_separators", img);
}

#[test]
fn menu_submenu_open() {
    let img = render_widget_to_png(
        |cx| {
            let open = signal(true);
            let m = container()
                .layout(LayoutStyle::flex_column().fill())
                .class("bg-surface")
                .child(
                    menu(open)
                        .width(200.0)
                        .item(MenuItem::new("Export"))
                        .item(MenuItem::new("Recent").submenu(true))
                        .item(MenuItem::new("Quit")),
                );
            cx.add(on_panel(m, 140.0, 112.0))
        },
        InteractionMode::Default,
        [140, 112],
        (70.0, 56.0),
    );
    check("menu_submenu_open", img);
}

#[test]
fn tooltip_visible() {
    let img = render_widget_to_png(
        |cx| {
            cx.add(
                container()
                    .layout(LayoutStyle::flex_column().padding_y(30.0))
                    .child(tooltip("Save the file").child(button("Save"))),
            )
        },
        InteractionMode::Hover,
        [150, 74],
        (74.0, 50.0),
    );
    check("tooltip_visible", img);
}

#[test]
fn tooltip_offscreen_clamp() {
    let img = render_widget_to_png(
        |cx| cx.add(tooltip("Save the file").clamp_top().child(button("Save"))),
        InteractionMode::Hover,
        [150, 44],
        (74.0, 22.0),
    );
    check("tooltip_offscreen_clamp", img);
}

#[test]
fn tooltip_with_keyboard_shortcut() {
    let img = render_widget_to_png(
        |cx| {
            cx.add(
                container()
                    .layout(LayoutStyle::flex_column().padding_y(30.0))
                    .child(
                        tooltip("Save the file")
                            .shortcut(Some("Ctrl+S"))
                            .child(button("Save")),
                    ),
            )
        },
        InteractionMode::Hover,
        [180, 74],
        (90.0, 50.0),
    );
    check("tooltip_with_keyboard_shortcut", img);
}

#[derive(Clone)]
struct TableRow {
    name: &'static str,
    role: &'static str,
    city: &'static str,
}

fn table_golden_state() -> sigil::widgets::TableState<TableRow> {
    let rows = vec![
        TableRow {
            name: "Ada",
            role: "Engineer",
            city: "London",
        },
        TableRow {
            name: "Grace",
            role: "Admiral",
            city: "Arlington",
        },
        TableRow {
            name: "Edsger",
            role: "Scientist",
            city: "Nuenen",
        },
        TableRow {
            name: "Barbara",
            role: "Architect",
            city: "Cambridge",
        },
    ];
    let text_cell = |value: &'static str| container().child(text(value));
    sigil::widgets::TableState::new(
        rows,
        vec![
            TableColumn::new("name", "Name")
                .sort_by(|row: &TableRow| row.name)
                .width(ColWidth::Fixed(120.0))
                .cell(move |row: &TableRow| text_cell(row.name)),
            TableColumn::new("role", "Role")
                .sort_by(|row: &TableRow| row.role)
                .width(ColWidth::Fixed(96.0))
                .cell(move |row: &TableRow| text_cell(row.role)),
            TableColumn::new("city", "City")
                .width(ColWidth::Fixed(104.0))
                .cell(move |row: &TableRow| text_cell(row.city)),
        ],
    )
}

fn table_golden_view() -> NodeBuilder {
    container()
        .layout(LayoutStyle::block().fixed_size(320.0, 172.0))
        .class("bg-surface")
        .child(
            table(table_golden_state())
                .row_height(24.0)
                .header_height(28.0)
                .viewport_width(320.0)
                .viewport_height(144.0),
        )
}

#[test]
fn table_default() {
    let img = render_widget_to_png(
        |cx| cx.add(table_golden_view()),
        InteractionMode::Default,
        [320, 172],
        (1.0, 1.0),
    );
    check("table_default", img);
}

#[test]
fn table_selected() {
    let img = render_widget_to_png(
        |cx| {
            let state = table_golden_state();
            state.select_only(1);
            cx.add(
                container()
                    .layout(LayoutStyle::block().fixed_size(320.0, 172.0))
                    .class("bg-surface")
                    .child(
                        table(state)
                            .row_height(24.0)
                            .header_height(28.0)
                            .viewport_width(320.0)
                            .viewport_height(144.0),
                    ),
            )
        },
        InteractionMode::Default,
        [320, 172],
        (1.0, 1.0),
    );
    check("table_selected", img);
}

#[test]
fn table_sorted() {
    let img = render_widget_to_png(
        |cx| {
            let state = table_golden_state();
            state.toggle_sort("name");
            cx.add(
                container()
                    .layout(LayoutStyle::block().fixed_size(320.0, 172.0))
                    .class("bg-surface")
                    .child(
                        table(state)
                            .row_height(24.0)
                            .header_height(28.0)
                            .viewport_width(320.0)
                            .viewport_height(144.0),
                    ),
            )
        },
        InteractionMode::Default,
        [320, 172],
        (1.0, 1.0),
    );
    check("table_sorted", img);
}
