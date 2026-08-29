#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum PanicPolicy {
    #[default]
    Isolate,
    Abort,
}

#[derive(Debug, Clone)]
pub struct AppConfig {
    pub title: String,
    pub size: (u32, u32),
    pub esc_exits: bool,
    pub panic_policy: PanicPolicy,
    pub overlay: bool,
    pub frameless: bool,
    pub anchor: Option<sigil_hal::Anchor>,
    pub theme_file: Option<std::path::PathBuf>,
    pub user_zoom: f32,
}

impl AppConfig {
    #[must_use]
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            size: (720, 480),
            esc_exits: true,
            panic_policy: PanicPolicy::default(),
            overlay: false,
            frameless: false,
            anchor: None,
            theme_file: None,
            user_zoom: 1.0,
        }
    }

    #[must_use]
    pub const fn size(mut self, width: u32, height: u32) -> Self {
        self.size = (width, height);
        self
    }

    #[must_use]
    pub const fn esc_exits(mut self, yes: bool) -> Self {
        self.esc_exits = yes;
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
    pub const fn anchor(mut self, anchor: sigil_hal::Anchor) -> Self {
        self.anchor = Some(anchor);
        self
    }

    #[must_use]
    pub fn theme_file(mut self, path: impl Into<std::path::PathBuf>) -> Self {
        self.theme_file = Some(path.into());
        self
    }

    #[must_use]
    pub const fn user_zoom(mut self, zoom: f32) -> Self {
        self.user_zoom = zoom;
        self
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
