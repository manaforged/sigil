use crate::kernel::NodeId;
use crate::scene::builder::SceneBuilder;
use crate::scene::command::{GlyphRun, RunInk};
use crate::style::{TextRuns, Theme};

pub(super) fn inks(runs: Option<&TextRuns>, theme: &Theme) -> Vec<RunInk> {
    runs.map_or_else(Vec::new, |runs| {
        runs.0
            .iter()
            .map(|run| {
                let face = &run.face;
                let background = face
                    .fill
                    .or_else(|| face.background.map(|token| theme.color(token)));
                let under = background.unwrap_or(theme.surface);
                let color = face.ink.or_else(|| {
                    face.color
                        .map(|token| Theme::legible(theme.color(token), under))
                });
                RunInk {
                    color: if face.dim {
                        Some(faint(color.unwrap_or(theme.on_surface)))
                    } else {
                        color
                    },
                    background,
                }
            })
            .collect()
    })
}

const fn faint(color: crate::style::Color) -> crate::style::Color {
    crate::style::Color {
        a: color.a / 5 * 3,
        ..color
    }
}

#[cfg(not(feature = "scene"))]
pub(super) const fn split(_: &mut SceneBuilder, _: NodeId, run: GlyphRun) -> GlyphRun {
    run
}

#[cfg(feature = "scene")]
pub(super) fn split(scene: &mut SceneBuilder, id: NodeId, run: GlyphRun) -> GlyphRun {
    use crate::scene::command::{CellRect, DrawRect};
    use crate::scene::corner_radii::CornerRadii;
    let Some(runs) = run.style.runs.clone() else {
        return run;
    };
    let Some(paragraph) = run.paragraph().cloned() else {
        return run;
    };
    let mut base = Vec::new();
    let mut at = 0;
    let inks = run.inks().to_vec();
    for (text_run, ink) in runs.0.iter().zip(inks) {
        let face = &text_run.face;
        let rects = run_rects(&paragraph, &run, text_run.start, text_run.end);
        if let Some(background) = ink.background {
            let style = |rect: crate::geometry::Rect| {
                let (rect, radius) = if face.square {
                    (cell_edges(rect, &run), 0.0)
                } else {
                    (rect, 3.0)
                };
                DrawRect::new(rect, background)
                    .border_radius(CornerRadii::uniform(radius))
                    .opacity(run.opacity)
            };
            for rect in &rects {
                scene.draw_rect_for_node(id, style(*rect), CellRect::default());
            }
        }
        decorate(scene, &rects, ink.color.unwrap_or(run.color), &run, face);
        let Some(color) = ink.color else {
            continue;
        };
        base.extend(paragraph.rendered_glyphs_in(at, text_run.start));
        at = text_run.end;
        let glyphs = paragraph.rendered_glyphs_in(text_run.start, text_run.end);
        scene.draw_glyph_run_for_node(id, run.clone().with_glyphs(color, glyphs));
    }
    base.extend(paragraph.rendered_glyphs_in(at, usize::MAX));
    let color = run.color;
    run.with_glyphs(color, base)
}

#[cfg(feature = "scene")]
fn decorate(
    scene: &mut SceneBuilder,
    rects: &[crate::geometry::Rect],
    color: crate::style::Color,
    run: &GlyphRun,
    face: &crate::style::TextFace,
) {
    use crate::scene::command::DecorationKind;
    if face.link.is_some() || face.underline {
        line(scene, rects, color, run, DecorationKind::Underline);
    }
    if face.strike {
        line(scene, rects, color, run, DecorationKind::Strikethrough);
    }
}

#[cfg(feature = "scene")]
fn cell_edges(rect: crate::geometry::Rect, run: &GlyphRun) -> crate::geometry::Rect {
    use crate::geometry::{Point, Rect};
    let left = rect.origin.x.floor();
    let top = run.rect.origin.y.floor();
    Rect::new(
        Point::new(left, top),
        (rect.origin.x + rect.width).ceil() - left,
        (run.rect.origin.y + run.rect.height).ceil() - top,
    )
}

#[cfg(feature = "scene")]
fn run_rects(
    paragraph: &crate::text::Paragraph,
    run: &GlyphRun,
    start: usize,
    end: usize,
) -> Vec<crate::geometry::Rect> {
    use crate::geometry::{Point, Rect};
    use crate::hal::{HitPosition, TextAffinity};
    let at = |byte_index| HitPosition {
        byte_index,
        affinity: TextAffinity::Downstream,
    };
    paragraph
        .selection_rects(at(start), at(end))
        .into_iter()
        .map(|rect| {
            Rect::new(
                Point::new(
                    run.rect.origin.x + rect.origin.x,
                    run.rect.origin.y + rect.origin.y,
                ),
                rect.width,
                rect.height,
            )
        })
        .collect()
}

#[cfg(feature = "scene")]
fn line(
    scene: &mut SceneBuilder,
    rects: &[crate::geometry::Rect],
    color: crate::style::Color,
    run: &GlyphRun,
    kind: crate::scene::command::DecorationKind,
) {
    use crate::geometry::{Point, Rect};
    use crate::scene::command::DecorationKind;
    use crate::scene::command::DrawDecoration;
    let thickness = (run.style.font_size * 0.06).max(1.0);
    for rect in rects {
        let y = if kind == DecorationKind::Strikethrough {
            rect.origin.y + (rect.height - thickness) / 2.0
        } else {
            rect.origin.y + rect.height - thickness * 2.0
        };
        let band = Rect::new(Point::new(rect.origin.x, y), rect.width, thickness);
        scene.draw_decoration(
            DrawDecoration::new(kind, band, color)
                .opacity(run.opacity)
                .line_width(thickness),
        );
    }
}
