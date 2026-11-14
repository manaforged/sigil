pub(crate) mod cursor;
pub(crate) mod events;
pub(crate) mod geometry;
pub(crate) mod layout;
#[cfg(scene_paint)]
pub(crate) mod renderer;
#[cfg(feature = "desktop")]
pub(crate) mod shell;
pub(crate) mod size;
pub(crate) mod text;

#[cfg(scene_paint)]
pub use renderer::Renderer;
#[cfg(feature = "desktop")]
pub use shell::Shell;
pub use text::{HitPosition, RenderedGlyph, TextAffinity};
