#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]

use sigil::prelude::*;

const README: &str = "# sigil-demo\n\nA small **Sigil** app.\n\n- Reactive signals\n- Desktop and terminal hosts\n\nRun it with `cargo run`.";

fn main() -> sigil::AppResult<()> {
    sigil::run(("Sigil - editor", 1024, 640), app)
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
        "    sigil::run((\"Counter\", 320, 200), app)",
        "}",
        "",
        "fn app(cx: &mut Cx) -> NodeId {",
        "    let count = signal(0_i32);",
        "    let add = move || count.set(count.peek() + 1);",
        "    cx.add(button(\"Add\").activate(add))",
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
    tabs(active, vec!["main.rs".into(), "lib.rs".into()])
        .line()
        .content_for(
            0,
            container()
                .layout(LayoutStyle::flex_column().fill_width().padding(cols(2.0)))
                .child(code_edit(lines, caret)),
        )
        .content_for(1, empty_state("lib.rs", "This file is empty."))
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

pub(crate) fn app(cx: &mut Cx) -> NodeId {
    let file_menu = [
        MenuItem::new("Open").shortcut("Ctrl+O").select(|| {}),
        MenuItem::new("Save").shortcut("Ctrl+S").select(|| {}),
    ];
    cx.add(app_shell(
        menu_bar().menu("File", file_menu),
        work_area(),
        status_bar([label("main"), label("Ln 7, Col 5"), label("Rust")]),
        None::<NodeBuilder>,
    ))
}
