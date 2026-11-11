/// What a panic in a handler does.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum PanicPolicy {
    /// The isolate panic policy.
    #[default]
    Isolate,
    /// The abort panic policy.
    Abort,
}

#[cfg(feature = "desktop")]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Resize {
    Free,
    Fixed,
}

#[cfg(feature = "terminal")]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum OtherKeys {
    Plain,
    Modified,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct QuitKeys {
    pub(crate) esc: bool,
    pub(crate) ctrl_c: bool,
}

pub(crate) const FRAME_INTERVAL: std::time::Duration = std::time::Duration::from_micros(16_667);

const QUIT_KEYS: QuitKeys = QuitKeys {
    esc: false,
    ctrl_c: true,
};

#[cfg(feature = "desktop")]
const HOST_MODE: crate::kernel::HostMode = crate::kernel::HostMode::Pixel;
#[cfg(all(feature = "terminal", not(feature = "desktop")))]
const HOST_MODE: crate::kernel::HostMode = crate::kernel::HostMode::Cell;

mod defaults;
mod hooks;
#[cfg(feature = "terminal")]
mod terminal;
mod text;

#[cfg(feature = "desktop")]
pub(crate) use hooks::CloseHook;
#[cfg(any(feature = "desktop", feature = "terminal"))]
pub(crate) use hooks::ExitHook;
#[cfg(feature = "terminal")]
pub(crate) use hooks::TerminalHooks;
#[cfg(feature = "terminal")]
pub(crate) use terminal::TerminalOptions;

#[doc = include_str!("../../docs/app-config.md")]
#[derive(Debug, Clone)]
pub struct AppConfig {
    #[cfg(any(test, feature = "desktop", feature = "terminal"))]
    pub(crate) title: String,
    pub(crate) size: (u32, u32),
    pub(crate) quit: QuitKeys,
    pub(crate) panic_policy: PanicPolicy,
    #[cfg(feature = "desktop")]
    pub(crate) overlay: bool,
    #[cfg(feature = "desktop")]
    pub(crate) frameless: bool,
    #[cfg(feature = "desktop")]
    pub(crate) inset_title_bar: Option<f32>,
    #[cfg(feature = "desktop")]
    pub(crate) anchor: Option<crate::app::Anchor>,
    pub(crate) theme_file: Option<std::path::PathBuf>,
    #[cfg(feature = "desktop")]
    pub(crate) user_zoom: f32,
    #[cfg(any(feature = "desktop", feature = "terminal"))]
    pub(crate) host: crate::kernel::HostMode,
    #[cfg(feature = "desktop")]
    pub(crate) min_size: Option<(u32, u32)>,
    #[cfg(feature = "desktop")]
    pub(crate) resize: Resize,
    #[cfg(feature = "desktop")]
    pub(crate) on_close: CloseHook,
    #[cfg(any(feature = "desktop", feature = "terminal"))]
    pub(crate) on_exit: ExitHook,
    pub(crate) inline: bool,
    pub(crate) frame_interval: std::time::Duration,
    pub(crate) reduced_motion: Option<bool>,
    pub(crate) double_click_time: Option<std::time::Duration>,
    pub(crate) double_click_distance: Option<f32>,
    pub(crate) task_workers: crate::tasks::TaskWorkers,
    #[cfg(feature = "scene")]
    pub(crate) text: text::TextSetup,
    #[cfg(feature = "terminal")]
    pub(crate) terminal: TerminalOptions,
    #[cfg(feature = "desktop")]
    #[cfg_attr(
        not(target_arch = "wasm32"),
        allow(dead_code, reason = "only the browser host binds a canvas")
    )]
    pub(crate) canvas: Option<String>,
}

impl AppConfig {
    /// Creates an app config.
    #[must_use]
    pub fn new(title: impl Into<String>) -> Self {
        #[cfg(not(any(test, feature = "desktop", feature = "terminal")))]
        drop(title);
        let config = Self::defaults();
        #[cfg(any(test, feature = "desktop", feature = "terminal"))]
        let config = Self {
            title: title.into(),
            ..config
        };
        config
    }

    /// Sets the size.
    #[must_use]
    pub const fn size(mut self, width: u32, height: u32) -> Self {
        self.size = (width, height);
        self
    }

    /// Exits the app on an Escape press the UI did not consume.
    #[must_use]
    pub const fn esc_exits(mut self, yes: bool) -> Self {
        self.quit.esc = yes;
        self
    }

    /// Exits a terminal app on a Ctrl+C press the UI did not consume.
    #[must_use]
    pub const fn quit_on_ctrl_c(mut self, yes: bool) -> Self {
        self.quit.ctrl_c = yes;
        self
    }

    /// Sets the panic policy.
    #[must_use]
    pub const fn panic_policy(mut self, policy: PanicPolicy) -> Self {
        self.panic_policy = policy;
        self
    }

    /// Sets the overlay.
    #[cfg(feature = "desktop")]
    #[must_use]
    pub const fn overlay(mut self, yes: bool) -> Self {
        self.overlay = yes;
        self
    }

    /// Sets the frameless.
    #[cfg(feature = "desktop")]
    #[must_use]
    pub const fn frameless(mut self, yes: bool) -> Self {
        self.frameless = yes;
        self
    }

    /// Draws content under the title bar and centres the window controls in a bar this tall.
    #[cfg(feature = "desktop")]
    #[must_use]
    pub const fn inset_title_bar(mut self, height: f32) -> Self {
        self.inset_title_bar = Some(height);
        self
    }

