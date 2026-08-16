//! Ergonomic facade for building command-buffer scenes.

#![allow(clippy::redundant_pub_crate)]

use sigil_hal::Rect;
use sigil_kernel::NodeId;
use sigil_style::Rgba8;

use crate::corner_radii::CornerRadii;
use crate::scene_command::{
    CommandScene, DrawDecoration, DrawGlyphRun, DrawGradient, DrawPattern, DrawRect, DrawSprite,
    GlyphRun, LayerScope, SceneCommand, SceneDecorationStyle, SceneGradientStyle,
    ScenePatternStyle, SceneRectStyle, SceneSpriteStyle, SpriteKind, SpriteSource, Transform,
};
use crate::shadow_paint::Shadow;

/// Builder for a command-buffer scene.
#[derive(Debug)]
pub struct SceneBuilder {
    commands: Vec<SceneCommand>,
}

impl SceneBuilder {
    /// Create an empty scene builder.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            commands: Vec::new(),
        }
    }

    /// Emit a rectangle draw command.
    pub fn draw_rect(&mut self, rect: Rect, style: SceneRectStyle) {
        self.commands
            .push(SceneCommand::Rect(DrawRect::new(rect, style)));
    }

    /// Emit a linear-gradient rectangle draw command.
    pub fn draw_gradient(&mut self, rect: Rect, style: SceneGradientStyle) {
        self.commands
            .push(SceneCommand::Gradient(DrawGradient::new(rect, style)));
    }

    /// Emit a pattern-filled rectangle draw command.
    pub fn draw_pattern(&mut self, rect: Rect, style: ScenePatternStyle) {
        self.commands
            .push(SceneCommand::Pattern(DrawPattern::new(rect, style)));
    }

    /// Emit a text-decoration draw command.
    ///
    /// For `DecorationKind::WavyUnderline` the `period` field is clamped
    /// to `>= 1.0` logical px to avoid a divide-by-zero NaN in the
    /// fragment shader. The shader has a matching `max(period, 1.0)`
    /// guard for callers that construct `DrawDecoration` directly. The
    /// Underline / Strikethrough kinds ignore `period`, so it passes
    /// through unchanged.
    pub fn draw_decoration(&mut self, style: SceneDecorationStyle) {
        let period = if matches!(
            style.kind,
            crate::scene_command::DecorationKind::WavyUnderline
        ) {
            style.period.max(1.0)
        } else {
            style.period
        };
        self.commands.push(SceneCommand::Decoration(DrawDecoration {
            node: None,
            kind: style.kind,
            band: style.band,
            color: style.color,
            opacity: style.opacity,
            period,
            line_width: style.line_width,
        }));
    }

    /// Emit a drop-shadow command.
    pub fn draw_shadow(&mut self, rect: Rect, radii: CornerRadii, shadow: Shadow) {
        self.commands.push(SceneCommand::DrawShadow {
            rect: rect_fields(rect),
            radii,
            shadow,
        });
    }

    /// Emit a raster-image draw command carrying the decoded pixels to
    /// paint time (where they upload to a texture once per content id).
    pub fn draw_image(&mut self, draw: crate::scene_command::DrawImage) {
        self.commands.push(SceneCommand::Image(draw));
    }

    #[doc(hidden)]
    pub fn draw_rect_for_node(&mut self, node: NodeId, rect: Rect, style: SceneRectStyle) {
        self.commands.push(SceneCommand::Rect(
            DrawRect::new(rect, style).with_node(node),
        ));
    }

    /// Emit a sprite draw command.
    pub fn draw_sprite(
        &mut self,
        source: SpriteSource,
        dest: Rect,
        kind: SpriteKind,
        color: Rgba8,
    ) -> &mut Self {
        let style = SceneSpriteStyle {
            source,
            kind,
            destination: dest,
            color,
            opacity: 10_000,
        };
        self.commands
            .push(SceneCommand::Sprite(DrawSprite::new(&style)));
        self
    }

    /// Emit a glyph-run draw command.
    pub fn draw_glyph_run(&mut self, run: GlyphRun) {
        self.commands
            .push(SceneCommand::GlyphRun(DrawGlyphRun::new(run)));
    }

    #[doc(hidden)]
    pub fn draw_glyph_run_for_node(&mut self, node: NodeId, run: GlyphRun) {
        self.commands.push(SceneCommand::GlyphRun(DrawGlyphRun::new(
            run.with_node(node),
        )));
    }

    /// Run `body` with `rect` installed as the current clip scope.
    pub fn with_clip<F: FnOnce(&mut Self)>(&mut self, rect: Rect, body: F) {
        self.with_scope(SceneCommand::PushClip(rect), ScopeKind::Clip, body);
    }

    /// Run `body` with `transform` installed as the current transform scope.
    pub fn with_transform<F: FnOnce(&mut Self)>(&mut self, transform: Transform, body: F) {
        self.with_scope(
            SceneCommand::PushTransform(transform),
            ScopeKind::Transform,
            body,
        );
    }

    /// Run `body` inside an opacity layer.
    pub fn with_layer<F: FnOnce(&mut Self)>(&mut self, opacity: f32, body: F) {
        self.with_scope(SceneCommand::SaveLayer { opacity }, ScopeKind::Layer, body);
    }

    /// Run `body` inside an offscreen compositing layer.
    pub fn with_composite_layer<F: FnOnce(&mut Self)>(&mut self, scope: LayerScope, body: F) {
        self.with_scope(SceneCommand::PushLayer(scope), ScopeKind::Offscreen, body);
    }

    /// Begin an offscreen layer scope.
    pub fn push_layer(&mut self, scope: LayerScope) {
        self.commands.push(SceneCommand::PushLayer(scope));
    }

    /// End the most-recently-opened offscreen layer scope.
    pub fn pop_layer(&mut self) {
        self.commands.push(SceneCommand::PopLayer);
    }

    /// Finish building and return the command scene.
    #[must_use]
    pub fn finish(self) -> CommandScene {
        CommandScene::new(self.commands)
    }

    fn with_scope<F: FnOnce(&mut Self)>(&mut self, open: SceneCommand, kind: ScopeKind, body: F) {
        self.commands.push(open);
        let mut scope = Scope::new(self, kind);
        body(scope.builder());
    }
}

const fn rect_fields(rect: Rect) -> [f32; 4] {
    let Rect {
        origin,
        width,
        height,
    } = rect;
    [origin.x, origin.y, width, height]
}

impl Default for SceneBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone, Copy, Debug)]
enum ScopeKind {
    Clip,
    Transform,
    Layer,
    Offscreen,
}

impl ScopeKind {
    const fn close_command(self) -> SceneCommand {
        match self {
            Self::Clip => SceneCommand::PopClip,
            Self::Transform => SceneCommand::PopTransform,
            Self::Layer => SceneCommand::RestoreLayer,
            Self::Offscreen => SceneCommand::PopLayer,
        }
    }
}

struct Scope<'builder> {
    builder: &'builder mut SceneBuilder,
    kind: ScopeKind,
}

impl<'builder> Scope<'builder> {
    const fn new(builder: &'builder mut SceneBuilder, kind: ScopeKind) -> Self {
        Self { builder, kind }
    }

    const fn builder(&mut self) -> &mut SceneBuilder {
        self.builder
    }
}

impl Drop for Scope<'_> {
    fn drop(&mut self) {
        self.builder.commands.push(self.kind.close_command());
    }
}
