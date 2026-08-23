//! HTML subset → the same document nodes as [`markdown`](super::markdown).

mod parse;

use sigil_kernel::NodeBuilder;
use sigil_reactive::{Signal, derived_eq};

use super::markdown::document;
use parse::parse_html;

/// Render `source` as a document column.
///
/// Headings, paragraphs, lists, fenced code, links, emphasis, tables.
/// `<script>` and `<style>` are dropped. Not a browser.
#[must_use]
pub fn html(source: Signal<String>) -> NodeBuilder {
    let blocks = derived_eq(move || parse_html(&source.get()));
    document(blocks)
}

#[cfg(test)]
#[path = "html_tests.rs"]
mod tests;
