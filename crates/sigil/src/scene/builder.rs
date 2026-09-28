#![allow(clippy::redundant_pub_crate)]

use crate::hal::Rect;
use crate::kernel::NodeId;
use crate::style::Rgba8;

use crate::scene::command::{
    CellRect, CommandScene, DrawDecoration, DrawGlyphRun, DrawGradient, DrawPattern, DrawRect,
    DrawSprite, GlyphRun, LayerScope, SceneCommand, SceneDecorationStyle, SceneGradientStyle,
    ScenePatternStyle, SceneRectStyle, SceneSpriteStyle, SpriteKind, SpriteSource, Transform,
};
use crate::scene::corner_radii::CornerRadii;
use crate::scene::shadow::Shadow;

#[derive(Debug)]
pub struct SceneBuilder {
    commands: Vec<SceneCommand>,
}

impl SceneBuilder {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            commands: Vec::new(),
        }
    }

    pub fn draw_rect(&mut self, rect: Rect, style: SceneRectStyle) {
        self.commands
            .push(SceneCommand::Rect(DrawRect::new(rect, style)));
    }

    pub fn draw_gradient(&mut self, rect: Rect, style: SceneGradientStyle) {
        self.commands
            .push(SceneCommand::Gradient(DrawGradient::new(rect, style)));
    }

    pub fn draw_pattern(&mut self, rect: Rect, style: ScenePatternStyle) {
        self.commands
            .push(SceneCommand::Pattern(DrawPattern::new(rect, style)));
    }

    pub fn draw_decoration(&mut self, style: SceneDecorationStyle) {
        self.commands.push(SceneCommand::Decoration(DrawDecoration {
            node: None,
            kind: style.kind,
            band: style.band,
            color: style.color,
            opacity: style.opacity,
            period: style.period,
            line_width: style.line_width,
        }));
    }

    pub fn draw_shadow(&mut self, rect: Rect, radii: CornerRadii, shadow: Shadow) {
        self.commands.push(SceneCommand::DrawShadow {
            rect: rect_fields(rect),
            radii,
            shadow,
        });
    }

    pub fn draw_image(&mut self, draw: crate::scene::command::DrawImage) {
        self.commands.push(SceneCommand::Image(draw));
    }

    pub(crate) fn draw_rect_for_node(
        &mut self,
        node: NodeId,
        rect: Rect,
        style: SceneRectStyle,
        cell: CellRect,
    ) {
        self.commands.push(SceneCommand::Rect(
            DrawRect::new(rect, style).with_node(node).with_cell(cell),
        ));
    }

    pub(crate) fn draw_gradient_for_node(
        &mut self,
        node: NodeId,
        rect: Rect,
        style: SceneGradientStyle,
        cell: CellRect,
    ) {
        self.commands.push(SceneCommand::Gradient(
            DrawGradient::new(rect, style)
                .with_node(node)
                .with_cell(cell),
        ));
    }

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

    pub fn draw_glyph_run(&mut self, run: GlyphRun) {
        self.commands
            .push(SceneCommand::GlyphRun(DrawGlyphRun::new(run)));
    }

    pub fn draw_glyph_run_for_node(&mut self, node: NodeId, run: GlyphRun) {
        self.commands.push(SceneCommand::GlyphRun(DrawGlyphRun::new(
            run.with_node(node),
        )));
    }

    pub fn with_clip<F: FnOnce(&mut Self)>(&mut self, rect: Rect, body: F) {
        self.with_scope(SceneCommand::PushClip(rect), ScopeKind::Clip, body);
    }

    pub fn with_transform<F: FnOnce(&mut Self)>(&mut self, transform: Transform, body: F) {
        self.with_scope(
            SceneCommand::PushTransform(transform),
            ScopeKind::Transform,
            body,
        );
    }

    pub fn with_layer<F: FnOnce(&mut Self)>(&mut self, opacity: f32, body: F) {
        self.with_scope(SceneCommand::SaveLayer { opacity }, ScopeKind::Layer, body);
    }

    pub fn with_composite_layer<F: FnOnce(&mut Self)>(&mut self, scope: LayerScope, body: F) {
        self.with_scope(SceneCommand::PushLayer(scope), ScopeKind::Offscreen, body);
    }

    pub fn push_layer(&mut self, scope: LayerScope) {
        self.commands.push(SceneCommand::PushLayer(scope));
    }

    pub fn pop_layer(&mut self) {
        self.commands.push(SceneCommand::PopLayer);
    }

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
