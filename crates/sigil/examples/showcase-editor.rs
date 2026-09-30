#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]

use sigil::prelude::*;
use sigil::widgets::{PanelBuilder, SplitterBuilder, TabsBuilder};

const README: &str = "# sigil-demo\n\nA small **Sigil** app.\n\n- Reactive signals\n- Desktop and terminal hosts\n\nRun it with `cargo run`.";

fn main() -> sigil::AppResult<()> {
    sigil::run(AppConfig::new("Sigil - editor").size(1024, 640), app)
}

fn file_list() -> NodeBuilder {
    sidebar(
        container()
            .layout(LayoutStyle::flex_column())
            .child(section_label("Files"))
            .child(list_row(true, "main.rs", "src"))
            .child(list_row(false, "lib.rs", "src"))
            .child(list_row(false, "Cargo.toml", ""))
            .child(list_row(false, "README.md", "")),
    )
}

fn source_lines() -> Vec<String> {
    [
        "use sigil::prelude::*;",
        "",
        "fn main() -> sigil::AppResult<()> {",
        "    sigil::run(\"Counter\", app)",
        "}",
        "",
        "fn app() -> impl IntoNode {",
        "    let count = signal(0_i32);",
        "    button(\"Add\").activate(move || count.update(|n| *n += 1))",
        "}",
    ]
    .into_iter()
    .map(String::from)
    .collect()
}

fn editor_tabs() -> TabsBuilder {
    let active = signal(0_usize);
    let lines = signal(source_lines());
    let caret = signal((6_usize, 4_usize));
    tabs()
        .active(active)
        .tab("main.rs", move || {
            container()
                .layout(LayoutStyle::flex_column().fill_width().padding(cols(2.0)))
                .child(code_edit(lines, caret))
        })
        .tab("lib.rs", move || {
            empty_state("lib.rs", "This file is empty.")
        })
        .line()
}

fn preview() -> PanelBuilder {
    let source = signal(String::from(README));
    panel().title("Preview").child(markdown(source))
}

fn work_area() -> SplitterBuilder {
    splitter(SplitterOrientation::Horizontal)
        .child(file_list(), 200.0)
        .child(editor_tabs(), 480.0)
        .child(preview(), 320.0)
}

pub(crate) fn app() -> impl IntoNode {
    let file_menu = [
        MenuItem::new("Open").shortcut("Ctrl+O").select(|| {}),
        MenuItem::new("Save").shortcut("Ctrl+S").select(|| {}),
    ];
    app_shell(
        menu_bar().menu("File", file_menu),
        work_area(),
        status_bar([
            text("main").nowrap(),
            text("Ln 7, Col 5").nowrap(),
            text("Rust").nowrap(),
        ]),
        None::<NodeBuilder>,
    )
}
