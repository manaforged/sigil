#![allow(
    clippy::unwrap_used,
    clippy::panic,
    clippy::float_cmp,
    clippy::many_single_char_names,
    clippy::too_many_lines,
    clippy::doc_markdown
)]

use super::*;
use crate::geometry::{LogicalSize, Size as HalSize};
use crate::kernel::TextContent;
use crate::layout::forest::{AvailableSpace, LayoutForest, MeasureScope};
use crate::layout::resolve::{Edges as AlgoEdges, clamp, dim_to_border_box, resolve_lp};
use crate::layout::style_data::{
    AlignContent, AlignItems, AlignSelf, BoxSizing, Dimension, Display, FlexDirection, FlexWrap,
    JustifyContent, LengthPercentage, LengthPercentageAuto, Position, Rect, Size as StyleSize,
    StyleData,
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

struct NullText;
impl crate::layout::TextMeasure for NullText {
    fn measure(&self, _: &TextContent, _: HalSize) -> LogicalSize {
        LogicalSize::default()
    }
}

fn near(a: f32, b: f32) -> bool {
    (a - b).abs() <= TOL
}

fn run(
    forest: &LayoutForest,
    root: crate::layout::forest::LocalId,
    w: f32,
    h: f32,
) -> FlexSolver<'_> {
    let measurer: &'static NullText = &NullText;
    let scope = MeasureScope::new(forest, measurer, AvailableSpace::definite(w, h));
    let solver = FlexSolver::new(scope);
    solver.solve(forest.refer(root).unwrap()).unwrap();
    solver
}

fn flex_parent() -> StyleData {
    StyleData {
        display: Display::Flex,
        ..StyleData::default()
    }
}

fn fixed(w: f32, h: f32) -> StyleData {
    StyleData {
        size: StyleSize::new(Dimension::Px(w), Dimension::Px(h)),
        ..StyleData::default()
    }
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
