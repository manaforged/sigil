#![allow(
    clippy::unwrap_used,
    clippy::panic,
    clippy::float_cmp,
    clippy::many_single_char_names,
    clippy::too_many_lines,
    clippy::doc_markdown,
    clippy::indexing_slicing,
    clippy::type_complexity
)]

use super::*;
use crate::layout::forest::{AvailableSpace, LayoutForest, MeasureScope};
use crate::layout::resolve::{ResolvedEdges as AlgoEdges, clamp, dim_to_border_box, resolve_lp};
use crate::layout::style_data::{
    AlignContent, AlignItems, Axes as StyleSize, BoxSizing, Display, Edges, FlexDirection,
    FlexWrap, JustifyContent, Length, Position, StyleData,
};

impl FlexSolver<'_> {
    pub(crate) fn boxed(&self, id: LocalId) -> Option<ComputedBox> {
        let forest = self.scope.forest();
        if forest.frame_for(id)? != forest.current_frame() {
            return None;
        }
        forest.boxed(id)
    }
}

const TOL: f32 = 0.01;

const NO_TEXT: crate::layout::FakeMeasure = crate::layout::FakeMeasure::fixed(0.0, 0.0);

fn near(a: f32, b: f32) -> bool {
    (a - b).abs() <= TOL
}

fn run(
    forest: &LayoutForest,
    root: crate::layout::forest::LocalId,
    w: f32,
    h: f32,
) -> FlexSolver<'_> {
    let measurer: &'static crate::layout::FakeMeasure = &NO_TEXT;
    let scope = MeasureScope::new(forest, measurer, AvailableSpace::definite(w, h));
    let solver = FlexSolver::new(scope);
    solver.solve(forest.refer(root).unwrap()).unwrap();
    solver
}

fn fixed(w: f32, h: f32) -> StyleData {
    sized(Length::Px(w), Length::Px(h))
}

fn flex_box(w: f32, h: f32) -> StyleData {
    StyleData {
        display: Display::Flex,
        ..fixed(w, h)
    }
}

fn sized(w: Length, h: Length) -> StyleData {
    StyleData {
        size: StyleSize::new(w, h),
        ..StyleData::default()
    }
}

fn kids(parent: StyleData, children: &[StyleData]) -> Vec<ComputedBox> {
    let mut f = LayoutForest::new();
    let root = f.insert(None, parent);
    let ids: Vec<_> = children
        .iter()
        .map(|style| add_child(&mut f, root, style.clone()))
        .collect();
    let s = run(&f, root, 400.0, 400.0);
    ids.into_iter().map(|id| s.boxed(id).unwrap()).collect()
}

fn add_child(
    f: &mut LayoutForest,
    parent: crate::layout::forest::LocalId,
    style: StyleData,
) -> crate::layout::forest::LocalId {
    let id = f.insert(None, style);
    f.attach(parent, id).unwrap();
    id
}

#[path = "flex_tests/alignment.rs"]
mod alignment;
#[path = "flex_tests/axis.rs"]
mod axis;
#[path = "flex_tests/box_model.rs"]
mod box_model;
#[path = "flex_tests/flow.rs"]
mod flow;
#[path = "flex_tests/resolve.rs"]
mod resolve;
#[path = "flex_tests/wrap.rs"]
mod wrap;
