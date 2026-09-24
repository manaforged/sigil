use crate::reactive::Signal;
use crate::widgets::grapheme::{byte_at_col, col_at_byte, col_count};

pub(super) fn hiding(password: bool, reveal: Signal<bool>) -> bool {
    password && !reveal.get()
}

pub(super) fn hiding_peek(password: bool, reveal: Signal<bool>) -> bool {
    password && !reveal.peek()
}

pub(super) fn mask_chars(value: &str) -> String {
    "*".repeat(col_count(value))
}

pub(super) fn mask_byte(value: &str, byte: usize) -> usize {
    col_at_byte(value, byte)
}

pub(super) fn unmask_byte(value: &str, mask_byte: usize) -> usize {
    byte_at_col(value, mask_byte)
}

#[cfg(test)]
#[path = "mask_tests.rs"]
mod tests;
