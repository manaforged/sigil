mod clipboard;
mod graphics;
mod keys;
mod mouse;
mod paint;
mod run;
mod screen;
mod transcript;
mod write;

pub use clipboard::install_clipboard;
pub use graphics::{
    Mux, Placement, Protocol, base64, blocks, blocks_window, detect, detect_mux, mux, png,
    png_window, protocol, sextants, sextants_window,
};
pub use keys::{key_down, map_key, paste};
pub use mouse::{cell_point, click, map_mouse, pointer, wheel};
pub use paint::{CONTINUES, Cell, Grid, Ink, raster, raster_into};
pub use run::{AfterFrame, EventPump, Hooks, calibrate, present, run, run_with, step};
pub use screen::Screen;
pub use sigil::app::{
    AppConfig, AppResult, CellWidth, Error, Host, Painted, TreeRoot, cell_width, set_cell_width,
};
pub use sigil::events::EventRouter;
pub use write::render;

pub use sigil::widgets;

#[cfg(test)]
#[path = "lib_dev_deps_tests.rs"]
mod dev_deps;
