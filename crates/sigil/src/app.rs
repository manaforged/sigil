//! Application entry points: `Host`, `AppConfig`, `launch`, and embed support.
#[cfg(windows)]
use windows_core as _;

#[cfg(feature = "desktop")]
mod appearance;
pub mod audio;
mod cell_text;
mod config;
#[cfg(feature = "dialog")]
pub mod dialog;
mod embed;
mod error;
mod exit;
mod frame;
#[cfg(feature = "desktop")]
mod launch;
#[cfg(feature = "menu")]
pub mod menu;
#[cfg(feature = "desktop")]
pub mod open;
mod paint;
pub mod pty;
#[cfg(feature = "scene")]
mod text_geometry;
#[cfg(feature = "scene")]
mod text_measure;
#[cfg(feature = "desktop")]
mod theme_file;
#[cfg(feature = "desktop")]
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
#[cfg(feature = "desktop")]
pub use appearance::set_theme;
pub use audio::Player;
pub use cell_text::{CellWidth, cell_text_lines, cell_width, set_cell_width};
pub use config::{AppConfig, PanicPolicy};
pub use embed::{AccessibilityTree, Host, Painted};
pub use error::{AppResult, Error};
pub use exit::{clear_exit, request_exit, take_exit};
pub use frame::{
    FrameHooks, FrameInfo, FrameInputHook, FrameLayoutHook, FrameOutput, FramePaintHook,
};
#[cfg(feature = "desktop")]
pub use launch::launch;
#[cfg(feature = "desktop")]
pub use open::open;
pub use paint::TextFlow;
pub use pty::Session;
#[cfg(feature = "scene")]
pub use text_geometry::EngineGeometry;
#[cfg(feature = "scene")]
pub use text_measure::DefaultTextMeasure;
#[cfg(feature = "desktop")]
pub use theme_file::parse as parse_theme;
#[cfg(feature = "desktop")]
pub use theme_watch::{ThemeWatcher, watch_theme};
pub use tree_root::{TreeRoot, root};

#[cfg(feature = "desktop")]
#[doc(hidden)]
pub use launch::__drive_test_frame;
