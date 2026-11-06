#![allow(clippy::redundant_pub_crate)]
#[cfg(any(test, feature = "kittest"))]
use crate::scene::command::DrawSprite;
use crate::scene::command::{DrawPattern, PatternKind, Wash};

use crate::geometry::Rect;
use crate::kernel::NodeId;

use crate::scene::command::{
    CellRect, Clip, CommandScene, DrawDecoration, DrawGradient, DrawRect, GlyphRun, LayerScope,
    SceneCommand, Transform,
};
use crate::scene::corner_radii::CornerRadii;
use crate::scene::shadow::Shadow;

/// Appends drawing commands to a scene.
#[derive(Debug)]
pub struct SceneBuilder {
    commands: Vec<SceneCommand>,
}

impl SceneBuilder {
    /// Creates a scene builder.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            commands: Vec::new(),
        }
    }

    /// Fills `rect` with a watercolour wash.
    pub fn draw_wash(&mut self, rect: Rect, wash: Wash) {
        self.commands.push(SceneCommand::Pattern(DrawPattern::new(
            rect,
            PatternKind::Wash(wash),
        )));
    }

    /// Appends a rectangle.
    pub fn draw_rect(&mut self, draw: DrawRect) {
        self.commands.push(SceneCommand::Rect(draw));
    }

    pub(crate) const fn recorded(&self) -> usize {
        self.commands.len()
    }

    pub(crate) fn mark_leaf_from(&mut self, start: usize) {
        for command in self.commands.iter_mut().skip(start) {
            if let SceneCommand::Rect(draw) = command {
                draw.leaf = true;
            }
        }
    }

    #[cfg(any(test, feature = "kittest"))]
    /// Appends a gradient fill.
    #[doc(hidden)]
    pub fn draw_gradient(&mut self, draw: DrawGradient) {
        self.commands.push(SceneCommand::Gradient(draw));
    }

    #[cfg(any(test, feature = "kittest"))]
    /// Appends a pattern fill.
    #[doc(hidden)]
    pub fn draw_pattern(&mut self, draw: DrawPattern) {
        self.commands.push(SceneCommand::Pattern(draw));
    }

    kittest_pub! {
    /// Appends a text decoration.
    #[doc(hidden)]
    pub fn draw_decoration(&mut self, draw: DrawDecoration) {
        self.commands.push(SceneCommand::Decoration(draw));
    }
    }

    kittest_pub! {
    /// Appends a shadow for `rect`.
    #[doc(hidden)]
    pub fn draw_shadow(&mut self, rect: Rect, radii: CornerRadii, shadow: Shadow) {
        self.commands.push(SceneCommand::DrawShadow {
            rect: [rect.origin.x, rect.origin.y, rect.width, rect.height],
            radii,
            shadow,
        });
    }
    }

    kittest_pub! {
    /// Appends an image.
    #[doc(hidden)]
    pub fn draw_image(&mut self, draw: crate::scene::command::DrawImage) {
        self.commands.push(SceneCommand::Image(draw));
    }
    }

    pub(crate) fn draw_path(&mut self, draw: crate::scene::command::DrawPath) {
        self.commands.push(SceneCommand::Path(draw));
    }

    pub(crate) fn draw_rect_for_node(&mut self, node: NodeId, draw: DrawRect, cell: CellRect) {
        self.commands
            .push(SceneCommand::Rect(draw.with_node(node).with_cell(cell)));
    }

    pub(crate) fn draw_gradient_for_node(
        &mut self,
        node: NodeId,
        draw: DrawGradient,
        cell: CellRect,
    ) {
        self.commands
            .push(SceneCommand::Gradient(draw.with_node(node).with_cell(cell)));
    }

    #[cfg(any(test, feature = "kittest"))]
    /// Appends a sprite.
    #[doc(hidden)]
    pub fn draw_sprite(&mut self, draw: DrawSprite) -> &mut Self {
        self.commands.push(SceneCommand::Sprite(draw));
        self
    }

    /// Appends a run of glyphs.
    pub fn draw_glyph_run(&mut self, run: GlyphRun) {
        self.commands.push(SceneCommand::GlyphRun(run));
    }

    /// Appends a run of glyphs that belongs to `node`.
    pub fn draw_glyph_run_for_node(&mut self, node: NodeId, run: GlyphRun) {
        self.commands
            .push(SceneCommand::GlyphRun(run.with_node(node)));
    }

    /// Clips the commands that `body` appends to `clip`.
    pub fn with_clip<F: FnOnce(&mut Self)>(&mut self, clip: impl Into<Clip>, body: F) {
        let open = SceneCommand::PushClip(clip.into());
        self.with_scope(open, SceneCommand::PopClip, body);
    }

    /// Draw `body` under `transform`.
    pub fn with_transform<F: FnOnce(&mut Self)>(&mut self, transform: Transform, body: F) {
        let open = SceneCommand::PushTransform(transform);
        self.with_scope(open, SceneCommand::PopTransform, body);
    }

    /// Draw `body` into a layer composited at `opacity`.
    pub fn with_layer<F: FnOnce(&mut Self)>(&mut self, opacity: f32, body: F) {
        let open = SceneCommand::SaveLayer { opacity };
        self.with_scope(open, SceneCommand::RestoreLayer, body);
    }

    kittest_pub! {
    /// Draws the commands that `body` appends into a layer.
    #[doc(hidden)]
    pub fn with_composite_layer<F: FnOnce(&mut Self)>(&mut self, scope: LayerScope, body: F) {
        self.with_scope(SceneCommand::PushLayer(scope), SceneCommand::PopLayer, body);
    }
    }

    #[cfg(any(test, feature = "kittest"))]
    /// Opens a layer.
    #[doc(hidden)]
    pub fn push_layer(&mut self, scope: LayerScope) {
        self.commands.push(SceneCommand::PushLayer(scope));
    }

    /// Closes the last open layer.
    pub fn pop_layer(&mut self) {
        self.commands.push(SceneCommand::PopLayer);
    }

    /// Returns the finish.
    #[must_use]
    pub fn finish(self) -> CommandScene {
        CommandScene::new(self.commands)
    }

    pub(crate) const fn command_len(&self) -> usize {
        self.commands.len()
    }

    pub(crate) fn extend_commands(&mut self, cmds: impl IntoIterator<Item = SceneCommand>) {
        self.commands.extend(cmds);
    }

    pub(crate) fn copy_from(&self, start: usize) -> Vec<SceneCommand> {
        self.commands.get(start..).unwrap_or(&[]).to_vec()
    }

    fn with_scope<F: FnOnce(&mut Self)>(
        &mut self,
        open: SceneCommand,
        close: SceneCommand,
        body: F,
    ) {
        self.commands.push(open);
        body(self);
        self.commands.push(close);
    }
}

impl Default for SceneBuilder {
    fn default() -> Self {
        Self::new()
    }
}
