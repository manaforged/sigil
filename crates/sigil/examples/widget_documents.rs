#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]

use sigil::prelude::*;
use sigil::widgets::{CodeEditBuilder, DiffBuilder, MarkdownEditBuilder};

const NOTES: &str = "# Notes\n\nBody with **bold** and `code`.\n\n- one\n- two";
const PATCH: &str = "--- a/src/app.rs\n+++ b/src/app.rs\n@@ -1,3 +1,3 @@\n fn main() {\n-    println!(\"hello\");\n+    println!(\"hello, sigil\");\n }\n";

fn main() -> sigil::AppResult<()> {
    sigil::run(AppConfig::new("Sigil - documents").size(640, 720), build)
}

fn markdown_demo() -> NodeBuilder {
    let source = signal(String::from(NOTES));
    markdown(source)
}

fn markdown_edit_demo() -> MarkdownEditBuilder {
    let source = signal(String::from("## Draft\n\nEdit *this* text."));
    markdown_edit(source)
}

fn html_demo() -> NodeBuilder {
    let source = signal(String::from(
        "<h2>Release</h2><p>Version <b>0.1</b> is out.</p><ul><li>Faster layout</li></ul>",
    ));
    html(source)
}

fn code_edit_demo() -> CodeEditBuilder {
    let lines = signal(vec![
        String::from("fn main() {"),
        String::from("    println!(\"hello\");"),
        String::from("}"),
    ]);
    let caret = signal((0_usize, 0_usize));
    code_edit(lines, caret)
}

fn mermaid_demo() -> NodeBuilder {
    mermaid("graph TD\n  Edit --> Parse\n  Parse --> Paint")
}

fn diff_demo() -> DiffBuilder {
    diff(PATCH).path("src/app.rs").view(DiffView::Unified)
}

fn build() -> impl IntoNode {
    scroll(
        container()
            .role(Role::Main)
            .layout(LayoutStyle::flex_column().gap(rows(1.0)).padding(cols(2.0)))
            .child(markdown_demo())
            .child(markdown_edit_demo())
            .child(html_demo())
            .child(code_edit_demo())
            .child(mermaid_demo())
            .child(diff_demo()),
    )
    .fill()
}
