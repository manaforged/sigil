//! Terminal host for Sigil.
//!
//! Same [`Host`](sigil_app::Host), [`TreeRoot`], and widget builders as
//! desktop. This crate maps keys to [`sigil_hal::ShellEvent`] and paints
//! [`sigil_app::Painted`] onto a cell grid. It is not a second widget kit.

#![deny(missing_docs)]

mod keys;
mod mouse;
mod paint;
mod run;
mod timer;
mod work;
mod write;

pub use keys::{key_down, map_key, paste};
pub use mouse::map_mouse;
pub use paint::{CONTINUES, Cell, Grid, Ink, raster, raster_into};
pub use run::{Screen, present, run, step};
pub use sigil_app::{AppConfig, AppResult, Error, Host, Painted, TreeRoot};
pub use sigil_events::EventRouter;
pub use timer::{clear, every};
pub use work::{abandon, collect_work, work};
pub use write::render;

/// Widget builders — the same catalog desktop apps import from the facade.
pub use sigil_widgets as widgets;

/// Dev-dependencies the benchmark and integration tests use. The lint checks
/// them per target, and the lib's own unit tests touch none of them.
#[cfg(test)]
#[path = "lib_dev_deps_tests.rs"]
mod dev_deps;
