#[cfg(windows)]
use windows_core as _;

mod appearance;
#[cfg(feature = "audio")]
pub(crate) mod audio;
mod cell_text;
pub(crate) mod config;
#[cfg(feature = "dialog")]
pub mod dialog;
pub(crate) mod embed;
pub(crate) mod error;
pub(crate) mod exit;
mod frame;
mod host_env;
#[cfg(all(feature = "inspect", any(feature = "desktop", feature = "terminal")))]
pub(crate) mod inspect;
#[cfg(feature = "desktop")]
mod launch;
#[cfg(feature = "menu")]
pub mod menu;
#[cfg(feature = "desktop")]
mod open;
#[cfg(any(feature = "desktop", feature = "terminal"))]
pub(crate) mod platform_prefs;
#[cfg(feature = "pty")]
pub(crate) mod pty;
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
pub(crate) mod tree_root;

pub use crate::hal::shell::{Anchor, ResizeDirection, WindowCommand, WindowOpts};
#[cfg(feature = "scene")]
pub use crate::renderer::slug::{TextTuning, set_text_tuning, text_tuning};
#[cfg(feature = "scene")]
pub use crate::text::fonts::{DefaultFonts, set_default_fonts, use_system_fonts};
pub use appearance::set_theme;
#[cfg(feature = "audio")]
pub use audio::Player;
pub use cell_text::{
    Ambiguous, CellWidth, cell_text_lines, cell_width, set_ambiguous, set_cell_width,
};
#[cfg(feature = "terminal")]
pub(crate) use config::FRAME_INTERVAL;
pub use config::PanicPolicy;
pub use embed::AccessibilityTree;
pub use error::ErrorKind;
pub use exit::{clear_exit, take_exit};
pub use frame::{
    FrameHooks, FrameInfo, FrameInputHook, FrameLayoutHook, FrameOutput, FramePaintHook,
};
pub use host_env::{HostCaps, display_scale, is_grid, viewport_size};
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

#[cfg(all(feature = "desktop", any(test, feature = "kittest")))]
pub use launch::__drive_test_frame;
