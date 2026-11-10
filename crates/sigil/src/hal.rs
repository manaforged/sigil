#[cfg(any(test, feature = "kittest", feature = "desktop"))]
pub(crate) mod a11y;
pub(crate) mod events;
pub(crate) mod geometry;
pub(crate) mod layout;
#[cfg(any(
    test,
    feature = "kittest",
    all(feature = "scene", feature = "desktop"),
    all(feature = "scene", feature = "terminal")
))]
pub(crate) mod renderer;
pub(crate) mod scheduler;
#[cfg(feature = "desktop")]
pub(crate) mod shell;
pub(crate) mod size;
pub(crate) mod text;

#[cfg(any(test, feature = "kittest", feature = "desktop"))]
pub use a11y::A11yBridge;
#[cfg(any(
    test,
    feature = "kittest",
    all(feature = "scene", feature = "desktop"),
    all(feature = "scene", feature = "terminal")
))]
pub use renderer::Renderer;
pub use scheduler::Scheduler;
#[cfg(feature = "desktop")]
pub use shell::Shell;
pub use text::{HitPosition, RenderedGlyph, TextAffinity};
#[cfg(any(test, feature = "kittest", feature = "scene"))]
pub use text::{ParagraphMetrics, TextEngine};
