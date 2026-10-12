#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss,
    clippy::float_cmp,
    clippy::indexing_slicing,
    clippy::panic,
    clippy::unwrap_used
)]

use std::cell::RefCell;
use std::rc::Rc;

use super::*;

use crate::kernel::{Cx, NodeId, container};
use crate::reactive::{Runtime, signal};

use super::Identifiable;

fn bound_visible(cx: &Cx, root: NodeId) -> NodeId {
    let host = cx
        .first_child(root)
        .expect("children_reactive host child of root");
    let content = cx.first_child(host).expect("content child of host");
    cx.first_child(content)
        .expect("visible window child of content")
}

fn first_row_label(cx: &Cx, root: NodeId) -> Option<String> {
    let visible = bound_visible(cx, root);
    let first = cx.first_child(visible)?;
    name_of(cx, first).or_else(|| cx.first_child(first).and_then(|row| name_of(cx, row)))
}

fn name_of(cx: &Cx, id: NodeId) -> Option<String> {
    match &cx.get(id)?.a11y.name {
        crate::kernel::AccessibleName::Static(name) => Some(name.clone()),
        _ => None,
    }
}

type Made = RefCell<Vec<(usize, Signal<usize>)>>;

fn top_row(cx: &Cx, layout: &crate::layout::LayoutEngine, root: NodeId) -> Option<usize> {
    let viewport_top = layout.result(root)?.rect().origin.y;
    let mut best: Option<(f32, usize)> = None;
    cx.visit_preorder(root, |id, node| {
        let crate::kernel::AccessibleName::Static(label) = &node.a11y.name else {
            return;
        };
        let Some(index) = label
            .strip_prefix("row ")
            .and_then(|s| s.parse::<usize>().ok())
        else {
            return;
        };
        let Some(rect) = layout.result(id).map(|r| r.rect()) else {
            return;
        };
        let bottom = rect.origin.y + rect.height - viewport_top;
        if rect.origin.y - viewport_top <= 0.0 && bottom > 0.0 {
            best = Some((rect.origin.y, index));
        }
    });
    best.map(|(_, index)| index)
}

struct ZeroMeasure;

impl crate::layout::TextMeasure for ZeroMeasure {
    fn measure(
        &self,
        _content: &crate::kernel::TextContent,
        _space: crate::geometry::Size,
    ) -> crate::geometry::LogicalSize {
        crate::geometry::LogicalSize::default()
    }
}

fn engine() -> crate::layout::LayoutEngine {
    crate::layout::LayoutEngine::with_text_measure(Box::new(ZeroMeasure))
}

fn mounted<T: 'static, R: RowRender<T>>(list: impl FnOnce() -> VirtualListBuilder<T, R>) -> usize {
    Runtime::scope("sigil-virtual-list-mounted", || {
        let mut cx = Cx::new_root();
        let root = cx.add(list());
        cx.flush_reactive_children();
        cx.child_count(bound_visible(&cx, root))
    })
}

mod binding;
mod rebuild;
mod scroll;
mod viewport;
