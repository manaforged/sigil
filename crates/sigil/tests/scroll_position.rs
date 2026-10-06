#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]
#![allow(clippy::unwrap_used)]

use sigil::events::ScrollEvent;
use sigil::geometry::Point;
use sigil::prelude::*;
use sigil::reactive::Runtime;
use sigil_kittest::Harness;
use sigil_kittest::input::{pointer_down, pointer_move, scroll_lines};

#[test]
fn scroll_and_pointer_handlers_see_the_same_local_position() {
    Runtime::scope("scroll-local", || {
        let seen = signal((Point::new(0.0, 0.0), Point::new(0.0, 0.0)));
        let mut ui = Harness::new(move || {
            container()
                .layout(LayoutStyle::block().fill().padding(100.0))
                .child(
                    container()
                        .layout(LayoutStyle::block().width(300.0).height(200.0))
                        .pointer_down(move |event: &mut PointerEvent| {
                            seen.update(|s| s.0 = event.position);
                        })
                        .on_scroll(move |event: &mut ScrollEvent| {
                            seen.update(|s| s.1 = event.position);
                        }),
                )
        })
        .pixels(600, 400);
        ui.settle();
        ui.send([pointer_move(150.0, 150.0), pointer_down(150.0, 150.0)]);
        ui.send([scroll_lines(150.0, 150.0, 1.0)]);
        ui.settle();
        let (pointer, wheel) = seen.peek();
        assert_eq!(pointer, Point::new(50.0, 50.0));
        assert_eq!(wheel, pointer);
    });
}
