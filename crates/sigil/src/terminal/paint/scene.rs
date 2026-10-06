use crate::geometry::Rect;
use crate::scene::{
    CellBorder, CommandScene, DrawDecoration, DrawGlyphRun, DrawGradient, DrawImage, DrawRect,
    LinearGradient, SceneCommand,
};
use crate::style::{Rgba8, Theme, Tone};
use crate::terminal::caps::{Caps, ColorDepth};

use super::decoration::{self, Edge, Surface};
use super::marks::Marks;
use super::paint_clip::{Text, image_crop, paint_text};
use super::transform::Space;
use super::{Face, Grid, Ink, Underline, cell_edge, floor_cell};

pub struct Paint<'a> {
    pub theme: &'a Theme,
    pub cell: f32,
    pub caps: Caps,
    pub marks: &'a Marks,
}

#[must_use]
pub fn raster(scene: &CommandScene, theme: &Theme, cols: u16, rows: u16, cell: f32) -> Grid {
    let mut grid = Grid::new(cols, rows);
    raster_into(&mut grid, scene, theme, cell);
    grid
}

pub fn raster_into(grid: &mut Grid, scene: &CommandScene, theme: &Theme, cell: f32) {
    let marks = Marks::default();
    let paint = Paint {
        theme,
        cell,
        caps: Caps::default(),
        marks: &marks,
    };
    raster_with(grid, scene, &paint);
}

pub fn raster_with(grid: &mut Grid, scene: &CommandScene, paint: &Paint<'_>) {
    grid.reset();
    let mut walk = Walk {
        grid,
        paint,
        theme: paint.theme,
        cell: if paint.cell <= 0.0 { 1.0 } else { paint.cell },
        clips: Vec::new(),
        alphas: Vec::new(),
        space: Space::default(),
    };
    for command in &scene.commands {
        walk.command(command);
    }
    grid.map_glyphs(paint.caps.glyphs);
}

fn gradient_flat(gradient: &LinearGradient) -> Rgba8 {
    let mut stops = gradient.stops.iter();
    let Some(first) = stops.next() else {
        return Rgba8 {
            r: 0,
            g: 0,
            b: 0,
            a: 0,
        };
    };
    if first.offset >= 0.5 {
        return first.color;
    }
    let mut prev = first;
    for next in stops {
        if next.offset >= 0.5 {
            let span = (next.offset - prev.offset).max(f32::EPSILON);
            let t = ((0.5 - prev.offset) / span).clamp(0.0, 1.0);
            return blend(prev.color, next.color, t);
        }
        prev = next;
    }
    prev.color
}

#[allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    reason = "channels are clamped to the u8 range before the cast"
)]
fn blend(a: Rgba8, b: Rgba8, t: f32) -> Rgba8 {
    let mix = |x: u8, y: u8| (f32::from(x).mul_add(1.0 - t, f32::from(y) * t)).round() as u8;
    Rgba8 {
        r: mix(a.r, b.r),
        g: mix(a.g, b.g),
        b: mix(a.b, b.b),
        a: mix(a.a, b.a),
    }
}

struct Walk<'a> {
    grid: &'a mut Grid,
    paint: &'a Paint<'a>,
    theme: &'a Theme,
    cell: f32,
    clips: Vec<Rect>,
    alphas: Vec<f32>,
    space: Space,
}

type Area = (Rect, (usize, usize), (usize, usize));

