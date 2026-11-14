use sigil::geometry::{Constraint, Point, Rect};
use sigil::kernel::*;
use sigil::layout::{LayoutEngine, LayoutStyle};
use sigil::reactive::{Owner, signal};
use sigil::scene::emit_scene;
use sigil::widgets::Widget;
use sigil_kittest::FakeMeasure;

fn layout(cx: &mut Cx, root: NodeId) {
    let mut engine = LayoutEngine::new(Box::new(FakeMeasure::default()));
    engine
        .compute(cx, root, Constraint::fixed(320.0, 240.0))
        .expect("layout compute succeeds");
}

#[test]
fn culled_parents_still_paint_their_top_layer_children_after_one_is_added() {
    Owner::scope("culled-top-layer", || {
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
        let scene = emit_scene(&cx, root, viewport);
        assert!(
            scene.find_text("Early").is_some(),
            "a culled parent keeps its popup"
        );
        late.set(true);
        layout(&mut cx, root);
        let added = emit_scene(&cx, root, viewport);
        assert!(added.find_text("Early").is_some());
        assert!(
            added.find_text("Late").is_some(),
            "a popup added under a culled parent paints on the next frame"
        );
    });
}
