#[cfg(windows)]
use windows_core as _;

use sigil_reactive as _;
use sigil_style as _;

#[cfg(test)]
use sigil as _;

pub mod audio;
mod config;
#[cfg(feature = "dialog")]
pub mod dialog;
mod embed;
mod error;
mod exit;
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
#[cfg(feature = "tray")]
pub mod tray;
mod tree_root;

pub use audio::Player;
pub use config::{AppConfig, PanicPolicy};
pub use embed::{AccessibilityTree, Host, Painted};
pub use error::{AppResult, Error};
pub use exit::{clear_exit, request_exit, take_exit};
#[cfg(feature = "desktop")]
pub use launch::launch;
#[cfg(feature = "desktop")]
pub use open::open;
pub use pty::Session;
pub use sigil_hal::{Anchor, ResizeDirection, WindowCommand, WindowOpts};
pub use sigil_layout::LayoutError;
pub use sigil_profile as profile;
#[cfg(feature = "scene")]
pub use sigil_renderer::{TextTuning, set_text_tuning, text_tuning};
#[cfg(feature = "scene")]
pub use sigil_text::{DefaultFonts, set_default_fonts, use_system_fonts};
#[cfg(feature = "scene")]
pub use text_geometry::EngineGeometry;
#[cfg(feature = "scene")]
pub use text_measure::DefaultTextMeasure;
pub use tree_root::{TreeRoot, root};

#[cfg(feature = "desktop")]
pub use launch::install_g5_executor;

#[cfg(feature = "desktop")]
#[doc(hidden)]
pub use launch::__drive_test_frame;

#[cfg(not(feature = "desktop"))]
mod desktop_unused {
    use accesskit as _;
    use crossbeam_channel as _;
    use notify as _;
    use sigil_a11y as _;
    use sigil_animation_runtime as _;
    use sigil_tasks as _;
    use toml as _;
    use tracing as _;
}