impl Walk<'_> {
    const fn ink(&self, colour: Rgba8, tone: Option<Tone>) -> Ink {
        match (self.paint.caps.color, tone) {
            (ColorDepth::Ansi16 | ColorDepth::Monochrome, Some(tone)) => Ink::Of(tone),
            _ => Ink::pick(colour, tone),
        }
    }

    fn command(&mut self, command: &SceneCommand) {
        match command {
            SceneCommand::PushClip(rect) => self.push_clip(*rect),
            SceneCommand::PopClip => {
                self.clips.pop();
            }
            SceneCommand::SaveLayer { opacity } => self.alphas.push(*opacity),
            SceneCommand::RestoreLayer => {
                self.alphas.pop();
            }
            SceneCommand::PushLayer(scope) => {
                self.push_clip(scope.clip);
                self.alphas.push(scope.opacity);
                self.space.push_effect(scope.effect);
            }
            SceneCommand::PopLayer => {
                self.clips.pop();
                self.alphas.pop();
                self.space.pop();
            }
            SceneCommand::PushTransform(transform) => self.space.push(*transform),
            SceneCommand::PopTransform => self.space.pop(),
            SceneCommand::Rect(draw) => self.rect(draw),
            SceneCommand::Gradient(draw) => self.gradient(draw),
            SceneCommand::GlyphRun(draw) => self.text(draw),
            SceneCommand::Image(draw) => self.image(draw),
            SceneCommand::Decoration(draw) => self.decoration(draw),
            SceneCommand::Pattern(_)
            | SceneCommand::DrawShadow { .. }
            | SceneCommand::Sprite(_) => {}
        }
    }

    fn clip(&self) -> Option<Rect> {
        self.clips.last().copied()
    }

    fn push_clip(&mut self, rect: Rect) {
        let rect = self.space.map(rect);
        let next = self.clip().map_or(rect, |outer| {
            outer
                .intersection(rect)
                .unwrap_or_else(|| Rect::new(rect.origin, 0.0, 0.0))
        });
        self.clips.push(next);
    }

    #[allow(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "alpha is clamped to the u8 range before the cast"
    )]
    fn fade(&self, color: Rgba8, opacity: u16) -> Rgba8 {
        let layers: f32 = self.alphas.iter().product();
        let scale = layers * f32::from(opacity.min(10_000)) / 10_000.0;
        if scale >= 1.0 {
            return color;
        }
        let a = (f32::from(color.a) * scale.max(0.0))
            .round()
            .clamp(0.0, 255.0) as u8;
        Rgba8 { a, ..color }
    }

    fn area(&self, rect: Rect) -> Option<Area> {
        let rect = self.space.map(rect);
        let visible = match self.clip() {
            Some(clip) => rect.intersection(clip)?,
            None => rect,
        };
        let cell = self.cell;
        let col = cell_edge(visible.origin.x / cell);
        let row = cell_edge(visible.origin.y / cell);
        let width = cell_edge((visible.origin.x + visible.width) / cell)
            .saturating_sub(col)
            .max(1);
        let height = cell_edge((visible.origin.y + visible.height) / cell)
            .saturating_sub(row)
            .max(1);
        Some((visible, (col, row), (width, height)))
    }

    fn rect(&mut self, draw: &DrawRect) {
        let Some((visible, origin, size)) = self.area(draw.rect) else {
            return;
        };
        if self.paint.marks.caret(draw.node) {
            let middle = (visible.origin.y + visible.height / 2.0) / self.cell;
            self.grid.cursor = Some((origin.0, floor_cell(middle)));
            return;
        }
        if !draw.outline {
            let color = self.fade(draw.color, draw.opacity);
            let ink = self.ink(color, draw.tone);
            if ink.set() {
                let surface = Surface {
                    rect: self.space.map(draw.rect),
                    visible,
                    origin,
                    size,
                    ink,
                    leaf: draw.leaf,
                    halves: draw.leaf && draw.border.is_none(),
                    theme: self.theme,
                };
                decoration::surface(self.grid, &surface, self.cell);
            }
        }
        if let Some(border) = draw.border {
            let border = CellBorder {
                rect: self.space.map(border.rect),
                color: self.fade(border.color, draw.opacity),
                ..border
            };
            let edge = Edge {
                clip: self.clip(),
                theme: self.theme,
                borders: self.paint.caps.borders,
                cell: self.cell,
            };
            decoration::border(self.grid, &border, &edge);
        }
    }

    fn gradient(&mut self, draw: &DrawGradient) {
        let flat = gradient_flat(&draw.gradient.normalized());
        let draw_rect = DrawRect {
            node: draw.node,
            rect: draw.rect,
            color: flat,
            border_radius: draw.border_radius,
            opacity: draw.opacity,
            stroke: draw.stroke,
            tone: draw.tone,
            leaf: draw.leaf,
            border: draw.border,
            outline: draw.outline,
        };
        self.rect(&draw_rect);
    }

    fn text(&mut self, draw: &DrawGlyphRun) {
        let run = &draw.run;
        let Some((visible, ..)) = self.area(run.rect) else {
            return;
        };
        let text = Text {
            rect: self.space.map(run.rect),
            clip: self.clip(),
            content: &run.text,
            flow: run.flow,
            ink: self.ink(self.fade(run.color, run.opacity), run.tone),
            face: self.face(draw),
            link: self.paint.marks.link(run.node),
            theme: self.theme,
            runs: run.style.runs.as_ref().map(|runs| (runs, run.inks())),
            hyperlinks: self.paint.caps.hyperlinks,
        };
        paint_text(self.grid, &text, visible, self.cell);
    }

    fn face(&self, draw: &DrawGlyphRun) -> Face {
        let style = &draw.run.style;
        let linked = self.paint.marks.link(draw.run.node).is_some();
        Face {
            bold: style.font_weight >= 700,
            italic: style.italic,
            dim: style.font_weight < 400,
            underline: if linked && !self.paint.caps.hyperlinks {
                Underline::Straight
            } else {
                Underline::None
            },
        }
    }

    fn decoration(&mut self, draw: &DrawDecoration) {
        let color = self.fade(draw.color, draw.opacity);
        if color.a == 0 {
            return;
        }
        super::marks::decorate(
            self.grid,
            &super::marks::line(
                &DrawDecoration {
                    band: self.space.map(draw.band),
                    ..*draw
                },
                color,
            ),
            self.clip(),
            self.cell,
        );
    }

    fn image(&mut self, draw: &DrawImage) {
        let Some((visible, origin, size)) = self.area(draw.destination) else {
            return;
        };
        let crop = image_crop(
            (self.space.map(draw.destination), draw.crop),
            visible,
            &draw.source,
        );
        self.grid.place(&draw.source, crop, origin, size);
    }
}
