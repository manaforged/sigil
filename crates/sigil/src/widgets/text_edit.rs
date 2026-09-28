mod buffer;
#[cfg(test)]
#[path = "text_edit/buffer_tests.rs"]
mod buffer_tests;
mod builder;
mod handlers;
mod ime;
mod render;

pub(crate) use buffer::TextEdit;
pub use builder::{TextEditBuilder, text_edit};
pub use handlers::{FocusCaret, TextEditState};
pub(in crate::widgets) use handlers::{
    KeyProfile, LineStep, TextEditRuntime, attach_a11y_edit, clipboard_keys, commit, edit_keys,
    edit_value, history_keys, move_caret, noop_change, submit_text, word_keys,
};
pub(in crate::widgets) use ime::attach_ime_handlers;
pub(crate) use render::splice_preedit;
