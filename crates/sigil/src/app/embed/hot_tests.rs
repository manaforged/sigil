use std::cell::Cell;
use std::rc::Rc;

use crate::app::Host;
use crate::events::EventRouter;
use crate::reactive::{Runtime, signal};
use crate::terminal::step;

#[test]
fn a_patch_remounts_the_root_and_keeps_outside_state() {
    Runtime::scope("hot-patch", || {
        let count = signal(3_u32);
        let calls = Rc::new(Cell::new(0_u32));
        let counter = Rc::clone(&calls);
        let mut host = Host::cells();
        host.mount_hot(move |cx| {
            counter.set(counter.get() + 1);
            cx.add(crate::kernel::text(format!("Count: {}", count.get())))
        });
        let mut router = EventRouter::new();
        let _ = step(&mut host, &mut router, [], 30, 3);
        assert_eq!(calls.get(), 1);
        assert!(host.fire_hot_patch(), "a hot root is mounted");
        let grid = step(&mut host, &mut router, [], 30, 3).expect("frame");
        assert_eq!(calls.get(), 2, "the patch re-ran the root");
        assert!(grid.text().contains("Count: 3"), "{}", grid.text());
    });
}
