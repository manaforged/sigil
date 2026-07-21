//! `AppConfig` — window/runtime configuration for [`crate::launch`].
//!
//! Multiple shorthand forms are accepted via `impl Into<AppConfig>` so the
//! call site stays terse for the 80% case (`sigil::launch("title", tree)`)
//! while keeping the full struct available for anything bigger.

/// How the default keyboard shortcut policy dismisses a window.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum PanicPolicy {
    /// Default: a panic inside a frame (a widget handler, an effect, a
    /// paint closure) is caught by the launch loop, logged via `tracing`,
    /// and the reactive runtime is reset to a clean resting state so the
    /// next frame still runs. The panicking effect itself is dropped (the
    /// reactive runtime disposes an effect that unwinds mid-run); the app
    /// stays alive. Wired in `launch::run_frame`.
    #[default]
    Isolate,
    /// Abort the process on any panic. Matches `panic = "abort"` in the
    /// binary's profile when you want a crash-report pipeline instead of
    /// silent subtree loss.
    Abort,
}

/// Full launch configuration.
///
/// Instances are normally built via the `Into` impls rather than
/// by hand:
///
/// ```text
/// sigil::launch("Demo", tree);                   // title only
/// sigil::launch(("Demo", 1280, 720), tree);      // title + size
/// sigil::launch(AppConfig::new("Demo").size(1280, 720).esc_exits(false), tree);
/// ```
#[derive(Debug, Clone)]
pub struct AppConfig {
    /// Window title.
    pub title: String,
    /// Initial window size in logical pixels. The launch loop converts
    /// to physical pixels using the window's current scale factor.
    pub size: (u32, u32),
    /// Whether pressing `Esc` triggers an exit request. Default: `true`.
    pub esc_exits: bool,
    /// Panic policy for root-level fallback. Default: isolate.
    pub panic_policy: PanicPolicy,
    /// Overlay bundle: the window is undecorated, transparent,
    /// always-on-top, shadowless, created inactive and click-through,
    /// and the process runs as an accessory on macOS (no Dock icon or
    /// Cmd-Tab entry; it never steals focus). The frame clear is fully
    /// transparent, so whatever the tree does not paint shows the
    /// desktop behind it. Meant for status pills. Default: `false`.
    pub overlay: bool,
    /// Optional placement resolved against the primary monitor when the
    /// window materialises. `None` accepts the OS default position.
    pub anchor: Option<sigil_hal::Anchor>,
    /// Dev-mode edit-see loop (5/5 Wave 4 T14): watch this theme TOML
    /// file and repaint live on save — no rebuild. See
    /// `crates/app/src/theme_file.rs` for the format. Default: `None`.
    pub theme_file: Option<std::path::PathBuf>,
}

impl AppConfig {
    /// Build a new config with `title` and the default 720×480 window.
    #[must_use]
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            size: (720, 480),
            esc_exits: true,
            panic_policy: PanicPolicy::default(),
            overlay: false,
            anchor: None,
            theme_file: None,
        }
    }

    /// Chainable size setter.
    #[must_use]
    pub const fn size(mut self, width: u32, height: u32) -> Self {
        self.size = (width, height);
        self
    }

    /// Chainable ESC-exits policy setter.
    #[must_use]
    pub const fn esc_exits(mut self, yes: bool) -> Self {
        self.esc_exits = yes;
        self
    }

    /// Chainable panic-policy setter.
    #[must_use]
    pub const fn panic_policy(mut self, policy: PanicPolicy) -> Self {
        self.panic_policy = policy;
        self
    }

    /// Chainable overlay-bundle setter (see [`Self::overlay`] field docs).
    #[must_use]
    pub const fn overlay(mut self, yes: bool) -> Self {
        self.overlay = yes;
        self
    }

    /// Chainable placement setter (see [`Self::anchor`] field docs).
    #[must_use]
    pub const fn anchor(mut self, anchor: sigil_hal::Anchor) -> Self {
        self.anchor = Some(anchor);
        self
    }

    /// Chainable theme-file watcher (see [`Self::theme_file`] field docs).
    #[must_use]
    pub fn theme_file(mut self, path: impl Into<std::path::PathBuf>) -> Self {
        self.theme_file = Some(path.into());
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
