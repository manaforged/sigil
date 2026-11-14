#![allow(clippy::redundant_pub_crate)]
#[cfg(feature = "kittest")]
use crate::scene::command::DrawSprite;
use crate::scene::command::payloads::{PathCommand, PathStroke, outline_key};
use crate::scene::command::{DrawPattern, PatternKind, Wash};

use crate::geometry::{Point, Rect, Size};
use crate::kernel::NodeId;

use crate::scene::command::{BlendMode, Filter, LayerEffect};
use crate::scene::command::{
    CellRect, Clip, CommandScene, DrawDecoration, DrawGradient, DrawImage, DrawPath, DrawRect,
    GlyphRun, LayerScope, PathPaint, SceneCommand, StrokeCap, StrokeJoin, Transform,
};
use crate::scene::corner_radii::CornerRadii;
use crate::scene::geometry::IDENTITY;
use crate::scene::shadow::Shadow;
use crate::style::srgb::encode_byte;
use crate::style::{Color, TextLineHeight, TextStyle};

/// Appends drawing commands to a scene.
#[derive(Debug, Default)]
#[must_use]
pub struct SceneBuilder {
    commands: Vec<SceneCommand>,
}

impl SceneBuilder {
    /// Creates a scene builder with no commands.
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

    /// Paints a solid `color` rectangle at `rect`.
    pub fn fill_rect(&mut self, rect: Rect, color: Color) {
        self.commands
            .push(SceneCommand::Rect(DrawRect::new(rect, color)));
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

    /// Appends a gradient fill.
    pub fn draw_gradient(&mut self, draw: DrawGradient) {
        self.commands.push(SceneCommand::Gradient(draw));
    }

    #[cfg(feature = "kittest")]
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

    /// Appends a shadow for `rect` with corner radii `[top_left, top_right, bottom_right, bottom_left]`.
    pub fn draw_shadow(&mut self, rect: Rect, radii: [f32; 4], shadow: Shadow) {
        self.commands.push(SceneCommand::DrawShadow {
            rect: [rect.origin.x, rect.origin.y, rect.width, rect.height],
            radii,
            shadow,
        });
    }

    /// Appends an image.
    pub fn draw_image(&mut self, draw: DrawImage) {
        self.commands.push(SceneCommand::Image(draw));
    }

    /// Appends a vector path.
    pub fn draw_path(&mut self, draw: DrawPath) {
        self.commands.push(SceneCommand::Path(draw));
    }

    /// Strokes a straight line segment from `from` to `to`.
    pub fn stroke_line(&mut self, from: Point, to: Point, color: Color, width: f32) {
        let outline: [PathCommand; 2] = [
            PathCommand::MoveTo(from.x, from.y),
            PathCommand::LineTo(to.x, to.y),
        ];
        let stroke = PathStroke {
            paint: PathPaint::Solid(color),
            width,
            cap: StrokeCap::Butt,
            join: StrokeJoin::Miter,
            dash: smallvec::SmallVec::new(),
        };
        let seed = crate::fnv::mix_f32(
            crate::fnv::mix(crate::fnv::OFFSET, crate::fnv::rgba(color)),
            width,
        );
        let key = outline_key(seed, &outline);
        let draw = DrawPath::new(std::sync::Arc::from(outline), IDENTITY, key).with_stroke(stroke);
        self.draw_path(draw);
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

    #[cfg(feature = "kittest")]
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

    /// Shapes `text` with `style` and draws it at `origin`.
    pub fn draw_text(&mut self, origin: Point, text: &str, style: &TextStyle, color: Color) {
        let size = self.measure_text(text, style);
        let rect = Rect::new(origin, size.width, size.height);
        let run =
            crate::scene::command::shape_run(GlyphRun::new(rect, text, color, style.clone(), 1.0));
        self.draw_glyph_run(run);
    }

    /// Measures `text` styled with `style` without wrapping.
    #[must_use]
    #[allow(
        clippy::unused_self,
        reason = "method lives on SceneBuilder for consistency with draw_text"
    )]
    pub fn measure_text(&self, text: &str, style: &TextStyle) -> Size {
        if let Some(size) = crate::layout::with_text_geometry(|measure| {
            let content = crate::kernel::TextContent::Static(text.to_owned());
            measure.measure_styled(&content, style, crate::geometry::Constraint::max_content())
        }) {
            return size;
        }
        #[cfg(feature = "scene")]
        if let Ok(paragraph) =
            crate::text::OwnedTextEngine::thread().layout_paragraph_styled(text, None, style)
        {
            return Size::new(paragraph.width(), paragraph.height());
        }
        #[allow(
            clippy::cast_precision_loss,
            reason = "text column count fits cleanly within f32 mantissa"
        )]
        let width = text
            .split('\n')
            .map(|line| crate::grapheme::col_count(line) as f32 * 8.0)
            .fold(0.0_f32, f32::max);
        #[allow(
            clippy::cast_precision_loss,
            reason = "line count fits cleanly within f32 mantissa"
        )]
        let lines = text.split('\n').count().max(1) as f32;
        let line_height = match style.line_height {
            TextLineHeight::FontSizeRelative(factor) => {
                (style.font_size.max(1.0) * factor.max(0.1)).max(1.0)
            }
            TextLineHeight::Absolute(value) => value.max(1.0),
        };
        Size::new(width, lines * line_height)
    }

    /// Appends a run of glyphs that belongs to `node`.
    pub(crate) fn draw_glyph_run_for_node(&mut self, node: NodeId, run: GlyphRun) {
        self.commands
            .push(SceneCommand::GlyphRun(run.with_node(node)));
    }

    kittest_pub! {
    /// Draws `shadow` behind `run` like CSS `text-shadow`, with `shadow.blur` as the standard deviation; draw a run's shadows last to first, then the run.
    #[doc(hidden)]
    #[allow(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "alpha is clamped to 0..=1 before the byte cast"
    )]
    pub fn draw_text_shadow(&mut self, run: &GlyphRun, shadow: Shadow) {
        let [dx, dy] = shadow.offset;
        let [r, g, b, a] = shadow.color;
        let [r, g, b] = [r, g, b].map(encode_byte);
        let alpha = (a.clamp(0.0, 1.0) * 255.0).round() as u8;
        let mut copy = run.tinted(Color { r, g, b, a: alpha });
        copy.shift(dx, dy);
        if shadow.blur <= 0.0 {
            self.draw_glyph_run(copy);
            return;
        }
        let scope = LayerScope {
            clip: copy
                .rect
                .expand(shadow.blur.mul_add(3.0, copy.rect.height * 0.5)),
            clip_radii: CornerRadii::ZERO,
            effect: LayerEffect::Filter([Filter::Blur(shadow.blur)].into()),
            opacity: 1.0,
            blend: BlendMode::default(),
        };
        self.with_scope(
            SceneCommand::PushLayer(scope),
            SceneCommand::PopLayer,
            |scene| {
                scene.draw_glyph_run(copy);
            },
        );
    }
    }

    /// Clips the commands that `body` appends to `clip`.
    pub fn clip<F: FnOnce(&mut Self)>(&mut self, clip: impl Into<Clip>, body: F) {
        let open = SceneCommand::PushClip(clip.into());
        self.with_scope(open, SceneCommand::PopClip, body);
    }

    /// Draws `body` under `transform`.
    pub fn transform<F: FnOnce(&mut Self)>(&mut self, transform: Transform, body: F) {
        let open = SceneCommand::PushTransform(transform);
        self.with_scope(open, SceneCommand::PopTransform, body);
    }

    #[cfg(feature = "kittest")]
    /// Draw `body` into a layer composited at `opacity`.
    #[doc(hidden)]
    pub fn layer<F: FnOnce(&mut Self)>(&mut self, opacity: f32, body: F) {
        let open = SceneCommand::SaveLayer { opacity };
        self.with_scope(open, SceneCommand::RestoreLayer, body);
    }

    kittest_pub! {
    /// Draws the commands that `body` appends into a layer.
    #[doc(hidden)]
    pub fn composite<F: FnOnce(&mut Self)>(&mut self, scope: LayerScope, body: F) {
        self.with_scope(SceneCommand::PushLayer(scope), SceneCommand::PopLayer, body);
    }
    }

    #[cfg(feature = "kittest")]
    /// Opens a layer.
    #[doc(hidden)]
    pub fn push_layer(&mut self, scope: LayerScope) {
        self.commands.push(SceneCommand::PushLayer(scope));
    }

    kittest_pub! {
        if feature = "scene";
        /// Closes the last open layer.
        #[doc(hidden)]
        pub fn pop_layer(&mut self) {
            self.commands.push(SceneCommand::PopLayer);
        }
    }

    /// Consumes the builder and returns the scene of its commands.
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
