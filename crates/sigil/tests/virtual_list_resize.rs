#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]
#![allow(clippy::expect_used, clippy::indexing_slicing)]

use std::rc::Rc;

use sigil::Host;
use sigil::events::EventRouter;
use sigil::kernel::{Cx, container, text};
use sigil::prelude::{LayoutBuilderExt, LayoutStyle, virtual_list};
use sigil::reactive::{Runtime, Signal, signal};

const COLS: u16 = 30;
const ROWS: u16 = 10;

fn frame(host: &mut Host, router: &mut EventRouter) -> String {
    sigil::terminal::step(host, router, [], COLS, ROWS)
        .expect("layout")
        .text()
}

fn settle(host: &mut Host, router: &mut EventRouter) -> String {
    (0..8).fold(String::new(), |_, _| frame(host, router))
}

fn grow(lines: Signal<Vec<usize>>, row: usize) {
    lines.update(|lines| lines[row] = 3);
}

#[test]
fn measured_rows_keep_the_view_still_when_a_row_grows() {
    Runtime::scope("virtual-list-resize", || {
        let lines = signal(vec![1_usize; 60]);
        let scroll_y = signal(0.0_f32);
        let keys = signal(Rc::new((0..60_u64).collect::<Vec<_>>()));
        let mut host = Host::cells();
        let mut router = EventRouter::new();
        host.mount(move |cx: &mut Cx| {
            cx.add(
                container()
                    .layout(LayoutStyle::flex_column().fill_width().fill_height())
                    .child(
                        virtual_list(keys, move |index, _: &u64| {
                            container()
                                .layout(LayoutStyle::flex_column().fill_width())
                                .children_reactive(move || {
                                    (0..lines.get()[index])
                                        .map(|line| text(format!("r{index} l{line}")))
                                        .collect::<Vec<_>>()
                                })
                        })
                        .key_by(|_, key| *key)
                        .measured()
                        .viewport_fill()
                        .scroll_bind(scroll_y)
                        .auto_pin_to_bottom(true)
                        .scroll_to_end(),
                    ),
            )
        });
        let pinned = settle(&mut host, &mut router);
        assert!(pinned.contains("r59 l0"), "tail shown:\n{pinned}");
        grow(lines, 57);
        let grown = frame(&mut host, &mut router);
        assert!(
            grown.contains("r59 l0"),
            "tail kept in the same frame:\n{grown}"
        );

        scroll_y.set(25.0 * Host::CELL);
        let held = settle(&mut host, &mut router);
        assert!(held.contains("r30 l0"), "scrolled view:\n{held}");
        grow(lines, 22);
        let after = frame(&mut host, &mut router);
        assert_eq!(after, held, "rows above the view must not move it");
    });
}
