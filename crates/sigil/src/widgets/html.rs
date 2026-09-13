mod parse;

use crate::kernel::NodeBuilder;
use crate::reactive::{Signal, derived_eq};

use super::markdown::document;
use parse::parse_html;

#[must_use]
pub fn html(source: Signal<String>) -> NodeBuilder {
    let blocks = derived_eq(move || parse_html(&source.get()));
    document(blocks)
}

#[cfg(test)]
#[path = "html_tests.rs"]
mod tests;
