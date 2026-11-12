mod parse;

use crate::kernel::NodeBuilder;
use crate::reactive::{Signal, memo};

use super::markdown::document;
use parse::parse_html;

/// Rendered HTML subset.
#[must_use]
pub fn html(source: Signal<String>) -> NodeBuilder {
    let blocks = memo(move || parse_html(&source.get()));
    document(blocks)
}

#[cfg(test)]
#[path = "html_tests.rs"]
mod tests;
