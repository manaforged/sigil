use sigil_reactive::Signal;

pub(super) fn hiding(password: bool, reveal: Signal<bool>) -> bool {
    password && !reveal.get()
}

pub(super) fn hiding_peek(password: bool, reveal: Signal<bool>) -> bool {
    password && !reveal.peek()
}

pub(super) fn mask_chars(value: &str) -> String {
    "*".repeat(value.chars().count())
}

pub(super) fn mask_byte(value: &str, byte: usize) -> usize {
    let byte = crate::text_edit::floor_char_boundary(value, byte.min(value.len()));
    value[..byte].chars().count()
}

pub(super) fn unmask_byte(value: &str, mask_byte: usize) -> usize {
    value.chars().take(mask_byte).map(char::len_utf8).sum()
}

#[cfg(test)]
#[path = "mask_tests.rs"]
mod tests;
