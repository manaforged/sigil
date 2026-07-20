//! Curated render + compositing surface for external host engines (feature `render`).
//!
//! Re-exports sigil's Slug text pipeline and widget-scene compositing so a
//! host (e.g. a game engine) writes `use sigil::render::{SlugTextPipeline, DrawParams,
//! WgpuRenderer, ScreenSubmission};` and composites a scene into a
//! caller-owned `wgpu::TextureView` — never naming the internal `renderer` /
//! `hal` crates directly. See DR-054.

pub use sigil_hal::{Color, PhysicalSize, RenderedGlyph};
pub use sigil_renderer::{
    CommandScene, CornerRadii, DecorationKind, DrawDecoration, DrawGlyphRun, DrawGradient,
    DrawParams, DrawPattern, DrawRect, DrawSprite, Fill, GlyphRun, GradientStop, LayerEffect,
    LayerScope, LinearGradient, OffscreenTarget, PatternKind, RendererError, Scene, SceneBuilder,
    SceneCommand, SceneDecorationStyle, SceneGradientStyle, SceneItem, ScenePatternStyle,
    SceneRect, SceneRectStyle, SceneSpriteStyle, SceneText, ScreenSubmission, Shadow, ShadowKind,
    SlugTextPipeline, SpriteKind, SpriteSource, Stroke, StrokeAlign, StrokeStyle, Transform,
    WgpuRenderer, WgpuSurface, WindowHandle, WorldSubmission, emit_scene, into_window_handle,
    submit_clear,
};
