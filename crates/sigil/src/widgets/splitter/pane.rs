use crate::kernel::{IntoNode, NodeBuilder, Role, container};
use crate::layout::{LayoutBuilderExt, LayoutStyle};
use crate::style::{Dimension, Px, StyleAtom};
use crate::widgets::orientation::Orientation;
use crate::widgets::scroll::geometry::{Cache, atoms};
use crate::widgets::scroll::to_i16;

use super::drag::SplitterState;

const ATOM_CACHE_CAP: usize = 512;

pub(super) fn shares_free_space(initial_size: f32, grow: f32) -> f32 {
    if initial_size <= 0.0 && grow <= 0.0 {
        1.0
    } else {
        grow
    }
}

pub(super) fn build_pane(
    content: Box<dyn IntoNode>,
    pane_idx: usize,
    state: SplitterState,
    grow: f32,
) -> NodeBuilder {
    let orientation = state.orientation();
    let node = container().role(Role::GenericContainer).layout(pane_layout(
        orientation,
        state.size_at(pane_idx),
        grow,
    ));
    let node = if grow > 0.0 {
        node
    } else {
        node.style_reactive(move || pane_size_atom(orientation, state.size_at(pane_idx)))
    };
    node.child(content)
}

fn pane_layout(orientation: Orientation, size_px: f32, grow: f32) -> LayoutStyle {
    let style = LayoutStyle::flex_column();
    let style = match orientation {
        _ if grow > 0.0 && size_px <= 0.0 => style,
        Orientation::Horizontal => style.width(size_px),
        Orientation::Vertical => style.height(size_px),
    };
    if grow > 0.0 {
        style.grow(grow).min_size(0.0, 0.0)
    } else {
        style.shrink(0.0)
    }
}

pub(in crate::widgets) fn cached_width_atom(px: i16) -> &'static [StyleAtom] {
    static CACHE: Cache = Cache::new();
    atoms(&CACHE, ATOM_CACHE_CAP, (px, 0), || {
        vec![StyleAtom::Width(Dimension::Px(Px(px)))]
    })
}

fn pane_size_atom(orientation: Orientation, size_px: f32) -> &'static [StyleAtom] {
    static HEIGHTS: Cache = Cache::new();
    let px = to_i16(size_px);
    match orientation {
        Orientation::Horizontal => cached_width_atom(px),
        Orientation::Vertical => atoms(&HEIGHTS, ATOM_CACHE_CAP, (px, 0), || {
            vec![StyleAtom::Height(Dimension::Px(Px(px)))]
        }),
    }
}
