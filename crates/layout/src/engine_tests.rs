use sigil_hal::{LogicalSize, Size};
use sigil_kernel::{Cx, TextContent, prelude::container};
use sigil_reactive::{Runtime, effect, signal};
use sigil_style::{NodeStyleExt, Px, StyleAtom};

use crate::{LayoutBuilderExt, LayoutReactiveExt, LayoutStyle, TextMeasure};

use super::LayoutEngine;

struct Measure;

impl TextMeasure for Measure {
    fn measure(&self, _: &TextContent, _: Size) -> LogicalSize {
        LogicalSize::new(0.0, 0.0)
    }
}

#[test]
fn bounds_callbacks_invalidate_geometry_for_the_next_frame() {
    Runtime::scope("layout-bounds-feedback", || {
        let measured = signal((0.0, 0.0));
        let shifted = signal(false);
        effect(move || {
            if measured.get().1 > 0.0 {
                shifted.set(true);
            }
        });
        let mut cx = Cx::new_root();
        let root = cx.add(
            container()
                .layout(LayoutStyle::block().fixed_size(80.0, 40.0))
                .size_reactive(measured)
                .child(
                    container()
                        .layout(LayoutStyle::block().fixed_size(20.0, 10.0))
                        .style_reactive(move || {
                            if shifted.get() {
                                &[StyleAtom::MarginTop(Px(20))] as &[_]
                            } else {
                                &[StyleAtom::MarginTop(Px(0))] as &[_]
                            }
                        }),
                ),
        );
        let child = cx.first_child(root).expect("child");
        let mut engine = LayoutEngine::with_text_measure(Box::new(Measure));
        let viewport = Size::fixed(80.0, 40.0);
        engine
            .compute(&mut cx, root, viewport)
            .expect("measurement");
        assert!(shifted.peek(), "the measurement requests a new position");
        engine.compute(&mut cx, root, viewport).expect("next frame");
        let top = engine.result(child).expect("child layout").rect().origin.y;
        assert!((top - 20.0).abs() < f32::EPSILON, "content stays at {top}");
        let generation = engine.generation();
        engine
            .compute(&mut cx, root, viewport)
            .expect("unchanged frame");
        assert_eq!(
            engine.generation(),
            generation,
            "settled geometry stays cached"
        );
    });
}

#[test]
fn repaint_reuses_layout_storage_and_preserves_geometry() {
    Runtime::scope("layout-repaint-storage", || {
        let revision = signal(0);
        let mut cx = Cx::new_root();
        let root = cx.add(
            container()
                .layout(LayoutStyle::flex_column().fixed_size(80.0, 40.0))
                .children_reactive(move || {
                    let _revision = revision.get();
                    (0..20)
                        .map(|_| container().layout(LayoutStyle::block().fixed_size(80.0, 2.0)))
                        .collect::<Vec<_>>()
                }),
        );
        let mut engine = LayoutEngine::with_text_measure(Box::new(Measure));
        let viewport = Size::fixed(80.0, 40.0);
        engine
            .compute(&mut cx, root, viewport)
            .expect("first frame");
        let expected: Vec<_> = cx
            .get(root)
            .expect("root")
            .children()
            .iter()
            .map(|id| engine.result(*id).expect("child layout").rect())
            .collect();
        for frame in 1..100 {
            revision.set(frame);
            engine.compute(&mut cx, root, viewport).expect("repaint");
            let actual: Vec<_> = cx
                .get(root)
                .expect("root")
                .children()
                .iter()
                .map(|id| engine.result(*id).expect("child layout").rect())
                .collect();
            assert_eq!(actual, expected, "repaint changes no geometry");
            assert_eq!(
                engine.drive.count(),
                cx.len(),
                "frame {frame} retains dead layout nodes"
            );
        }
    });
}
