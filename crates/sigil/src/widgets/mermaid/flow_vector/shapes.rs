use crate::style::{BORDER_WIDTH, Token};
use crate::widgets::vector::PathBuilder;

use super::super::flow::{Graph, NodeShape};
use super::{Area, CELL_W, CIRCLE_K, CORNER, DOT, canvas, rounded};

pub(super) fn shapes(graph: &Graph, areas: &[Area], size: (f32, f32)) -> PathBuilder {
    let mut node = canvas(size);
    for (entry, area) in graph.nodes.iter().zip(areas) {
        if !entry.dummy && entry.shape != NodeShape::Terminal {
            node = shape(node, entry.shape, *area);
        }
    }
    node.fill_color(Token::Surface)
        .stroke(Token::Border, f32::from(BORDER_WIDTH))
}

pub(super) fn dots(graph: &Graph, areas: &[Area], size: (f32, f32)) -> PathBuilder {
    let mut node = canvas(size);
    for (entry, area) in graph.nodes.iter().zip(areas) {
        if entry.shape == NodeShape::Terminal {
            let end = entry.label.first().is_some_and(|glyph| glyph == "◉");
            node = ellipse(node, area.cx(), area.cy(), DOT, DOT);
            if end {
                node = ellipse(node, area.cx(), area.cy(), DOT * 0.55, DOT * 0.55);
            }
        }
    }
    node.fill_color(Token::OnSurface)
}

fn shape(node: PathBuilder, shape: NodeShape, area: Area) -> PathBuilder {
    match shape {
        NodeShape::Round => rounded(node, area, CORNER * 1.5),
        NodeShape::Stadium | NodeShape::Terminal => rounded(node, area, area.h / 2.0),
        NodeShape::Circle => ellipse(node, area.cx(), area.cy(), area.w / 2.0, area.h / 2.0),
        NodeShape::Diamond | NodeShape::Hexagon => hexagon(node, area),
        NodeShape::Cylinder => cylinder(node, area),
        NodeShape::Subroutine => subroutine(node, area),
        NodeShape::Flag => flag(node, area),
        NodeShape::Box => rounded(node, area, CORNER / 2.0),
    }
}

fn ellipse(node: PathBuilder, cx: f32, cy: f32, rx: f32, ry: f32) -> PathBuilder {
    let (kx, ky) = (rx * CIRCLE_K, ry * CIRCLE_K);
    node.move_to(cx + rx, cy)
        .cubic_to(cx + rx, cy + ky, cx + kx, cy + ry, cx, cy + ry)
        .cubic_to(cx - kx, cy + ry, cx - rx, cy + ky, cx - rx, cy)
        .cubic_to(cx - rx, cy - ky, cx - kx, cy - ry, cx, cy - ry)
        .cubic_to(cx + kx, cy - ry, cx + rx, cy - ky, cx + rx, cy)
        .close()
}

fn hexagon(node: PathBuilder, area: Area) -> PathBuilder {
    let inset = (area.h / 2.0).min(area.w / 4.0);
    node.move_to(area.x + inset, area.y)
        .line_to(area.right() - inset, area.y)
        .line_to(area.right(), area.cy())
        .line_to(area.right() - inset, area.bottom())
        .line_to(area.x + inset, area.bottom())
        .line_to(area.x, area.cy())
        .close()
}

fn cylinder(node: PathBuilder, area: Area) -> PathBuilder {
    let lip = (area.h / 6.0).min(6.0);
    let (x0, x1, top, bottom) = (area.x, area.right(), area.y + lip, area.bottom() - lip);
    let k = CIRCLE_K * area.w / 2.0;
    let belly = 2.0f32.mul_add(lip, top);
    node.move_to(x0, top)
        .cubic_to(x0, top - lip, area.cx() - k, area.y, area.cx(), area.y)
        .cubic_to(area.cx() + k, area.y, x1, top - lip, x1, top)
        .line_to(x1, bottom)
        .cubic_to(
            x1,
            bottom + lip,
            area.cx() + k,
            area.bottom(),
            area.cx(),
            area.bottom(),
        )
        .cubic_to(area.cx() - k, area.bottom(), x0, bottom + lip, x0, bottom)
        .close()
        .move_to(x0, top)
        .cubic_to(x0, top + lip, area.cx() - k, belly, area.cx(), belly)
        .cubic_to(area.cx() + k, belly, x1, top + lip, x1, top)
}

fn subroutine(node: PathBuilder, area: Area) -> PathBuilder {
    let side = CELL_W;
    rounded(node, area, CORNER / 2.0)
        .move_to(area.x + side, area.y)
        .line_to(area.x + side, area.bottom())
        .move_to(area.right() - side, area.y)
        .line_to(area.right() - side, area.bottom())
}

fn flag(node: PathBuilder, area: Area) -> PathBuilder {
    node.move_to(area.x, area.y)
        .line_to(area.right(), area.y)
        .line_to(area.right(), area.bottom())
        .line_to(area.x, area.bottom())
        .line_to(area.x + CELL_W, area.cy())
        .close()
}
