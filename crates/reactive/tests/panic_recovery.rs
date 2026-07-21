//! `Runtime::recover_from_panic` restores a clean resting state after a
//! frame is abandoned mid-flight.
//!
//! This is the contract the app frame loop relies on to honour
//! `PanicPolicy::Isolate`: it catches a panic in a frame, then calls
//! `recover_from_panic` so the NEXT frame runs. Before this, a panic in a
//! widget handler took the process down, contradicting the config.

#![allow(unused_crate_dependencies)]

use std::cell::Cell;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::rc::Rc;

use sigil_reactive::{DrawPhase, Runtime, effect, signal};

#[test]
fn a_frame_abandoned_mid_phase_recovers_to_a_runnable_state() {
    Runtime::scope("panic-recovery-frame", || {
        // Enter a frame and stop partway, as a panic mid-paint would.
        Runtime::begin_effect();
        Runtime::begin_layout();
        Runtime::begin_paint();
        assert_eq!(Runtime::current_phase(), DrawPhase::Paint);

        // No `end_frame`: the frame is abandoned with the phase stuck.
        Runtime::recover_from_panic();
        assert_eq!(
            Runtime::current_phase(),
            DrawPhase::Idle,
            "recovery must return the runtime to Idle so the next frame's \
             begin_effect (which asserts Idle) can run",
        );

        // A full, clean frame must now be possible. begin_effect debug-
        // asserts phase == Idle, so a botched recovery would panic here.
        let ran = Rc::new(Cell::new(0_u32));
        {
            let ran = Rc::clone(&ran);
            effect(move || ran.set(ran.get() + 1));
        }
        Runtime::begin_effect();
        Runtime::begin_layout();
        Runtime::begin_paint();
        Runtime::end_frame();
        assert!(ran.get() >= 1, "effect ran in the frame after recovery");
    });
}

#[test]
fn the_runtime_stays_usable_after_an_effect_panics() {
    Runtime::scope("panic-recovery-usable", || {
        let value = signal(0_i32);
        let panic_armed = Rc::new(Cell::new(false));

        // A widget-ish effect that will panic when re-run. By design the
        // reactive runtime DROPS an effect that panics mid-run (its
        // `RunGuard` sees `thread::panicking()` and does not restore the
        // closure), so this test does not expect it to survive — it checks
        // that everything ELSE keeps working, which is what "isolate the
        // failure, keep the app" actually means.
        {
            let panic_armed = Rc::clone(&panic_armed);
            effect(move || {
                let _ = value.get();
                assert!(!panic_armed.get(), "boom");
            });
        }

        panic_armed.set(true);
        let result = catch_unwind(AssertUnwindSafe(|| value.set(1)));
        assert!(result.is_err(), "the effect panicked through the write");

        Runtime::recover_from_panic();

        // The runtime must be fully usable: a NEW effect registered after
        // recovery runs, and further writes to the same signal propagate to
        // it without tripping over the dead effect's leftover bookkeeping.
        let after_runs = Rc::new(Cell::new(0_u32));
        {
            let after_runs = Rc::clone(&after_runs);
            effect(move || {
                let _ = value.get();
                after_runs.set(after_runs.get() + 1);
            });
        }
        assert_eq!(after_runs.get(), 1, "new effect ran on registration");

        value.set(2);
        assert_eq!(
            after_runs.get(),
            2,
            "a write after recovery must still drive live effects",
        );
    });
}
