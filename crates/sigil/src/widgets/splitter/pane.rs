use crate::kernel::{IntoNode, NodeBuilder, Role, container};
use crate::layout::{LayoutBuilderExt, LayoutStyle};
use crate::style::StyleAtom;
use crate::widgets::orientation::Orientation;

use super::atoms::{cached_height_atom, cached_width_atom, to_i16};
use super::drag::SplitterState;

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
    orientation: Orientation,
    state: SplitterState,
    grow: f32,
) -> NodeBuilder {
    let initial_px = state.size_at(pane_idx);
    let initial_layout = pane_layout(orientation, initial_px, grow);
    let node = container()
        .role(Role::GenericContainer)
        .layout(initial_layout);
    let node = if grow > 0.0 {
        node
    } else {
        node.style_reactive(move || pane_size_atom(orientation, state.size_at(pane_idx)))
    };
    node.child(content)
}

fn pane_layout(orientation: Orientation, size_px: f32, grow: f32) -> LayoutStyle {
    let style = match orientation {
        Orientation::Horizontal => {
            let style = LayoutStyle::flex_column();
            if grow > 0.0 {
                if size_px > 0.0 {
                    style.width(size_px)
                } else {
                    style
                }
            } else {
                style.width(size_px)
            }
        }
        Orientation::Vertical => {
            let style = LayoutStyle::flex_column();
            if grow > 0.0 {
                if size_px > 0.0 {
                    style.height(size_px)
                } else {
                    style
                }
            } else {
                style.height(size_px)
            }
        }
    };
    if grow > 0.0 {
        style.grow(grow).min_size(0.0, 0.0)
    } else {
        style.shrink(0.0)
    }
}

fn pane_size_atom(orientation: Orientation, size_px: f32) -> &'static [StyleAtom] {
    let px = to_i16(size_px);
    match orientation {
        Orientation::Horizontal => cached_width_atom(px),
        Orientation::Vertical => cached_height_atom(px),
    }
}
