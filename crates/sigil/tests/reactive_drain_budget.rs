#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]

use std::sync::mpsc;
use std::time::Duration;

use sigil::reactive::{Runtime, batch, effect, signal};

#[test]
fn ping_pong_effects_stop_at_the_drain_budget() {
    let (done, finished) = mpsc::channel();
    std::thread::spawn(move || {
        Runtime::scope("reactive-ping-pong", || {
            let x = signal(0_u64);
            let y = signal(0_u64);
            batch(|| {
                effect(move || y.set(x.get() + 1));
                effect(move || x.set(y.get() + 1));
            });
        });
        let _ = done.send(());
    });
    assert!(
        finished.recv_timeout(Duration::from_secs(10)).is_ok(),
        "a write cycle between two effects must stop instead of spinning"
    );
}
