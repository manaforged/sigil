#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum PanicPolicy {
    #[default]
    Isolate,
    Abort,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Resize {
    Free,
    Fixed,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct QuitKeys {
    pub(crate) esc: bool,
    pub(crate) ctrl_c: bool,
}

#[derive(Debug, Clone)]
pub struct AppConfig {
    pub(crate) title: String,
    pub(crate) size: (u32, u32),
    pub(crate) quit: QuitKeys,
    pub(crate) panic_policy: PanicPolicy,
    pub(crate) overlay: bool,
    pub(crate) frameless: bool,
    pub(crate) anchor: Option<crate::hal::Anchor>,
    pub(crate) theme_file: Option<std::path::PathBuf>,
    pub(crate) user_zoom: f32,
    pub(crate) host: crate::kernel::HostMode,
    pub(crate) min_size: Option<(u32, u32)>,
    pub(crate) resize: Resize,
    pub(crate) frame_interval: std::time::Duration,
}

impl AppConfig {
    #[must_use]
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            size: (720, 480),
            quit: QuitKeys {
                esc: false,
                ctrl_c: true,
            },
            panic_policy: PanicPolicy::default(),
            overlay: false,
            frameless: false,
            anchor: None,
            theme_file: None,
            user_zoom: 1.0,
            host: crate::kernel::HostMode::Pixel,
            min_size: None,
            resize: Resize::Free,
            frame_interval: std::time::Duration::from_millis(33),
        }
    }

    #[must_use]
    pub const fn size(mut self, width: u32, height: u32) -> Self {
        self.size = (width, height);
        self
    }

    #[must_use]
    pub const fn esc_exits(mut self, yes: bool) -> Self {
        self.quit.esc = yes;
        self
    }

    #[must_use]
    pub const fn quit_on_ctrl_c(mut self, yes: bool) -> Self {
        self.quit.ctrl_c = yes;
        self
    }

    #[must_use]
    pub const fn panic_policy(mut self, policy: PanicPolicy) -> Self {
        self.panic_policy = policy;
        self
    }

    #[must_use]
    pub const fn overlay(mut self, yes: bool) -> Self {
        self.overlay = yes;
        self
    }

    #[must_use]
    pub const fn frameless(mut self, yes: bool) -> Self {
        self.frameless = yes;
        self
    }

    #[must_use]
    pub const fn anchor(mut self, anchor: crate::hal::Anchor) -> Self {
        self.anchor = Some(anchor);
        self
    }

    #[must_use]
    pub fn theme_file(mut self, path: impl Into<std::path::PathBuf>) -> Self {
        self.theme_file = Some(path.into());
        self
    }

    #[must_use]
    pub const fn window(mut self) -> Self {
        self.host = crate::kernel::HostMode::Pixel;
        self
    }

    #[cfg(feature = "terminal")]
    #[must_use]
    pub const fn terminal(mut self) -> Self {
        self.host = crate::kernel::HostMode::Cell;
        self
    }

    #[must_use]
    pub const fn user_zoom(mut self, zoom: f32) -> Self {
        self.user_zoom = zoom;
        self
    }

    #[must_use]
    pub const fn min_size(mut self, width: u32, height: u32) -> Self {
        self.min_size = Some((width, height));
        self
    }

    #[must_use]
    pub const fn resizable(mut self, yes: bool) -> Self {
        self.resize = if yes { Resize::Free } else { Resize::Fixed };
        self
    }

    #[must_use]
    pub const fn frame_interval(mut self, interval: std::time::Duration) -> Self {
        self.frame_interval = interval;
        self
    }

    #[must_use]
    pub fn title(&self) -> &str {
        &self.title
    }

    #[must_use]
    pub const fn window_size(&self) -> (u32, u32) {
        self.size
    }

    #[must_use]
    pub const fn exits_on_esc(&self) -> bool {
        self.quit.esc
    }

    #[must_use]
    pub const fn on_panic(&self) -> PanicPolicy {
        self.panic_policy
    }
}

impl Default for AppConfig {
    fn default() -> Self {
        Self::new("Sigil")
    }
}

impl From<()> for AppConfig {
    fn from((): ()) -> Self {
        Self::default()
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

impl From<(&str, u32, u32)> for AppConfig {
    fn from((title, width, height): (&str, u32, u32)) -> Self {
        Self::new(title).size(width, height)
    }
}

impl From<(String, u32, u32)> for AppConfig {
    fn from((title, width, height): (String, u32, u32)) -> Self {
        Self::new(title).size(width, height)
    }
}
