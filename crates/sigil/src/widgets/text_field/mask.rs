pub(super) use crate::text::{byte_at_col as unmask_byte, col_at_byte as mask_byte};

use std::borrow::Cow;

use crate::reactive::Signal;
use crate::text::col_count;

pub(super) fn hiding(password: bool, reveal: Signal<bool>) -> bool {
    password && !reveal.get()
}

pub(super) fn hiding_peek(password: bool, reveal: Signal<bool>) -> bool {
    password && !reveal.peek()
}

pub(super) fn mask_chars(value: &str) -> String {
    "*".repeat(col_count(value))
}

pub(super) fn layout_text(text: &str, byte: usize, hiding: bool) -> (Cow<'_, str>, usize) {
    if hiding {
        (Cow::Owned(mask_chars(text)), mask_byte(text, byte))
    } else {
        (Cow::Borrowed(text), byte)
    }
}

#[cfg(test)]
#[path = "mask_tests.rs"]
mod tests;
