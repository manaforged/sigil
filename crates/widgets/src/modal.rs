//! Modal dialog and edge sheet. Pass the open signal:
//! `modal(open)` or `sheet(open)`.

mod builder;

pub use builder::{ModalBuilder, Side, modal, sheet};
