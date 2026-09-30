#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]
#![allow(clippy::unwrap_used, clippy::float_cmp)]

use sigil::events::EventRouter;
use sigil::events::ShellEvent;
use sigil::events::{ScrollDelta, ScrollEvent, ScrollEventInit, ScrollPhase};
use sigil::geometry::Point;
use sigil::kernel::Cx;
use sigil::layout::LayoutEngine;
use sigil::prelude::*;
use sigil_kittest::FixedMeasure;

const TEXT_MEASURE: FixedMeasure = FixedMeasure {
    width: 60.0,
    height: 16.0,
};

fn wheel(cx: &mut Cx, layout: &LayoutEngine, at: Point) {
    EventRouter::new().route(
        cx,
        layout,
        ShellEvent::Scroll(ScrollEvent::new(ScrollEventInit {
            position: at,
            delta: ScrollDelta::Pixels(Point::new(0.0, -400.0)),
            modifiers: ModifiersState::empty(),
            phase: ScrollPhase::Moved,
        })),
    );
}

#[test]
fn a_wheel_over_an_open_modal_never_scrolls_the_page_behind_it() {
    sigil::reactive::Runtime::scope("sigil-modal-scroll-contained", || {
        let page = signal(0.0_f32);
        let open = signal(true);
        let mut cx = Cx::new_root();
        let root = cx.add(
            scroll(
                container()
                    .layout(LayoutStyle::flex_column().fixed_size(640.0, 2_000.0))
                    .child(
                        modal(open)
                            .title("Edit task")
                            .width(320.0)
                            .height(200.0)
                            .child(button("Save")),
                    ),
            )
            .extent(2_000.0)
            .height(480.0)
            .scroll_bind(page),
        );
        let mut layout = LayoutEngine::with_text_measure(Box::new(TEXT_MEASURE));
        layout
            .compute(&mut cx, root, Size::fixed(640.0, 480.0))
            .expect("layout computes");

        wheel(&mut cx, &layout, Point::new(320.0, 240.0));
        wheel(&mut cx, &layout, Point::new(20.0, 20.0));

        assert_eq!(page.get(), 0.0, "the page behind an open modal moved");
    });
}
