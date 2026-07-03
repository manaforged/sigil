//! Compile-time assertion that an embedding host's text + render surface
//! resolves through `sigil::*` alone — never the internal `renderer` / `text`
//! / `hal` crates. See DR-054. Building this test target IS the assertion:
//! if any re-export is missing or renamed, the `use` blocks fail to compile.
#![cfg(feature = "render")]
#![allow(unused_imports)]

use sigil::render::{
    Color, CommandScene, DrawParams, OffscreenTarget, PhysicalSize, RenderedGlyph,
    ScreenSubmission, SlugTextPipeline, WgpuRenderer, WgpuSurface, emit_scene, submit_clear,
};
use sigil::text::{Font, FontCollection, OwnedTextEngine, Paragraph, TextConfig, TextError};

#[test]
fn render_surface_compiles() {
    // Resolving the imports above is the surface check; nothing to run here.
}
