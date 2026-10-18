use crate::kernel::{AccessibleName, Cx, FrameDelta, NodeId};
use crate::scene::{CommandScene, DecorationKind, DrawDecoration, DrawRect, SceneCommand};
use crate::style::Rgba8;
use smol_str::SmolStr;

use super::{Grid, Ink, Underline, cell_edge, floor_cell};

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Marks {
    links: Vec<(NodeId, SmolStr)>,
    caret: Option<NodeId>,
    focus: Option<NodeId>,
}

impl Marks {
    #[must_use]
    pub(crate) fn stale(&self, cx: &Cx, focus: Option<NodeId>, delta: &FrameDelta) -> bool {
        let touched = |id: &NodeId| delta.paint.contains(id) || delta.layout.contains(id);
        delta.structure
            || focus != self.focus
            || self.links.iter().any(|(id, _)| touched(id))
            || delta
                .paint
                .iter()
                .chain(&delta.layout)
                .any(|id| link(cx, *id).is_some())
    }

    #[must_use]
    pub fn collect(cx: &Cx, focus: Option<NodeId>, scene: &CommandScene, cell: f32) -> Self {
        let mut marks = Self {
            focus,
            ..Self::default()
        };
        for command in &scene.commands {
            match command {
                SceneCommand::GlyphRun(draw) => {
                    if let Some(id) = draw.run.node
                        && let Some(url) = link(cx, id)
                    {
                        marks.links.push((id, url));
                    }
                }
                SceneCommand::Rect(draw) if caret_shape(draw, cell) => {
                    if let (Some(id), Some(focus)) = (draw.node, focus)
                        && within(cx, id, focus)
                    {
                        marks.caret = Some(id);
                    }
                }
                _ => {}
            }
        }
        marks
    }

    pub(super) fn link(&self, node: Option<NodeId>) -> Option<&SmolStr> {
        let node = node?;
        self.links
            .iter()
            .find_map(|(id, url)| (*id == node).then_some(url))
    }

    pub(crate) fn same_links(&self, other: &Self) -> bool {
        self.links == other.links
    }

    pub(super) fn caret(&self, node: Option<NodeId>) -> bool {
        node.is_some() && node == self.caret
    }
}

fn link(cx: &Cx, id: NodeId) -> Option<SmolStr> {
    let text = match cx.get(id)?.a11y.description.as_ref()? {
        AccessibleName::Static(text) => text.clone(),
        AccessibleName::Derived(signal) => signal.peek(),
        AccessibleName::None => return None,
    };
    (text.contains("://") || text.starts_with("mailto:")).then(|| SmolStr::from(text))
}

fn caret_shape(draw: &DrawRect, cell: f32) -> bool {
    !draw.outline
        && draw.border.is_none()
        && draw.rect.width > 0.0
        && draw.rect.width <= cell / 2.0
        && draw.rect.height >= cell / 2.0
        && draw.rect.height <= cell * 2.0
}

fn within(cx: &Cx, mut id: NodeId, focus: NodeId) -> bool {
    for _ in 0..64 {
        if id == focus {
            return true;
        }
        let Some(parent) = cx.parent(id) else {
            return false;
        };
        id = parent;
    }
    false
}

pub(super) fn decorate(
    grid: &mut Grid,
    line: &DrawDecoration,
    clip: Option<crate::geometry::Rect>,
    size: f32,
) {
    let band = line.band;
    let middle = band.origin.y + band.height / 2.0;
    if let Some(clip) = clip
        && (middle < clip.origin.y || middle >= clip.origin.y + clip.height)
    {
        return;
    }
    let (left, right) = clip.map_or((band.origin.x, band.origin.x + band.width), |clip| {
        (
            band.origin.x.max(clip.origin.x),
            (band.origin.x + band.width).min(clip.origin.x + clip.width),
        )
    });
    let row = floor_cell(middle / size);
    let ink = if line.color.a > 0 {
        Ink::Exact(Rgba8 {
            a: 255,
            ..line.color
        })
    } else {
        Ink::Bare
    };
    for col in cell_edge(left / size)..cell_edge(right / size) {
        let Some(cell) = grid.cell_mut(col, row) else {
            continue;
        };
        match line.kind {
            DecorationKind::Strikethrough => cell.lines.strike = true,
            DecorationKind::Underline => cell.lines.underline = Underline::Straight,
            DecorationKind::WavyUnderline => cell.lines.underline = Underline::Curly,
        }
        if !matches!(line.kind, DecorationKind::Strikethrough) {
            cell.underline_ink = ink;
        }
    }
}
