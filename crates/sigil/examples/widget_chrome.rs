#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]

use sigil::prelude::*;
use sigil::widgets::{PanelBuilder, ScrollBuilder, SidebarBuilder, SplitterBuilder, TabsBuilder};

fn main() -> sigil::AppResult<()> {
    sigil::run(AppConfig::new("Sigil - chrome").size(720, 720), build)
}

fn app_shell_demo() -> NodeBuilder {
    let file_menu = [MenuItem::new("Quit").shortcut("Ctrl+Q").activate(|| {})];
    app_shell(
        menu_bar().menu("File", file_menu),
        container().child(text("Work area").nowrap()),
        status_bar([text("main").nowrap(), text("UTF-8").nowrap()]),
        None::<NodeBuilder>,
    )
}

fn panel_demo() -> PanelBuilder {
    panel()
        .title("Details")
        .child(text("Panel content").nowrap())
}

fn card_demo() -> NodeBuilder {
    card(kv_grid(&[("Host", "window"), ("Theme", "dark")]))
}

fn sidebar_demo() -> SidebarBuilder {
    sidebar(
        container()
            .layout(LayoutStyle::flex_column())
            .child(section_label("Project"))
            .child(list_row(true, "Overview", ""))
            .child(list_row(false, "Settings", "")),
    )
}

fn tabs_demo() -> TabsBuilder {
    let active = signal(0_usize);
    tabs()
        .active(active)
        .tab("Summary", move || text("Summary pane").nowrap())
        .tab("Details", move || text("Details pane").nowrap())
        .line()
}

fn splitter_demo() -> SplitterBuilder {
    splitter(Orientation::Horizontal)
        .child(card(text("Left").nowrap()), 160.0)
        .pane(card(text("Right").nowrap()))
        .min(120.0)
        .grow(1.0)
        .commit()
}

fn scroll_demo() -> ScrollBuilder {
    let content = (1..=20).fold(
        container().layout(LayoutStyle::flex_column()),
        |column, index| column.child(text(format!("Line {index}")).nowrap()),
    );
    scroll(content).height(120.0)
}

fn surface_demo() -> NodeBuilder {
    surface(1)
}

fn status_bar_demo() -> NodeBuilder {
    status_bar([
        text("main").nowrap(),
        text("Ln 12, Col 4").nowrap(),
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
            .child(panel_demo())
            .child(card_demo())
            .child(splitter_demo())
            .child(scroll_demo())
            .child(surface_demo())
            .child(status_bar_demo()),
    )
    .fill()
}
