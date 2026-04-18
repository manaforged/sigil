//! Ergonomic facade for building command-buffer scenes.

#![allow(clippy::redundant_pub_crate)]

use sigil_core::NodeId;
use sigil_hal::Rect;

use crate::scene_command::{
    CommandScene, DrawGlyphRun, DrawRect, GlyphRun, SceneCommand, SceneRectStyle, Transform,
};

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
            .push(SceneCommand::DrawRect(DrawRect::new(rect, style)));
    }

    #[doc(hidden)]
    pub fn draw_rect_for_node(&mut self, node: NodeId, rect: Rect, style: SceneRectStyle) {
        self.commands.push(SceneCommand::DrawRect(
            DrawRect::new(rect, style).with_node(node),
        ));
    }

    /// Emit a glyph-run draw command.
    pub fn draw_glyph_run(&mut self, run: GlyphRun) {
        self.commands
            .push(SceneCommand::DrawGlyphRun(DrawGlyphRun::new(run)));
    }

    #[doc(hidden)]
    pub fn draw_glyph_run_for_node(&mut self, node: NodeId, run: GlyphRun) {
        self.commands
            .push(SceneCommand::DrawGlyphRun(DrawGlyphRun::new(
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
}

impl ScopeKind {
    const fn close_command(self) -> SceneCommand {
        match self {
            Self::Clip => SceneCommand::PopClip,
            Self::Transform => SceneCommand::PopTransform,
            Self::Layer => SceneCommand::RestoreLayer,
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
