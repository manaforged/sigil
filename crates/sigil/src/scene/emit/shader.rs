use crate::geometry::{Point, Rect};
use crate::kernel::Node;
use crate::scene::builder::SceneBuilder;
use crate::scene::corner_radii::CornerRadii;
use crate::scene::shader::Shading;
use crate::scene::style_adapter::corner_radii_for;
use crate::style::shader_paint::lookup;
use crate::style::{RenderStyle, Theme};

pub(super) fn push_fill(
    scene: &mut SceneBuilder,
    node: &Node,
    rect: Rect,
    style: &RenderStyle,
    theme: &Theme,
) {
    let Some(shader) = style.paint.shader.and_then(|id| lookup(node, id)) else {
        return;
    };
    let [top, right, bottom, left] = super::borders::border_edges(style).map(|edge| edge.max(0.0));
    let inner = Rect::new(
        Point::new(rect.origin.x + left, rect.origin.y + top),
        (rect.width - left - right).max(0.0),
        (rect.height - top - bottom).max(0.0),
    );
    let inset = top.max(right).max(bottom).max(left);
    let radii = CornerRadii(
        corner_radii_for(style.border_radius)
            .0
            .map(|radius| (radius - inset).max(0.0)),
    );
    scene.push_shader(shader.draw(inner, radii, theme).opacity(style.opacity));
}

pub(super) fn layer(node: &Node, style: &RenderStyle, theme: &Theme) -> Option<Shading> {
    let shader = style.filter.shader.and_then(|id| lookup(node, id))?;
    Some(shader.shading(theme))
}
