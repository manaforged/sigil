//! Data table — headless core (DR-057 D1) + styled view (D2).
//!
//! [`TableState`] holds rows, columns, sort, quick-filter, and
//! selection as signals, deriving a visible index list a row
//! virtualizer binds to. [`table`] is the styled view over it: a
//! sticky header with sort indicators above a windowed
//! `virtual_list` body, so 100k-row tables render at viewport cost.

mod chrome;
mod columns;
mod derive;
mod header;
mod identity;
mod row;
mod state;
mod view;

pub use columns::{ColWidth, MIN_COL_WIDTH, TableColumn};
pub use identity::Identifiable;
pub use state::{ColumnLayout, SortDir, SortKey, TableState};
pub use view::{Density, TableBuilder, table};
