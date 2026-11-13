#[cfg(windows)]
use windows_core as _;

mod appearance;
#[cfg(feature = "audio")]
pub(crate) mod audio;
#[cfg(any(native_commands, global_hotkey))]
mod bridge;
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
#[cfg(global_hotkey)]
pub mod hotkey;
#[cfg(inspector)]
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
#[cfg(app_host)]
mod open;
#[cfg(any(native_commands, global_hotkey, inspector))]
pub(crate) type Wake = std::sync::Arc<dyn Fn() + Send + Sync + 'static>;
#[cfg(app_host)]
pub(crate) mod platform_prefs;
#[cfg(feature = "pty")]
pub(crate) mod pty;
#[cfg(app_host)]
mod theme_file;
#[cfg(app_host)]
mod theme_watch;
/// The system tray.
#[cfg(feature = "tray")]
pub mod tray;
pub(crate) mod tree_root;
#[cfg(feature = "desktop")]
pub(crate) mod window;

#[cfg(desktop_harness)]
#[doc(hidden)]
pub use crate::hal::shell::WindowOpts;
#[cfg(live_desktop)]
pub(crate) use crate::hal::shell::WindowOpts;
#[cfg(feature = "desktop")]
pub use crate::hal::shell::{Anchor, ResizeDirection, WindowCommand, WindowLevel};
#[cfg(scene_harness)]
#[doc(hidden)]
pub use crate::renderer::slug::{TextTuning, set_text_tuning, text_tuning};
#[cfg(scene_harness)]
#[doc(hidden)]
pub use crate::text::fonts::{DefaultFonts, default_fonts};
#[cfg(all(feature = "scene", target_arch = "wasm32"))]
pub use crate::text::load_font_url;
#[cfg(scene_harness)]
#[doc(hidden)]
pub use crate::text::web_fallbacks;
#[cfg(feature = "scene")]
pub use crate::text::{WebFont, WebFontScript};
#[cfg(feature = "audio")]
pub use audio::Player;
#[cfg(feature = "terminal")]
pub(crate) use cell_text::cell_glyph;
#[cfg(feature = "terminal")]
pub(crate) use cell_text::cell_text_lines;
pub use cell_text::cell_width;
#[cfg(feature = "terminal")]
pub(crate) use cell_text::{clip_cells, ellipsize_cells};
#[cfg(terminal_host)]
kittest_pub! {
    #[doc(hidden)]
    pub use cell_text::{ambiguous, set_cell_width};
}
#[cfg(terminal_host)]
pub use cell_text::Ambiguous;
kittest_pub! {
    #[cfg(terminal_host)]
    #[doc(hidden)]
    pub use cell_text::CellWidth;
}
pub use crate::kernel::HostMode;
#[cfg(feature = "terminal")]
pub(crate) use config::FRAME_INTERVAL;
pub use config::PanicPolicy;
#[cfg(feature = "terminal")]
pub(crate) use exit::{clear_exit, take_exit};
pub use frame::{Frame, FrameHooks, FrameInputHook, FrameLayoutHook, FramePaintHook};
pub(crate) use host_env::{HostCaps, is_entered};
pub use host_env::{is_grid, viewport_size};
#[cfg(feature = "desktop")]
pub(crate) use launch::launch;
#[cfg(app_host)]
pub use open::open_url;
#[cfg(feature = "pty")]
pub use pty::Session;
#[cfg(app_host)]
pub(crate) use theme_file::launch as launch_theme;
#[cfg(app_host)]
pub use theme_watch::{ThemeWatcher, watch_theme};
#[cfg(feature = "desktop")]
pub use window::{CloseAction, Window};

#[cfg(desktop_harness)]
pub use launch::__drive_test_frame;
