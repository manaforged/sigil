use sigil::hal::Rect;
use sigil::prelude::*;

pub(crate) fn top_layer(
    cx: &Cx,
    layout: &LayoutEngine,
    root: NodeId,
    viewport: Rect,
) -> Option<Rect> {
    let mut out = None;
    collect(cx, layout, root, viewport, &mut out);
    out
}

fn collect(cx: &Cx, layout: &LayoutEngine, id: NodeId, viewport: Rect, out: &mut Option<Rect>) {
    let Some(node) = cx.get(id) else {
        return;
    };
    if node.get_extension::<TopLayer>().is_some() {
        visible(cx, layout, id, viewport, out);
        return;
    }
    for child in node.children() {
        collect(cx, layout, *child, viewport, out);
    }
}

fn visible(cx: &Cx, layout: &LayoutEngine, id: NodeId, viewport: Rect, out: &mut Option<Rect>) {
    let Some(rect) = layout.result(id).map(|result| result.rect()) else {
        return;
    };
    if rect.width <= 0.0 || rect.height <= 0.0 {
        return;
    }
    let covers = rect.width >= viewport.width && rect.height >= viewport.height;
    let children = cx.get(id).map_or(&[][..], |node| node.children());
    if covers {
        for child in children {
            visible(cx, layout, *child, viewport, out);
        }
        return;
    }
    *out = Some(out.map_or(rect, |known| known.union(rect)));
}
