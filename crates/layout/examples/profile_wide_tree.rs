#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]
#![allow(
    missing_docs,
    clippy::missing_docs_in_private_items,
    clippy::unwrap_used,
    clippy::cast_precision_loss
)]

use sigil_hal::{LogicalSize, Size};
use sigil_kernel::{Cx, NodeBuilder, NodeId, TextContent, container};
use sigil_layout::{LayoutBuilderExt, LayoutEngine, LayoutStyle, TextMeasure};

const ROOT_WIDTH: f32 = 12_800.0;
const ROOT_HEIGHT: f32 = 100.0;
const CHILD_WIDTH: f32 = 12.0;
const CHILD_HEIGHT: f32 = 100.0;
const GRANDCHILD_WIDTH: f32 = 12.0;
const GRANDCHILD_HEIGHT: f32 = 1.0;
const CHILDREN: usize = 1_000;
const GRANDCHILDREN_PER_CHILD: usize = 100;
const AVAILABLE: Size = Size::fixed(ROOT_WIDTH, ROOT_HEIGHT);
const ITERATIONS: usize = 500;

struct StubMeasure;

impl TextMeasure for StubMeasure {
    fn measure(&self, _: &TextContent, _: Size) -> LogicalSize {
        LogicalSize::new(0.0, 0.0)
    }
}

fn build_tree(cx: &mut Cx) -> NodeId {
    let mut root = container().layout(LayoutStyle::flex_row().fixed_size(ROOT_WIDTH, ROOT_HEIGHT));
    for _ in 0..CHILDREN {
        root = root.child(column());
    }
    cx.add(root)
}

fn column() -> NodeBuilder {
    let mut child =
        container().layout(LayoutStyle::flex_column().fixed_size(CHILD_WIDTH, CHILD_HEIGHT));
    for _ in 0..GRANDCHILDREN_PER_CHILD {
        child = child.child(
            container()
                .layout(LayoutStyle::block().fixed_size(GRANDCHILD_WIDTH, GRANDCHILD_HEIGHT)),
        );
    }
    child
}

fn main() {
    eprintln!("building wide_tree (1 + 1000 + 100k nodes)…");
    let mut cx = Cx::new_root();
    let root = build_tree(&mut cx);
    let mut engine = LayoutEngine::with_text_measure(Box::new(StubMeasure));

    eprintln!("warm compute…");
    engine.compute(&mut cx, root, AVAILABLE).unwrap();
    cx.clear_dirty();

    eprintln!("profiling {ITERATIONS} broadcast measures…");
    let start = std::time::Instant::now();
    for _ in 0..ITERATIONS {
        cx.mark_mutated();
        engine.compute(&mut cx, root, AVAILABLE).unwrap();
        std::hint::black_box(engine.result(root));
    }
    let elapsed = start.elapsed();
    eprintln!(
        "done in {:?} ({:.2} ms / iter)",
        elapsed,
        elapsed.as_secs_f64() * 1000.0 / ITERATIONS as f64
    );
}
