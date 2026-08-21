//! Compile-time assertion that an embedding host's text + render surface
//! resolves through `sigil::*` alone — never the internal `renderer` / `text`
//! / `hal` crates. See DR-054. Building this test target IS the assertion:
//! if any re-export is missing or renamed, the `use` blocks fail to compile.
#![allow(unused_imports)]
// A crate-level `#![cfg]` would strip this allow along with the rest of the
// file, so the feature gate lives on the items instead — with the feature
// off this crate is empty and every dev-dep is legitimately unused.
#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]

#[cfg(feature = "render")]
use sigil::render::{
    Color, CommandScene, DrawParams, OffscreenTarget, PhysicalSize, RenderedGlyph,
    ScreenSubmission, SlugTextPipeline, WgpuRenderer, WgpuSurface, emit_scene, submit_clear,
};
#[cfg(feature = "render")]
use sigil::text::{Font, FontCollection, OwnedTextEngine, Paragraph, TextConfig, TextError};

#[cfg(feature = "render")]
#[test]
fn render_surface_compiles() {
    // Resolving the imports above is the surface check; nothing to run here.
}
