//! GPUI 0.2.2 Ledger runner. The model in [`ledger`] mirrors the Sigil
//! Ledger state; [`view`] renders it with the row chrome from GPUI's own
//! `examples/data_table.rs`.

#![allow(missing_docs)]

pub mod ledger;
mod view;

pub use ledger::{GPUI_VERSION, Ledger};