    /// Sets the anchor.
    #[cfg(feature = "desktop")]
    #[must_use]
    pub const fn anchor(mut self, anchor: crate::app::Anchor) -> Self {
        self.anchor = Some(anchor);
        self
    }

    /// Sets the theme file.
    #[must_use]
    pub fn theme_file(mut self, path: impl Into<std::path::PathBuf>) -> Self {
        self.theme_file = Some(path.into());
        self
    }

    /// Applies the window option.
    #[cfg(feature = "desktop")]
    #[must_use]
    pub const fn window(mut self) -> Self {
        self.host = crate::kernel::HostMode::Pixel;
        self
    }

    /// Sets the canvas.
    #[cfg(feature = "desktop")]
    #[must_use]
    pub fn canvas(self, id: impl Into<String>) -> Self {
        #[cfg(target_arch = "wasm32")]
        {
            Self {
                canvas: Some(id.into()),
                ..self
            }
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            drop(id);
            self
        }
    }

    /// Applies the terminal option.
    #[cfg(feature = "terminal")]
    #[must_use]
    pub const fn terminal(mut self) -> Self {
        self.host = crate::kernel::HostMode::Cell;
        self
    }

    /// Terminal host hooks for `sigil::run`: `external_events`, `after_frame`, `region_rows`.
    #[cfg(feature = "terminal")]
    #[must_use]
    pub fn terminal_hooks(mut self, hooks: crate::terminal::Hooks<'static>) -> Self {
        self.terminal.hooks = TerminalHooks(std::rc::Rc::new(std::cell::RefCell::new(Some(hooks))));
        self
    }

    /// Screen-reader options for the terminal host.
    #[cfg(feature = "terminal")]
    #[must_use]
    pub const fn terminal_access(mut self, access: crate::terminal::Access) -> Self {
        self.terminal.access = access;
        self
    }

    /// Mouse reports the terminal host asks for at start.
    #[cfg(feature = "terminal")]
    #[must_use]
    pub const fn mouse_tracking(mut self, tracking: crate::terminal::MouseTracking) -> Self {
        self.terminal.mouse_tracking = Some(tracking);
        self
    }

    /// Terminal host.
    #[cfg(feature = "terminal")]
    #[must_use]
    pub const fn modify_other_keys(mut self, yes: bool) -> Self {
        self.terminal.other_keys = if yes {
            OtherKeys::Modified
        } else {
            OtherKeys::Plain
        };
        self
    }

    /// The key that suspends the terminal host (job control); the default is Ctrl+Z.
    #[cfg(feature = "terminal")]
    #[must_use]
    pub fn suspend_on(mut self, shortcut: impl Into<Option<crate::events::Shortcut>>) -> Self {
        self.terminal.suspend = shortcut.into();
        self
    }

    /// Run the terminal host in the normal screen below the prompt instead of the alternate screen.
    #[must_use]
    pub const fn inline(mut self, yes: bool) -> Self {
        self.inline = yes;
        self
    }

    /// Sets the user zoom.
    #[cfg(feature = "desktop")]
    #[must_use]
    pub const fn user_zoom(mut self, zoom: f32) -> Self {
        self.user_zoom = zoom;
        self
    }

    /// Sets the min size.
    #[cfg(feature = "desktop")]
    #[must_use]
    pub const fn min_size(mut self, width: u32, height: u32) -> Self {
        self.min_size = Some((width, height));
        self
    }

    /// Let the user resize the window (feature `desktop`).
    #[cfg(feature = "desktop")]
    #[must_use]
    pub const fn resizable(mut self, yes: bool) -> Self {
        self.resize = if yes { Resize::Free } else { Resize::Fixed };
        self
    }

    /// Sets the callback that runs on close.
    #[cfg(feature = "desktop")]
    #[must_use]
    pub fn on_close(mut self, decide: impl Fn() -> crate::app::CloseAction + 'static) -> Self {
        self.on_close = CloseHook(Some(std::rc::Rc::new(decide)));
        self
    }

    /// Sets the callback that runs once when the app exits, on every host.
    #[cfg(any(feature = "desktop", feature = "terminal"))]
    #[must_use]
    pub fn on_exit(mut self, run: impl Fn() + 'static) -> Self {
        self.on_exit = ExitHook(Some(std::rc::Rc::new(run)));
        self
    }

    /// Sets the frame interval.
    #[must_use]
    pub const fn frame_interval(mut self, interval: std::time::Duration) -> Self {
        self.frame_interval = interval;
        self
    }

    /// Override the OS reduced-motion setting that `reduced_motion()` reports.
    #[must_use]
    pub const fn reduced_motion(mut self, reduced: bool) -> Self {
        self.reduced_motion = Some(reduced);
        self
    }

    /// Set the double-click time.
    #[must_use]
    pub const fn double_click_time(mut self, time: std::time::Duration) -> Self {
        self.double_click_time = Some(time);
        self
    }

    /// Set the double-click distance in pixels.
    #[must_use]
    pub const fn double_click_distance(mut self, distance: f32) -> Self {
        self.double_click_distance = Some(distance);
        self
    }

    /// Fixes the worker thread count of the task runtime.
    #[must_use]
    pub const fn task_workers(mut self, count: core::num::NonZeroUsize) -> Self {
        self.task_workers = crate::tasks::TaskWorkers::Fixed(count);
        self
    }
}

impl From<&str> for AppConfig {
    fn from(title: &str) -> Self {
        Self::new(title)
    }
}

impl From<String> for AppConfig {
    fn from(title: String) -> Self {
        Self::new(title)
    }
}

#[cfg(test)]
#[path = "config_tests.rs"]
mod tests;
