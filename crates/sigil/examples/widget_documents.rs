#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]

use sigil::prelude::*;
use sigil::widgets::{CodeEditBuilder, DiffBuilder};

const PATCH: &str = "--- a/src/app.rs\n+++ b/src/app.rs\n@@ -1,3 +1,3 @@\n fn main() {\n-    old();\n+    fresh();\n }";
const QUADRANTS: &str = "iVBORw0KGgoAAAANSUhEUgAAAGAAAAAwCAYAAADuFn/PAAAAeklEQVR42u3SQQ0AQAgDQYQh7MTihVPBo8k8VkAzrene5PpNdAUAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAwAFA+oD0AwEAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAcNAHChr0hrUljjwAAAAASUVORK5CYII=";

fn main() -> sigil::AppResult<()> {
    sigil::run(AppConfig::new("Sigil - documents").size(640, 720), build)
}

pub(crate) fn markdown_demo() -> NodeBuilder {
    markdown(signal(String::from(
        "# Heading\n\nBody with **bold** and `code`.\n\n- one\n- two",
    )))
}

pub(crate) fn markdown_image_demo() -> NodeBuilder {
    markdown(signal(format!(
        "Image:\n\n![quadrants](data:image/png;base64,{QUADRANTS})"
    )))
}

pub(crate) fn markdown_edit_demo(disabled: Signal<bool>) -> NodeBuilder {
    let source = signal(String::from(
        "# Sigil widgets\n\nEdit me, *live* preview.\n\n- one\n- two",
    ));
    container()
        .layout(LayoutStyle::flex_column().fill_width().height(228.0))
        .child(markdown_edit(source).disabled(disabled))
}

pub(crate) fn html_demo() -> NodeBuilder {
    html(signal(String::from(
        "<h2>Release notes</h2><p>Sigil renders <b>HTML</b> with <i>inline</i> styles.</p>",
    )))
}

pub(crate) fn code_edit_demo() -> CodeEditBuilder {
    let lines = vec![
        String::from("fn main() {"),
        String::from("    println!(\"hi\");"),
        String::from("}"),
    ];
    code_edit(signal(lines), signal((1, 4)))
}

pub(crate) fn mermaid_demo() -> NodeBuilder {
    mermaid("graph TD\n  Edit --> Parse\n  Parse --> Paint")
}

pub(crate) fn diff_demo() -> DiffBuilder {
    diff(PATCH)
        .path("src/app.rs")
        .view(DiffView::Unified)
        .max_rows(24)
}

fn build() -> impl IntoNode {
    scroll(
        container()
            .role(Role::Main)
            .layout(LayoutStyle::flex_column().gap(rows(1.0)).padding(cols(2.0)))
            .child(markdown_demo())
            .child(markdown_image_demo())
            .child(markdown_edit_demo(signal(false)))
            .child(html_demo())
            .child(code_edit_demo())
            .child(mermaid_demo())
            .child(diff_demo()),
    )
    .fill()
}
