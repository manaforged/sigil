use sigil::prelude::*;

use super::spec::{Build, State, node};

pub(crate) const BUILDERS: &[(&str, Build)] = &[
    ("panel", panel_row),
    ("card", card_row),
    ("surface", surface_row),
    ("sidebar", sidebar_row),
    ("status_bar", status_row),
    ("app_shell", shell_row),
    ("menu_bar", menu_bar_row),
    ("tabs", tabs_row),
    ("tabs_line", tabs_line_row),
    ("splitter", splitter_row),
    ("scroll", scroll_fixed_row),
    ("scroll_auto", scroll_row),
    ("terminal", terminal_row),
];

fn panel_row(_state: State) -> NodeBuilder {
    node(panel().title("Panel").child(text("Panel body").nowrap()))
}

fn card_row(_state: State) -> NodeBuilder {
    node(card(kv_grid(&[("Host", "wgpu"), ("Backend", "metal")])))
}

fn surface_row(_state: State) -> NodeBuilder {
    node(surface(7).layout(LayoutStyle::flex_column().fixed_size(160.0, 48.0)))
}

fn sidebar_row(_state: State) -> NodeBuilder {
    container()
        .layout(LayoutStyle::flex_row().fill_width().height(144.0))
        .child(sidebar(
            container()
                .layout(LayoutStyle::flex_column().padding(8.0).gap(2.0))
                .child(section_label("Widgets"))
                .child(list_row(true, "Controls", ""))
                .child(list_row(false, "Inputs", "")),
        ))
}

fn status_row(_state: State) -> NodeBuilder {
    node(status_bar([
        text("main").nowrap(),
        text("Ln 4, Col 12").nowrap(),
        text("UTF-8").nowrap(),
    ]))
}

fn shell_row(_state: State) -> NodeBuilder {
    container()
        .layout(LayoutStyle::flex_column().fill_width().height(200.0))
        .child(app_shell(
            text("Top bar").nowrap(),
            text("Work area").nowrap(),
            text("Status").nowrap(),
            None::<NodeBuilder>,
        ))
}

fn menu_bar_row(_state: State) -> NodeBuilder {
    node(menu_bar().menu(
        "File",
        [
            MenuItem::new("New").select(|| {}),
            MenuItem::new("Open").shortcut("Ctrl+O").select(|| {}),
            MenuItem::new("Quit").shortcut("Ctrl+Q").select(|| {}),
        ],
    ))
}

fn tabs_row(state: State) -> NodeBuilder {
    let active = signal(usize::from(state == State::Selected));
    let disabled: &[usize] = if state == State::Disabled { &[1] } else { &[] };
    node(
        tabs(
            active,
            vec!["First".into(), "Second".into(), "Third".into()],
        )
        .disabled(disabled)
        .content_for(0, text("First tab pane").nowrap())
        .content_for(1, text("Second tab pane").nowrap())
        .content_for(2, text("Third tab pane").nowrap()),
    )
}

fn tabs_line_row(state: State) -> NodeBuilder {
    let active = signal(usize::from(state == State::Selected));
    let disabled: &[usize] = if state == State::Disabled { &[1] } else { &[] };
    node(
        tabs(
            active,
            vec!["Overview".into(), "Activity".into(), "Settings".into()],
        )
        .line()
        .disabled(disabled)
        .content_for(0, text("Overview pane").nowrap())
        .content_for(1, text("Activity pane").nowrap())
        .content_for(2, text("Settings pane").nowrap()),
    )
}

fn splitter_row(_state: State) -> NodeBuilder {
    node(
        splitter(SplitterOrientation::Horizontal)
            .child(card(text("Navigation").nowrap()), 180.0)
            .child(card(text("Content").nowrap()), 260.0)
            .height(140.0),
    )
}

fn scroll_fixed_row(_state: State) -> NodeBuilder {
    container()
        .layout(LayoutStyle::flex_column().fill_width().height(120.0))
        .child(scroll(scroll_lines(), 280.0).fill())
}

fn scroll_lines() -> NodeBuilder {
    let mut column = container().layout(LayoutStyle::flex_column().fill_width().gap(4.0));
    for index in 1..=12 {
        column = column.child(text(format!("Scrolled line {index}")).nowrap());
    }
    column
}

fn scroll_row(_state: State) -> NodeBuilder {
    container()
        .layout(LayoutStyle::flex_column().fill_width().height(120.0))
        .child(scroll_auto(scroll_lines()).fill())
}

fn terminal_row(_state: State) -> NodeBuilder {
    let lines = vec![
        String::from("$ cargo build"),
        String::from("   Compiling sigil-ui v0.1.0"),
        String::from("    Finished dev profile"),
    ];
    container()
        .layout(LayoutStyle::flex_column().fill_width().height(120.0))
        .child(terminal(signal(lines), |_| {}))
}
