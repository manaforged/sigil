#![allow(clippy::unwrap_used, clippy::float_cmp)]

use std::cell::Cell;
use std::rc::Rc;

use super::display_scale;
use crate::Host;
use crate::app::{FrameHooks, FrameLayoutHook};
use crate::events::EventRouter;
use crate::geometry::Scale;
use crate::kernel::container;
use crate::reactive::Runtime;

#[test]
fn display_scale_follows_the_host_window_scale() {
    Runtime::scope("display-scale", || {
        assert_eq!(
            display_scale().get(),
            1.0,
            "outside a host the scale is 1.0"
        );
        let mut host = Host::new();
        host.mount(container);
        host.cx().set_scale(Scale::new(2.0));
        let seen = Rc::new(Cell::new(0.0));
        let read = Rc::clone(&seen);
        let after_layout: FrameLayoutHook<'_> =
            Box::new(move |_, _| read.set(display_scale().get()));
        let mut router = EventRouter::new();
        let _ = host.frame(
            &mut router,
            (400.0, 300.0),
            0.0,
            FrameHooks {
                input: None,
                after_layout: Some(after_layout),
                paint: None,
            },
        );
        assert_eq!(
            seen.get(),
            2.0,
            "inside a frame the scale is the window scale"
        );
    });
}
