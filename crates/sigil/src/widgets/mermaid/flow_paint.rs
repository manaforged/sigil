use super::canvas::{Canvas, LEFT, UP};
use super::flow::{Dir, Edge, Graph, Node, Rect, Shape};
use super::flow_route::{Pen, between, entry, exit, route, span};

pub(super) fn paint(graph: &Graph, rects: &[Rect]) -> Vec<String> {
    let (cols, rows, extent, lanes) = sizes(graph, rects);
    let mut canvas = Canvas::new(cols, rows);
    for (node, rect) in graph.nodes.iter().zip(rects) {
        if node.dummy {
            through(&mut canvas, *rect, graph.dir, node.dashed);
        } else {
            shape(&mut canvas, node, *rect);
        }
    }
    for (edge, lane) in graph.edges.iter().zip(&lanes) {
        let (Some(from), Some(to)) = (rects.get(edge.from), rects.get(edge.to)) else {
            continue;
        };
        match lane {
            Some(lane) => {
                let at = extent
                    .saturating_add(1)
                    .saturating_add(lane.saturating_mul(2));
                around(&mut canvas, edge, (*from, *to), graph.dir, at);
            }
            None => route(&mut canvas, edge, *from, *to, graph.dir),
        }
    }
    canvas.lines()
}

pub(super) fn sizes(graph: &Graph, rects: &[Rect]) -> (usize, usize, usize, Vec<Option<usize>>) {
    let margin = graph
        .edges
        .iter()
        .map(|edge| edge.label.chars().count())
        .max()
        .unwrap_or(0)
        .saturating_add(2);
    let cols = rects
        .iter()
        .map(|rect| rect.x.saturating_add(rect.w))
        .max()
        .unwrap_or(0)
        .saturating_add(margin);
    let rows = rects
        .iter()
        .map(|rect| rect.y.saturating_add(rect.h))
        .max()
        .unwrap_or(0)
        .saturating_add(1);
    let lanes = lanes(graph, rects);
    let extent = rects
        .iter()
        .map(|rect| {
            if graph.dir.vertical() {
                rect.x.saturating_add(rect.w)
            } else {
                rect.y.saturating_add(rect.h)
            }
        })
        .max()
        .unwrap_or(0);
    let back = match lanes.iter().flatten().count() {
        0 => 0,
        count => count.saturating_mul(2).saturating_add(2),
    };
    let (cols, rows) = if graph.dir.vertical() {
        (cols.saturating_add(back), rows)
    } else {
        (cols, rows.saturating_add(back))
    };
    (cols, rows, extent, lanes)
}

pub(super) fn lanes(graph: &Graph, rects: &[Rect]) -> Vec<Option<usize>> {
    let mut next = 0_usize;
    let pen = Pen {
        dir: graph.dir,
        dashed: false,
    };
    graph
        .edges
        .iter()
        .map(|edge| {
            let (Some(from), Some(to)) = (rects.get(edge.from), rects.get(edge.to)) else {
                return None;
            };
            let (pp, _) = pen.split(exit(*from, graph.dir));
            let (qp, _) = pen.split(entry(*to, graph.dir));
            if (qp > pp) == graph.dir.forward() && qp != pp {
                return None;
            }
            let lane = next;
            next = next.saturating_add(1);
            Some(lane)
        })
        .collect()
}

pub(super) fn around(canvas: &mut Canvas, edge: &Edge, ends: (Rect, Rect), dir: Dir, lane: usize) {
    let (from, to) = ends;
    let pen = Pen {
        dir,
        dashed: edge.dashed,
    };
    let (sp, ss) = pen.split(side(from, dir));
    let (tp, ts) = pen.split(side(to, dir));
    let toward = if dir.vertical() { LEFT } else { UP };
    let (first, second) = if tp < sp {
        (pen.back(), pen.ahead())
    } else {
        (pen.ahead(), pen.back())
    };
    if let Some((low, high)) = span(ss, lane) {
        pen.secondary(canvas, sp, low, high);
    }
    let (x1, y1) = pen.point(sp, lane);
    canvas.corner(x1, y1, toward | first);
    if let Some((low, high)) = between(sp, tp) {
        pen.primary(canvas, lane, low, high);
    }
    let (x2, y2) = pen.point(tp, lane);
    canvas.corner(x2, y2, toward | second);
    if let Some((low, high)) = between(ts, lane) {
        pen.secondary(canvas, tp, low, high);
    }
    let head = match (edge.arrow, dir.vertical()) {
        (true, true) => '◀',
        (true, false) => '▲',
        (false, _) => pen.glyphs().1,
    };
    let (x3, y3) = pen.point(tp, ts);
    canvas.set(x3, y3, head);
    if !edge.label.is_empty() {
        let middle = sp.min(tp).saturating_add(sp.abs_diff(tp) / 2);
        let (x4, y4) = pen.point(middle, lane.saturating_add(1));
        canvas.text(x4, y4, &edge.label);
    }
}

pub(super) const fn side(rect: Rect, dir: Dir) -> (usize, usize) {
    if dir.vertical() {
        (rect.x + rect.w, rect.y + rect.h / 2)
    } else {
        (rect.x + rect.w / 2, rect.y + rect.h)
    }
}

pub(super) fn through(canvas: &mut Canvas, rect: Rect, dir: Dir, dashed: bool) {
    let pen = Pen { dir, dashed };
    let (glyph, _) = pen.glyphs();
    if dir.vertical() {
        canvas.vline(
            rect.x,
            rect.y,
            rect.y.saturating_add(rect.h).saturating_sub(1),
            glyph,
        );
    } else {
        canvas.hline(
            rect.x,
            rect.x.saturating_add(rect.w).saturating_sub(1),
            rect.y,
            glyph,
        );
    }
}

pub(super) fn shape(canvas: &mut Canvas, node: &Node, rect: Rect) {
    let round = matches!(
        node.shape,
        Shape::Round | Shape::Stadium | Shape::Cylinder | Shape::Circle | Shape::Terminal
    );
    canvas.frame(rect.x, rect.y, (rect.w, rect.h), round);
    if matches!(node.shape, Shape::Diamond | Shape::Hexagon) {
        let right = rect.x.saturating_add(rect.w).saturating_sub(1);
        let bottom = rect.y.saturating_add(rect.h).saturating_sub(1);
        canvas.set(rect.x, rect.y, '╱');
        canvas.set(right, rect.y, '╲');
        canvas.set(rect.x, bottom, '╲');
        canvas.set(right, bottom, '╱');
    }
    let inner = rect.w.saturating_sub(2);
    for (index, line) in node.label.iter().enumerate() {
        let text = match node.shape {
            Shape::Circle | Shape::Stadium => format!("({line})"),
            Shape::Subroutine => format!("[{line}]"),
            Shape::Flag => format!("{line} ▷"),
            _ => line.clone(),
        };
        let width = text.chars().count();
        let x = rect
            .x
            .saturating_add(1)
            .saturating_add(inner.saturating_sub(width) / 2);
        canvas.text(x, rect.y.saturating_add(1).saturating_add(index), &text);
    }
}
