#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]

use sigil::geometry::{Point, Rect, Size};
use sigil::kernel::*;
use sigil::layout::{LayoutBuilderExt, LayoutEngine, LayoutStyle};
use sigil::reactive::{Runtime, signal};
use sigil::scene::emit_scene_clipped;
use sigil_kittest::FixedMeasure;

fn layout(cx: &mut Cx, root: NodeId) {
    let mut engine = LayoutEngine::with_text_measure(Box::new(FixedMeasure::default()));
    engine
        .compute(cx, root, Size::fixed(320.0, 240.0))
        .expect("layout compute succeeds");
}

#[test]
fn culled_parents_still_paint_their_top_layer_children_after_one_is_added() {
    Runtime::scope("culled-top-layer", || {
        let late = signal(false);
        let popup = |label: &'static str| {
            container()
                .layout(LayoutStyle::block().fixed_size(80.0, 24.0))
                .with(TopLayer)
                .child(text(label))
        };
        let mut cx = Cx::new_root();
        let root = cx.add(
            container()
                .layout(LayoutStyle::flex_column().fixed_size(320.0, 240.0))
                .child(container().layout(LayoutStyle::block().fixed_size(320.0, 2_000.0)))
                .child(
                    container()
                        .layout(LayoutStyle::flex_column().fixed_size(320.0, 48.0))
                        .child(popup("Early"))
                        .children_reactive(move || {
                            if late.get() {
                                vec![popup("Late")]
                            } else {
                                Vec::new()
                            }
                        }),
                ),
        );
        let viewport = Some(Rect::new(Point::new(0.0, 0.0), 320.0, 240.0));
        layout(&mut cx, root);
        let scene = emit_scene_clipped(&cx, root, viewport);
        assert!(
            scene.find_text("Early").is_some(),
            "a culled parent keeps its popup"
        );
        late.set(true);
        layout(&mut cx, root);
        let added = emit_scene_clipped(&cx, root, viewport);
        assert!(added.find_text("Early").is_some());
        assert!(
            added.find_text("Late").is_some(),
            "a popup added under a culled parent paints on the next frame"
        );
    });
}
