#![allow(missing_docs)]
#![allow(
    clippy::unwrap_used,
    clippy::indexing_slicing,
    clippy::cast_precision_loss
)]

//! Regression: switching a deep `children_reactive` subtree to taller content
//! inside a big tree (so layout takes the `incremental_sync_and_measure` path)
//! must keep the new content positioned in its pane — not escaped to the
//! window's top-left.
//!
//! The bug: the incremental path picked two "containment roots" — the tree root
//! (from the high reactive node) AND the scroll widget's nested fixed-size box
//! (from the deep new nodes) — and re-measured the nested one as an isolated
//! layout root anchored at (0,0), clobbering the content's real in-tree
//! position. A long editor file then painted over the sidebar. Fixed by
//! dropping containment roots nested under another (engine.rs `containment_roots`).

use arboard as _;
use criterion as _;
use image as _;
use pollster as _;
use sigil_actions as _;
use sigil_animation as _;
use sigil_animation_runtime as _;
use sigil_app as _;
use sigil_events as _;
use sigil_hal as _;
use sigil_kernel as _;
use sigil_layout as _;
use sigil_reactive as _;
use sigil_renderer as _;
use sigil_style as _;
use sigil_tasks as _;
use wgpu as _;

use sigil::prelude::*;

const VIEWPORT: Size = Size::fixed(1440.0, 900.0);

struct StubMeasure;

impl TextMeasure for StubMeasure {
    fn measure(&self, content: &TextContent, _space: sigil_hal::Size) -> sigil_hal::LogicalSize {
        let text = content.as_static_str().unwrap_or("");
        let lines = text.lines().count().max(1) as f32;
        sigil_hal::LogicalSize::new(120.0, lines * 16.8)
    }
}

#[test]
// One toggle-remeasure regression walk (57 lines); splitting it would
// obscure the scenario it pins.
#[allow(clippy::too_many_lines)]
fn deep_reactive_scroll_switch_stays_positioned() {
    sigil::reactive::Runtime::scope("incremental-scroll-position", || {
        let tall = signal(false);
        let pane = container()
            .layout(LayoutStyle::flex_column().grow(1.0))
            .children_reactive(move || {
                let lines = if tall.get() { 200 } else { 2 };
                let body = (0..lines)
                    .map(|i| format!("line {i} of source"))
                    .collect::<Vec<_>>()
                    .join("\n");
                let extent = (lines as f32).mul_add(16.8, 20.0);
                let code = container()
                    .layout(LayoutStyle::flex_column().padding(10.0).width(2000.0))
                    .child(text(body));
                vec![scroll(code, extent).width(580.0).height(720.0)]
            });
        // Filler inflates the live node count so the editor-switch dirty set
        // stays under half → the incremental layout path (where the bug lived).
        let mut filler = container().layout(LayoutStyle::flex_column());
        for i in 0..120 {
            filler = filler.child(label(format!("filler row {i}")));
        }
        let tree = container()
            .layout(LayoutStyle::flex_column().grow(1.0))
            .child(filler)
            .child(
                container()
                    .layout(LayoutStyle::flex_row().grow(1.0))
                    .child(
                        container()
                            .layout(LayoutStyle::flex_column().width(248.0))
                            .child(label("SIDEBAR".to_owned())),
                    )
                    .child(pane),
            );

        let mut cx = Cx::new_root();
        let root = cx.add(tree);
        let mut layout = LayoutEngine::with_text_measure(Box::new(StubMeasure));
        layout
            .compute(&mut cx, root, VIEWPORT)
            .expect("initial layout");

        tall.set(true); // open the "long file"
        layout
            .compute(&mut cx, root, VIEWPORT)
            .expect("layout after switch");

        // No tall content box may sit at the window's top-left corner.
        let mut escaped = Vec::new();
        cx.visit_preorder(root, |id, _node| {
            if let Some(res) = layout.result(id) {
                let r = res.rect();
                if r.height > 900.0 && r.origin.x < 60.0 && r.origin.y < 60.0 {
                    escaped.push((r.origin.x, r.origin.y, r.width, r.height));
                }
            }
        });
        assert!(
            escaped.is_empty(),
            "tall scroll content escaped to the top-left (incremental containment-root bug): {escaped:?}"
        );
    });
}
