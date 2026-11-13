#![allow(clippy::expect_used, clippy::indexing_slicing)]

use sigil::kernel::HostMode;
use std::rc::Rc;

use sigil::Host;
use sigil::kernel::{container, text};
use sigil::prelude::Widget;
use sigil::prelude::{LayoutStyle, virtual_list};
use sigil::reactive::{Owner, Signal, signal};
use sigil::widgets::Layout;

const COLS: u16 = 30;
const ROWS: u16 = 10;

fn frame(host: &mut Host) -> String {
    host.grid([], COLS, ROWS).expect("layout").text()
}

fn settle(host: &mut Host) -> String {
    (0..8).fold(String::new(), |_, _| frame(host))
}

fn grow(lines: Signal<Vec<usize>>, row: usize) {
    lines.update(|lines| lines[row] = 3);
}

#[test]
fn measured_rows_keep_the_view_still_when_a_row_grows() {
    Owner::scope("virtual-list-resize", || {
        let lines = signal(vec![1_usize; 60]);
        let scroll_y = signal(0.0_f32);
        let keys = signal(Rc::new((0..60_u64).collect::<Vec<_>>()));
        let mut host = Host::new(HostMode::Cell);
        host.mount(move || {
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
                    .fill()
                    .scroll_y(scroll_y)
                    .follow_end(true),
                )
        });
        let pinned = settle(&mut host);
        assert!(pinned.contains("r59 l0"), "tail shown:\n{pinned}");
        grow(lines, 57);
        let grown = frame(&mut host);
        assert!(
            grown.contains("r59 l0"),
            "tail kept in the same frame:\n{grown}"
        );

        scroll_y.set(25.0 * Host::CELL);
        let held = settle(&mut host);
        assert!(held.contains("r30 l0"), "scrolled view:\n{held}");
        grow(lines, 22);
        let after = frame(&mut host);
        assert_eq!(after, held, "rows above the view must not move it");
    });
}
