use crate::geometry::{Point, Rect};
use crate::kernel::ObjectFit;
use crate::scene::command::effects::transform_rect;
use crate::scene::command::image_fit::fit;
use crate::scene::geometry::IDENTITY;
use crate::scene::state::SceneState;
use crate::scene::{
    CellBorder, CommandScene, DrawDecoration, DrawGradient, DrawImage, DrawRect, GlyphRun,
    Gradient, LayerEffect, SceneCommand,
};
use crate::style::{Color, Theme};
use crate::terminal::caps::{Capabilities, Fills};

use super::decoration::{self, BorderEdge};
use super::fill::{self, FillSurface};
use super::marks::Marks;
use super::paint_clip::{Text, image_crop, paint_text};
use super::{Face, Grid, Ink, Underline, floor_cell, snapped_span};

mod image;
mod path;

pub(crate) struct CellPaint<'a> {
    pub theme: &'a Theme,
    pub cell: f32,
    pub caps: Capabilities,
    pub marks: &'a Marks,
}

#[cfg(test)]
#[must_use]
pub(crate) fn raster(scene: &CommandScene, theme: &Theme, cols: u16, rows: u16, cell: f32) -> Grid {
    let mut grid = Grid::new(cols, rows);
    raster_into(&mut grid, scene, theme, cell);
    grid
}

#[cfg(feature = "kittest")]
pub(crate) fn raster_into(grid: &mut Grid, scene: &CommandScene, theme: &Theme, cell: f32) {
    let marks = Marks::default();
    let paint = CellPaint {
        theme,
        cell,
        caps: Capabilities::default(),
        marks: &marks,
    };
    raster_with(grid, scene, &paint);
}

#[cfg(any(test, feature = "kittest"))]
pub(crate) fn raster_with(grid: &mut Grid, scene: &CommandScene, paint: &CellPaint<'_>) {
    grid.reset();
    raster_onto(grid, scene, paint);
}

pub(crate) fn raster_onto(grid: &mut Grid, scene: &CommandScene, paint: &CellPaint<'_>) {
    walk(grid, scene, paint, None);
    grid.map_glyphs(paint.caps.glyphs);
}

pub(super) fn walk(
    grid: &mut Grid,
    scene: &CommandScene,
    paint: &CellPaint<'_>,
    only: Option<&[Rect]>,
) {
    let mut walk = CellWalk {
        grid,
        paint,
        only,
        hull: only.and_then(hull),
        cell: if paint.cell <= 0.0 { 1.0 } else { paint.cell },
        state: SceneState::default(),
    };
    for command in &scene.commands {
        walk.command(command);
    }
}

fn hull(regions: &[Rect]) -> Option<[f32; 4]> {
    regions.iter().fold(None, |acc, region| {
        let edges = [
            region.origin.x,
            region.origin.y,
            region.origin.x + region.width,
            region.origin.y + region.height,
        ];
        Some(acc.map_or(edges, |[left, top, right, bottom]: [f32; 4]| {
            [
                left.min(edges[0]),
                top.min(edges[1]),
                right.max(edges[2]),
                bottom.max(edges[3]),
            ]
        }))
    })
}

fn gradient_flat(gradient: &Gradient) -> Color {
    crate::scene::command::fills::color_at(&gradient.stops, 0.5)
}

#[allow(
    clippy::cast_possible_truncation,
    reason = "cell coordinates fit an i32 and the cast saturates beyond it"
)]
fn signed_span(start: f32, extent: f32) -> (i32, u16) {
    let first = (start + 0.5).floor() as i32;
    let end = (start + extent + 0.5).floor() as i32;
    let width = end.saturating_sub(first).clamp(1, 256);
    (first, u16::try_from(width).unwrap_or(256))
}

fn span(start: f32, extent: f32) -> (usize, usize) {
    let (first, len) = snapped_span(start, extent);
    if len > 0 {
        (first, len)
    } else {
        (floor_cell(extent.mul_add(0.5, start)), 1)
    }
}

fn reach(rect: Rect, border: Option<CellBorder>) -> Rect {
    border.map_or(rect, |border| rect.union(border.rect))
}

struct CellWalk<'a> {
    grid: &'a mut Grid,
    paint: &'a CellPaint<'a>,
    only: Option<&'a [Rect]>,
    hull: Option<[f32; 4]>,
    cell: f32,
    state: SceneState,
}

type SceneArea = (Rect, (usize, usize), (usize, usize));

