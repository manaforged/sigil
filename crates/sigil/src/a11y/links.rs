use accesskit::{Node as AkNode, Role};

use super::mapping::BoundsLookup;
use super::text::{RunNodes, child_id};
use crate::events::links::{link_ranges, link_text, range_rects};
use crate::kernel::{Cx, NodeId};

pub(super) fn has_links(cx: &Cx, id: NodeId, bounds: BoundsLookup<'_>) -> bool {
    let Some(text) = link_text(cx, id) else {
        return false;
    };
    bounds(id).is_some()
        && link_ranges(&text)
            .into_iter()
            .any(|(start, end, _)| !range_rects(&text, start, end).is_empty())
}

pub(super) fn export(
    cx: &Cx,
    id: NodeId,
    bounds: BoundsLookup<'_>,
    factor: f64,
    ak: &mut AkNode,
) -> Option<RunNodes> {
    let text = link_text(cx, id)?;
    let origin = bounds(id)?.origin;
    let mut out = Vec::new();
    for (index, (start, end, href)) in link_ranges(&text).into_iter().enumerate() {
        let rects = range_rects(&text, start, end);
        let Some(first) = rects.first().copied() else {
            continue;
        };
        let area = rects
            .iter()
            .skip(1)
            .fold(first, |all, rect| all.union(*rect));
        let mut node = AkNode::new(Role::Link);
        node.set_label(text.text.get(start..end).unwrap_or_default());
        node.set_url(href.to_string());
        node.set_bounds(accesskit::Rect {
            x0: f64::from(origin.x + area.origin.x) * factor,
            y0: f64::from(origin.y + area.origin.y) * factor,
            x1: f64::from(origin.x + area.origin.x + area.width) * factor,
            y1: f64::from(origin.y + area.origin.y + area.height) * factor,
        });
        let link = child_id(id, index, 2);
        ak.push_child(link);
        out.push((link, node));
    }
    (!out.is_empty()).then_some(out)
}
