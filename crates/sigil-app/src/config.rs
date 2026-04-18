//! `AppConfig` — window/runtime configuration for [`crate::launch`].
//!
//! Multiple shorthand forms are accepted via `impl Into<AppConfig>` so the
//! call site stays terse for the 80% case (`sigil::launch("title", tree)`)
//! while keeping the full struct available for anything bigger.

/// How the default keyboard shortcut policy dismisses a window.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum PanicPolicy {
    /// Default: the process continues; a panic in an effect/subtree is
    /// isolated by the error-boundary machinery (Phase 16).
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
