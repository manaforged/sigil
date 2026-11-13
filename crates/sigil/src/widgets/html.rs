mod parse;

use crate::kernel::NodeBuilder;
use crate::reactive::{Reactive, memo};

use super::markdown::document;
use parse::parse_html;

/// Rendered HTML subset.
pub fn html(source: impl Reactive<String>) -> NodeBuilder {
    let source = source.into_signal();
    let blocks = memo(move || parse_html(&source.get()));
    document(blocks)
}

#[cfg(test)]
#[path = "html_tests.rs"]
mod tests;
