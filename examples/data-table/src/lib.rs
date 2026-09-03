//! Shared 10k × 24-column table used by the live window and bench-parity.
//!
//! `quote` / `fields` have no UI deps so the GPUI runner can format the
//! same cells. `view` is behind the `ui` feature (Sigil widgets).

#![allow(missing_docs)]

pub mod fields;
pub mod ledger;
pub mod quote;

#[cfg(feature = "ui")]
pub mod view;

pub use fields::{
    COLUMN_COUNT, FIELDS, Field, FieldKind, TABLE_WIDTH, VIEWPORT_HEIGHT, cell_text,
    columns_intersecting,
};
pub use quote::{ROW_HEIGHT, TOTAL_ITEMS, generate, tick_visible};
