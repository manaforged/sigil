pub(super) use crate::widgets::grapheme::{byte_at_col as unmask_byte, col_at_byte as mask_byte};

use crate::reactive::Signal;
use crate::widgets::grapheme::col_count;

pub(super) fn hiding(password: bool, reveal: Signal<bool>) -> bool {
    password && !reveal.get()
}

pub(super) fn hiding_peek(password: bool, reveal: Signal<bool>) -> bool {
    password && !reveal.peek()
}

pub(super) fn mask_chars(value: &str) -> String {
    "*".repeat(col_count(value))
}

#[cfg(test)]
#[path = "mask_tests.rs"]
mod tests;
