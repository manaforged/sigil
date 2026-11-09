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
#[cfg(feature = "desktop")]
pub(crate) mod shell;
pub(crate) mod size;
pub(crate) mod text;

#[cfg(any(
    test,
    feature = "kittest",
    all(feature = "scene", feature = "desktop"),
    all(feature = "scene", feature = "terminal")
))]
pub use renderer::Renderer;
#[cfg(feature = "desktop")]
pub use shell::Shell;
pub use text::{HitPosition, RenderedGlyph, TextAffinity};
