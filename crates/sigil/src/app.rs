#[cfg(windows)]
use windows_core as _;

mod appearance;
#[cfg(feature = "audio")]
pub mod audio;
mod cell_text;
mod config;
#[cfg(feature = "dialog")]
pub mod dialog;
mod embed;
mod error;
mod exit;
mod frame;
mod host_env;
#[cfg(feature = "desktop")]
mod launch;
#[cfg(feature = "menu")]
pub mod menu;
#[cfg(feature = "desktop")]
pub mod open;
#[cfg(feature = "pty")]
pub mod pty;
#[cfg(feature = "scene")]
mod text_geometry;
#[cfg(feature = "scene")]
mod text_measure;
#[cfg(any(feature = "desktop", feature = "terminal"))]
mod theme_file;
#[cfg(any(feature = "desktop", feature = "terminal"))]
mod theme_watch;
#[cfg(feature = "tray")]
pub mod tray;
mod tree_root;

pub use crate::hal::{Anchor, ResizeDirection, WindowCommand, WindowOpts};
pub use crate::layout::LayoutError;
pub use crate::profile;
#[cfg(feature = "scene")]
pub use crate::renderer::{TextTuning, set_text_tuning, text_tuning};
#[cfg(feature = "scene")]
pub use crate::text::{DefaultFonts, set_default_fonts, use_system_fonts};
pub use appearance::set_theme;
#[cfg(feature = "audio")]
pub use audio::Player;
pub use cell_text::{CellWidth, cell_text_lines, cell_width, set_cell_width};
pub use config::{AppConfig, PanicPolicy};
pub use embed::{AccessibilityTree, Host};
pub use error::{AppResult, Error, ErrorKind};
pub use exit::{clear_exit, request_exit, take_exit};
pub use frame::{
    FrameHooks, FrameInfo, FrameInputHook, FrameLayoutHook, FrameOutput, FramePaintHook,
};
pub use host_env::{HostCaps, is_grid, viewport_size};
#[cfg(feature = "desktop")]
pub(crate) use launch::launch;
#[cfg(feature = "desktop")]
pub use open::open;
#[cfg(feature = "pty")]
pub use pty::Session;
#[cfg(feature = "scene")]
pub use text_geometry::EngineGeometry;
#[cfg(feature = "scene")]
pub use text_measure::DefaultTextMeasure;
#[cfg(any(feature = "desktop", feature = "terminal"))]
pub(crate) use theme_file::launch as launch_theme;
#[cfg(any(feature = "desktop", feature = "terminal"))]
pub use theme_file::parse as parse_theme;
#[cfg(any(feature = "desktop", feature = "terminal"))]
pub use theme_watch::{ThemeWatcher, watch_theme};
pub use tree_root::{TreeRoot, root};

#[cfg(all(feature = "desktop", any(test, feature = "kittest")))]
pub use launch::__drive_test_frame;
