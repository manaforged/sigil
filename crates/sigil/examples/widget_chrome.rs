#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]

use sigil::prelude::*;

fn main() -> sigil::AppResult<()> {
    sigil::run(("Sigil - chrome", 720, 720), build)
}

fn app_shell_demo() -> NodeBuilder {
    let file_menu = [MenuItem::new("Quit").shortcut("Ctrl+Q").select(|| {})];
    app_shell(
        menu_bar().menu("File", file_menu),
        container().child(label("Work area")),
        status_bar([label("main"), label("UTF-8")]),
        None::<NodeBuilder>,
    )
}

fn panel_demo() -> PanelBuilder {
    panel().title("Details").child(label("Panel content"))
}

fn card_demo() -> NodeBuilder {
    card(kv_grid(&[("Host", "window"), ("Theme", "dark")]))
}

fn sidebar_demo() -> NodeBuilder {
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
    tabs(active, vec!["Summary".into(), "Details".into()])
        .line()
        .content_for(0, label("Summary pane"))
        .content_for(1, label("Details pane"))
}

fn splitter_demo() -> SplitterBuilder {
    splitter(SplitterOrientation::Horizontal)
        .child(card(label("Left")), 160.0)
        .pane(card(label("Right")))
        .min(120.0)
        .grow(1.0)
        .commit()
}

fn scroll_demo() -> ScrollBuilder {
    let content = (1..=20).fold(
        container().layout(LayoutStyle::flex_column()),
        |column, index| column.child(label(format!("Line {index}"))),
    );
    scroll_auto(content).height(120.0)
}

fn surface_demo() -> NodeBuilder {
    surface(1)
}

fn status_bar_demo() -> NodeBuilder {
    status_bar([label("main"), label("Ln 12, Col 4"), label("UTF-8")])
}

fn build(cx: &mut Cx) -> NodeId {
    cx.add(
        scroll_auto(
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
        .fill(),
    )
}