impl CellWalk<'_> {
    fn command(&mut self, command: &SceneCommand) {
        if self.state.apply(command) || self.layer(command) || self.state.clipped_out() {
            return;
        }
        match command {
            SceneCommand::Rect(draw) => {
                let caret = self.paint.marks.caret(draw.node);
                if caret || self.wanted(reach(draw.rect, draw.border)) {
                    self.rect(draw);
                }
            }
            SceneCommand::Gradient(draw) => {
                if self.wanted(reach(draw.rect, draw.border)) {
                    self.gradient(draw);
                }
            }
            SceneCommand::GlyphRun(draw) => {
                if self.wanted(draw.rect) {
                    self.text(draw);
                }
            }
            SceneCommand::Image(draw) => self.image(draw),
            SceneCommand::Path(draw) => self.path(draw),
            SceneCommand::Decoration(draw) => {
                if self.wanted(draw.band) {
                    self.decoration(draw);
                }
            }
            SceneCommand::Pattern(_)
            | SceneCommand::DrawShadow { .. }
            | SceneCommand::PushClip(_)
            | SceneCommand::PopClip
            | SceneCommand::PushLayer(_)
            | SceneCommand::PopLayer => {}
            #[cfg(feature = "kittest")]
            SceneCommand::Sprite(_) => {}
            SceneCommand::PushTransform(_) | SceneCommand::PopTransform => {}
            #[cfg(host)]
            SceneCommand::SaveLayer { .. } => {}
            #[cfg(terminal_host)]
            SceneCommand::RestoreLayer => {}
        }
    }

    fn layer(&mut self, command: &SceneCommand) -> bool {
        match command {
            SceneCommand::PushLayer(scope) => {
                self.state.push_clip(match scope.effect {
                    LayerEffect::Projected {
                        transform,
                        hide_backface,
                    } if transform.visible(hide_backface).is_none() => {
                        Rect::new(scope.clip.origin, 0.0, 0.0)
                    }
                    _ => scope.clip,
                });
                self.state.apply(&SceneCommand::SaveLayer {
                    opacity: scope.opacity,
                });
                self.state.push_transform(match scope.effect {
                    LayerEffect::Transform(transform) => transform,
                    LayerEffect::Projected { transform, .. } => transform.affine_fit(scope.clip),
                    LayerEffect::Identity | LayerEffect::Mask(_) | LayerEffect::Filter(_) => {
                        IDENTITY
                    }
                });
            }
            SceneCommand::PopLayer => {
                self.state.pop_clip();
                self.state.apply(&SceneCommand::RestoreLayer);
                self.state.pop_transform();
            }
            _ => return false,
        }
        true
    }

    fn wanted(&self, rect: Rect) -> bool {
        let Some(only) = self.only else {
            return true;
        };
        let Some([left, top, right, bottom]) = self.hull else {
            return false;
        };
        let rect = self.map(rect);
        rect.origin.x < right
            && left < rect.origin.x + rect.width
            && rect.origin.y < bottom
            && top < rect.origin.y + rect.height
            && only.iter().any(|region| region.intersects(rect))
    }

    fn clip(&self) -> Option<Rect> {
        self.state.current_clip()
    }

    fn map(&self, rect: Rect) -> Rect {
        transform_rect(self.state.current_transform(), rect)
    }

    #[allow(
        clippy::cast_possible_truncation,
        clippy::cast_sign_loss,
        reason = "alpha is clamped to the u8 range before the cast"
    )]
    fn fade(&self, color: Color, opacity: u16) -> Color {
        let layers = self.state.current_opacity();
        let scale = layers * f32::from(opacity.min(10_000)) / 10_000.0;
        if scale >= 1.0 {
            return color;
        }
        let a = (f32::from(color.a) * scale.max(0.0))
            .round()
            .clamp(0.0, 255.0) as u8;
        Color { a, ..color }
    }

    fn area(&self, rect: Rect) -> Option<SceneArea> {
        let rect = self.map(rect);
        let visible = match self.clip() {
            Some(clip) => rect.intersection(clip)?,
            None => rect,
        };
        let cell = self.cell;
        let (col, width) = span(visible.origin.x / cell, visible.width / cell);
        let (row, height) = span(visible.origin.y / cell, visible.height / cell);
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
        if !draw.outline && visible.width > 0.0 && visible.height > 0.0 {
            let color = self.fade(draw.color, draw.opacity);
            let ink = Ink::pick(color, draw.tone);
            if ink.set() {
                let surface = FillSurface {
                    rect: self.map(draw.rect),
                    visible,
                    origin,
                    size,
                    ink,
                    halves: draw.leaf
                        && draw.border.is_none()
                        && self.paint.caps.fills == Fills::Partial,
                    theme: self.paint.theme,
                };
                fill::surface(self.grid, &surface, self.cell);
            }
        }
        if let Some(border) = draw.border {
            let border = CellBorder {
                rect: self.map(border.rect),
                color: self.fade(border.color, draw.opacity),
                ..border
            };
            let edge = BorderEdge {
                clip: self.clip(),
                theme: self.paint.theme,
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

    fn text(&mut self, draw: &GlyphRun) {
        let run = draw;
        let Some((visible, ..)) = self.area(run.rect) else {
            return;
        };
        let text = Text {
            rect: self.map(run.rect),
            clip: self.clip(),
            content: run.text(),
            flow: run.flow(),
            ink: Ink::pick(self.fade(run.color(), run.opacity_bp()), run.tone()),
            face: self.face(draw),
            link: self.paint.marks.link(run.node),
            theme: self.paint.theme,
            runs: run.style().runs.as_ref().map(|runs| (runs, run.inks())),
            hyperlinks: self.paint.caps.hyperlinks,
        };
        paint_text(self.grid, &text, visible, self.cell);
    }

    fn face(&self, draw: &GlyphRun) -> Face {
        let style = draw.style();
        let linked = self.paint.marks.link(draw.node).is_some();
        Face {
            bold: style.font_weight >= 700,
            italic: style.italic,
            dim: style.font_weight < 400,
            strike: false,
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
        let line = DrawDecoration {
            band: self.map(draw.band),
            color,
            ..*draw
        };
        super::marks::decorate(self.grid, &line, self.clip(), self.cell);
    }
}
