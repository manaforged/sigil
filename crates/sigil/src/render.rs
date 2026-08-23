//! GPU scene types for an embed host (feature `render`).
//!
//! Submit [`Host::scene`](crate::Host::scene) into a caller-owned
//! `wgpu::TextureView` through this module. Do not name `sigil-renderer`.

pub use sigil_hal::{Color, PhysicalSize, RenderedGlyph};
pub use sigil_renderer::{
    CommandScene, CornerRadii, DecorationKind, DrawDecoration, DrawGlyphRun, DrawGradient,
    DrawParams, DrawPattern, DrawRect, DrawSprite, Fill, GlyphRun, GradientStop, LayerEffect,
    LayerScope, LinearGradient, OffscreenTarget, PatternKind, RendererError, SceneBuilder,
    SceneCommand, SceneDecorationStyle, SceneGradientStyle, ScenePatternStyle, SceneRectStyle,
    SceneSpriteStyle, ScreenSubmission, Shadow, ShadowKind, SlugTextPipeline, SpriteKind,
    SpriteSource, Stroke, StrokeAlign, StrokeStyle, Transform, WgpuRenderer, WgpuSurface,
    WindowHandle, WorldSubmission, emit_scene, into_window_handle, submit_clear,
};
