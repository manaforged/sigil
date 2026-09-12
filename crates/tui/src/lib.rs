mod graphics;
mod keys;
mod mouse;
mod paint;
mod run;
mod timer;
mod work;
mod write;

pub use graphics::{
    Placement, Protocol, base64, blocks, blocks_window, detect, png, png_window, protocol,
};
pub use keys::{key_down, map_key, paste};
pub use mouse::{cell_point, click, map_mouse, pointer, wheel};
pub use paint::{CONTINUES, Cell, Grid, Ink, raster, raster_into};
pub use run::{Screen, calibrate, present, run, step};
pub use sigil_app::{
    AppConfig, AppResult, CellWidth, Error, Host, Painted, TreeRoot, cell_width, set_cell_width,
};
pub use sigil_events::EventRouter;
pub use timer::{clear, every};
pub use work::{abandon, collect_work, work};
pub use write::render;

pub use sigil_widgets as widgets;

#[cfg(test)]
#[path = "lib_dev_deps_tests.rs"]
mod dev_deps;
