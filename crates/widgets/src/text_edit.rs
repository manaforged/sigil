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
pub(crate) use render::floor_char_boundary;
