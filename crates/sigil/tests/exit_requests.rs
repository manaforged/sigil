#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::sync::Mutex;

use sigil::Host;
use sigil::app::FrameHooks;
use sigil::events::EventRouter;
use sigil::prelude::*;
use sigil::reactive::Runtime;

const VIEW: (f32, f32) = (400.0, 200.0);

static EXIT_FLAG: Mutex<()> = Mutex::new(());

fn mounted(label: &'static str) -> (Host, EventRouter) {
    let mut host = Host::new();
    host.mount(column().child(text(label)));
    (host, EventRouter::new())
}

#[test]
fn an_exit_requested_outside_a_frame_ends_the_next_frame() {
    let _serial = EXIT_FLAG
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    Runtime::scope("exit-outside-frame", || {
        let (mut host, mut router) = mounted("Exit from outside a frame");
        sigil::request_exit();

        let next = host.frame(&mut router, VIEW, 1.0 / 60.0, FrameHooks::default());
        let after = host.frame(&mut router, VIEW, 1.0 / 60.0, FrameHooks::default());

        assert!(next.exit, "an exit requested outside a frame was dropped");
        assert!(!after.exit, "an exit request ends one frame only");
    });
}

#[test]
fn an_exit_requested_before_a_frame_whose_paint_panics_ends_the_next_frame() {
    let _serial = EXIT_FLAG
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    Runtime::scope("exit-survives-paint-panic", || {
        let (mut host, mut router) = mounted("Exit after a failed paint");
        sigil::request_exit();

        let failed = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            host.frame(
                &mut router,
                VIEW,
                1.0 / 60.0,
                FrameHooks {
                    paint: Some(Box::new(|_, _, _| {
                        std::panic::resume_unwind(Box::new("paint failed"))
                    })),
                    ..FrameHooks::default()
                },
            )
        }));
        Runtime::recover_from_panic();
        let next = host.frame(&mut router, VIEW, 1.0 / 60.0, FrameHooks::default());

        assert!(failed.is_err());
        assert!(
            next.exit,
            "the exit request was lost with the panicking frame"
        );
    });
}
