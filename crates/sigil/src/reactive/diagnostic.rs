#![allow(clippy::panic)]

use core::panic::Location;

#[cold]
#[track_caller]
pub(crate) fn no_scope(api: &'static str, caller: &'static Location<'static>) -> ! {
    panic!("{api} called outside a reactive scope (at {caller})")
}

#[cold]
#[track_caller]
pub(crate) fn disposed_signal(op: &'static str, caller: &'static Location<'static>) -> ! {
    panic!("Signal::{op} on a disposed signal (at {caller})")
}

#[cold]
#[track_caller]
pub(crate) fn reactive_cycle(cycle: Cycle) {
    match cycle {
        Cycle::ReEntry(loc) => {
            #[cfg(debug_assertions)]
            panic!("reactive cycle detected: effect at {loc:?} would re-enter itself");

            #[cfg(not(debug_assertions))]
            eprintln!(
                "reactive cycle detected: effect at {loc:?} would re-enter itself; skipping re-run"
            );
        }
        Cycle::Budget(limit) => tracing::error!(
            target: "sigil::reactive",
            "reactive cycle detected: {limit} effect runs in one drain; the rest stay dirty until the next drain"
        ),
    }
}

#[derive(Clone, Copy, Debug)]
pub(crate) enum Cycle {
    ReEntry(Option<&'static Location<'static>>),
    Budget(usize),
}
