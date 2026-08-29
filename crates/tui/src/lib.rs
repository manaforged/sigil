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

pub use sigil_widgets as widgets;

#[cfg(test)]
#[path = "lib_dev_deps_tests.rs"]
mod dev_deps;
