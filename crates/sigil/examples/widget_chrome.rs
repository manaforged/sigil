#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]

use sigil::prelude::*;
use sigil::widgets::{CardBuilder, PanelBuilder, SplitterBuilder, TabsBuilder};

fn main() -> sigil::AppResult<()> {
    sigil::run(AppConfig::new("Sigil - chrome").size(720, 720), build)
}

pub(crate) fn app_shell_demo() -> NodeBuilder {
    container()
        .layout(LayoutStyle::flex_column().fill_width().height(200.0))
        .child(app_shell(
            text("Top bar").nowrap(),
            text("Work area").nowrap(),
            text("Status").nowrap(),
            None::<NodeBuilder>,
        ))
}

pub(crate) fn panel_demo() -> PanelBuilder {
    panel().title("Panel").child(text("Panel body").nowrap())
}

pub(crate) fn card_demo() -> CardBuilder {
    card(kv_grid([("Host", "wgpu"), ("Backend", "metal")]))
}

pub(crate) fn sidebar_demo() -> NodeBuilder {
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

pub(crate) fn tabs_demo() -> TabsBuilder {
    tabs()
        .active(signal(0_usize))
        .tab("First", move || text("First tab pane").nowrap())
        .tab("Second", move || text("Second tab pane").nowrap())
        .tab("Third", move || text("Third tab pane").nowrap())
}

pub(crate) fn tabs_line_demo() -> TabsBuilder {
    tabs()
        .active(signal(0_usize))
        .tab("Overview", move || text("Overview pane").nowrap())
        .tab("Activity", move || text("Activity pane").nowrap())
        .tab("Settings", move || text("Settings pane").nowrap())
        .line()
}

pub(crate) fn splitter_demo() -> SplitterBuilder {
    splitter(Orientation::Horizontal)
        .child(card(text("Navigation").nowrap()), 180.0)
        .child(card(text("Content").nowrap()), 260.0)
        .height(140.0)
}

fn scroll_lines() -> NodeBuilder {
    (1..=12).fold(
        container().layout(LayoutStyle::flex_column().fill_width().gap(4.0)),
        |column, index| column.child(text(format!("Scrolled line {index}")).nowrap()),
    )
}

pub(crate) fn scroll_demo() -> NodeBuilder {
    container()
        .layout(LayoutStyle::flex_column().fill_width().height(120.0))
        .child(scroll(scroll_lines()).extent(280.0).fill())
}

pub(crate) fn scroll_auto_demo() -> NodeBuilder {
    container()
        .layout(LayoutStyle::flex_column().fill_width().height(120.0))
        .child(scroll(scroll_lines()).fill())
}

pub(crate) fn surface_demo() -> NodeBuilder {
    surface(7).layout(LayoutStyle::flex_column().fixed_size(160.0, 48.0))
}

pub(crate) fn status_bar_demo() -> NodeBuilder {
    status_bar([
        text("main").nowrap(),
        text("Ln 4, Col 12").nowrap(),
        text("UTF-8").nowrap(),
    ])
}

fn build() -> impl IntoNode {
    scroll(
        container()
            .role(Role::Main)
            .layout(LayoutStyle::flex_column().gap(rows(1.0)).padding(cols(2.0)))
            .child(app_shell_demo())
            .child(
                container()
                    .layout(LayoutStyle::flex_row().gap(cols(1.0)))
                    .child(sidebar_demo())
                    .child(tabs_demo()),
            )
            .child(tabs_line_demo())
            .child(panel_demo())
            .child(card_demo())
            .child(splitter_demo())
            .child(scroll_demo())
            .child(scroll_auto_demo())
            .child(surface_demo())
            .child(status_bar_demo()),
    )
    .fill()
}
