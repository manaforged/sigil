use resvg::usvg::{Pattern, Transform};

use crate::geometry::{Point, Rect};
use crate::scene::SceneBuilder;
use crate::scene::command::{Clip, DrawPath, LayerEffect, LayerScope};

use super::budget::MAX_COMMANDS;
use super::fallback::Fallback;
use super::group::{Scope, layer};
use super::lower::Walk;

const MAX_TILES: f32 = 4096.0;
const SLACK: f32 = 1e-3;

/// Tile index ranges of a pattern over an area.
struct Tiles {
    columns: (i32, i32),
    rows: (i32, i32),
}

impl Walk {
    /// Fills the area of `shape` with copies of the pattern tile, or fails when the copies exceed the budget.
    pub(super) fn pattern(
        &mut self,
        scene: &mut SceneBuilder,
        (pattern, opacity): (&Pattern, f32),
        shape: DrawPath,
        t: Transform,
    ) -> Result<(), Fallback> {
        if self.exhausted {
            return Err(Fallback::Budget);
        }
        let space = t.pre_concat(pattern.transform());
        let tiles = tiles(pattern, shape.bounds, space).ok_or(Fallback::PatternTiles)?;
        let mut body = SceneBuilder::new();
        let mut scope = Scope::default();
        let area = shape.bounds;
        scope.clip(&mut body, Clip::Path(shape));
        if opacity < 1.0 {
            scope.layer(
                &mut body,
                LayerScope {
                    opacity,
                    ..layer(area, LayerEffect::Identity)
                },
            );
        }
        self.copies(&mut body, pattern, (&tiles, space))?;
        scope.close(&mut body);
        scene.extend_commands(body.finish().commands);
        Ok(())
    }

    fn copies(
        &mut self,
        body: &mut SceneBuilder,
        pattern: &Pattern,
        (tiles, space): (&Tiles, Transform),
    ) -> Result<(), Fallback> {
        let start = self.spent;
        let inside = within(pattern);
        let rect = pattern.rect();
        for column in tiles.columns.0..=tiles.columns.1 {
            for row in tiles.rows.0..=tiles.rows.1 {
                #[allow(
                    clippy::cast_precision_loss,
                    reason = "tile indices stay below MAX_TILES"
                )]
                let (x, y) = (column as f32, row as f32);
                let tile = space.pre_translate(
                    x.mul_add(rect.width(), rect.x()),
                    y.mul_add(rect.height(), rect.y()),
                );
                self.tile(body, pattern, tile, inside);
                self.spent = start + body.command_len();
                if self.exhausted || self.spent > MAX_COMMANDS {
                    self.exhausted = true;
                    self.spent = start;
                    return Err(Fallback::Budget);
                }
            }
        }
        Ok(())
    }

    fn tile(&mut self, scene: &mut SceneBuilder, pattern: &Pattern, tile: Transform, inside: bool) {
        if inside {
            self.children(scene, pattern.root(), tile);
            return;
        }
        let rect = pattern.rect();
        let mut corners = [
            (0.0, 0.0),
            (rect.width(), 0.0),
            (rect.width(), rect.height()),
            (0.0, rect.height()),
        ]
        .map(|(x, y)| resvg::usvg::tiny_skia_path::Point::from_xy(x, y));
        tile.map_points(&mut corners);
        let outline = corners.map(|point| Point::new(point.x, point.y));
        scene.with_clip(Clip::polygon(&outline), |scene| {
            self.children(scene, pattern.root(), tile);
        });
    }
}

fn within(pattern: &Pattern) -> bool {
    let rect = pattern.rect();
    let content = pattern.root().layer_bounding_box();
    content.left() >= -SLACK
        && content.top() >= -SLACK
        && content.right() <= rect.width() + SLACK
        && content.bottom() <= rect.height() + SLACK
}

#[allow(
    clippy::cast_possible_truncation,
    reason = "the tile count is checked against MAX_TILES before the casts"
)]
fn tiles(pattern: &Pattern, area: Rect, space: Transform) -> Option<Tiles> {
    let inverse = space.invert()?;
    let (left, top) = (area.origin.x, area.origin.y);
    let (right, bottom) = (left + area.width, top + area.height);
    let mut corners = [(left, top), (right, top), (right, bottom), (left, bottom)]
        .map(|(x, y)| resvg::usvg::tiny_skia_path::Point::from_xy(x, y));
    inverse.map_points(&mut corners);
    let span = |pick: fn(&resvg::usvg::tiny_skia_path::Point) -> f32| {
        corners
            .iter()
            .map(pick)
            .fold((f32::INFINITY, f32::NEG_INFINITY), |(low, high), value| {
                (low.min(value), high.max(value))
            })
    };
    let rect = pattern.rect();
    let index = |(low, high): (f32, f32), start: f32, step: f32| {
        (
            ((low - start) / step).floor(),
            ((high - start) / step).ceil() - 1.0,
        )
    };
    let columns = index(span(|p| p.x), rect.x(), rect.width());
    let rows = index(span(|p| p.y), rect.y(), rect.height());
    let count = (columns.1 - columns.0 + 1.0) * (rows.1 - rows.0 + 1.0);
    if !count.is_finite() || count > MAX_TILES {
        return None;
    }
    Some(Tiles {
        columns: (columns.0 as i32, columns.1 as i32),
        rows: (rows.0 as i32, rows.1 as i32),
    })
}
