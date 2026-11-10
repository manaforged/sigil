#[cfg(windows)]
use windows_core as _;

mod appearance;
#[cfg(feature = "audio")]
pub(crate) mod audio;
mod cell_text;
pub(crate) mod config;
/// Native file and message dialogs.
#[cfg(feature = "dialog")]
pub mod dialog;
pub(crate) mod embed;
pub(crate) mod exit;
mod frame;
mod host_env;
/// System-wide keyboard shortcuts.
#[cfg(all(feature = "hotkey", not(target_arch = "wasm32")))]
pub mod hotkey;
#[cfg(all(feature = "inspect", any(feature = "desktop", feature = "terminal")))]
pub(crate) mod inspect;
#[cfg(feature = "desktop")]
mod launch;
/// The operating system's now-playing controls.
#[cfg(feature = "media")]
pub mod media;
/// Menu bars and context menus.
#[cfg(feature = "menu")]
pub mod menu;
/// Desktop notifications.
#[cfg(feature = "notify")]
pub mod notify;
#[cfg(feature = "desktop")]
mod open;
#[cfg(any(
    feature = "menu",
    feature = "tray",
    all(feature = "hotkey", not(target_arch = "wasm32")),
    feature = "media"
))]
pub(crate) mod platform;
#[cfg(any(
    feature = "menu",
    feature = "tray",
    all(feature = "hotkey", not(target_arch = "wasm32")),
    feature = "media",
    all(feature = "inspect", any(feature = "desktop", feature = "terminal"))
))]
pub(crate) type Wake = std::sync::Arc<dyn Fn() + Send + Sync + 'static>;
#[cfg(any(feature = "desktop", feature = "terminal"))]
pub(crate) mod platform_prefs;
#[cfg(feature = "pty")]
pub(crate) mod pty;
#[cfg(feature = "scene")]
#[cfg(feature = "scene")]
mod text_measure;
#[cfg(any(feature = "desktop", feature = "terminal"))]
mod theme_file;
#[cfg(any(feature = "desktop", feature = "terminal"))]
mod theme_watch;
/// The system tray.
#[cfg(feature = "tray")]
pub mod tray;
pub(crate) mod tree_root;
#[cfg(feature = "desktop")]
pub(crate) mod window;

#[cfg(all(feature = "desktop", feature = "kittest"))]
#[doc(hidden)]
pub use crate::hal::shell::WindowOpts;
#[cfg(all(feature = "desktop", not(feature = "kittest")))]
pub(crate) use crate::hal::shell::WindowOpts;
#[cfg(feature = "desktop")]
pub use crate::hal::shell::{Anchor, ResizeDirection, WindowCommand, WindowLevel};
#[cfg(all(feature = "scene", feature = "kittest"))]
#[doc(hidden)]
pub use crate::renderer::slug::{TextTuning, set_text_tuning, text_tuning};
#[cfg(all(feature = "scene", feature = "kittest"))]
#[doc(hidden)]
pub use crate::text::fonts::{DefaultFonts, set_default_fonts};
#[cfg(all(feature = "scene", target_arch = "wasm32"))]
pub use crate::text::load_font_url;
#[cfg(all(feature = "scene", feature = "kittest"))]
#[doc(hidden)]
pub use crate::text::web_fallbacks;
#[cfg(feature = "scene")]
pub use crate::text::{WebFont, WebFontScript, set_web_fallbacks};
pub use appearance::set_theme;
#[cfg(feature = "audio")]
pub use audio::Player;
#[cfg(feature = "terminal")]
pub(crate) use cell_text::cell_glyph;
#[cfg(feature = "terminal")]
pub(crate) use cell_text::cell_text_lines;
pub use cell_text::cell_width;
#[cfg(any(test, feature = "kittest", feature = "terminal"))]
kittest_pub! {
    #[doc(hidden)]
    pub use cell_text::{set_ambiguous, set_cell_width};
}
#[cfg(any(test, feature = "terminal"))]
pub(crate) use cell_text::{Ambiguous, CellWidth};
#[cfg(feature = "terminal")]
pub(crate) use config::FRAME_INTERVAL;
pub use config::PanicPolicy;
#[cfg(feature = "terminal")]
pub(crate) use exit::{clear_exit, take_exit};
pub use frame::{FrameHooks, FrameInfo};
kittest_pub! {
    #[doc(hidden)]
    pub use frame::{FrameInputHook, FramePaintHook};
}
pub(crate) use host_env::HostCaps;
pub use host_env::{is_grid, viewport_size};
#[cfg(feature = "desktop")]
pub(crate) use launch::launch;
#[cfg(feature = "desktop")]
pub use open::open;
#[cfg(feature = "pty")]
pub use pty::Session;
#[cfg(feature = "scene")]
#[cfg(feature = "scene")]
kittest_pub! {
    #[doc(hidden)]
    pub use text_measure::DefaultTextMeasure;
}
#[cfg(any(feature = "desktop", feature = "terminal"))]
pub(crate) use theme_file::launch as launch_theme;
#[cfg(any(feature = "desktop", feature = "terminal"))]
pub use theme_watch::{ThemeWatcher, watch_theme};
#[cfg(feature = "desktop")]
pub(crate) use window::current_window;
#[cfg(feature = "desktop")]
pub use window::{CloseAction, WindowId};

#[cfg(all(feature = "desktop", any(test, feature = "kittest")))]
pub use launch::__drive_test_frame;
