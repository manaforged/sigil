mod buffer;
#[cfg(test)]
#[path = "text_edit/buffer_tests.rs"]
mod buffer_tests;
mod builder;
mod handlers;
mod ime;
mod render;

pub(crate) use buffer::TextEdit;
pub use builder::SubmitKey;
pub(in crate::widgets) use builder::{MultilineConfig, build_multiline};
pub use handlers::TextState;
pub(in crate::widgets) use handlers::{
    KeyProfile, LineStep, TextEditRuntime, attach_a11y_edit, bind_command_keys, bind_history_keys,
    clipboard_keys, edit_keys, edit_value, history_keys, move_caret, noop_change, typed_event_text,
    word_keys,
};
pub(in crate::widgets) use ime::{attach_ime_handlers, compose_caret};
pub(crate) use render::splice_preedit;
pub(in crate::widgets) use render::{blink, overlay_rect};
