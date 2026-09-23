#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]
#![allow(missing_docs)]
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::ignored_unit_patterns
)]

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use sigil::reactive::{DrawPhase, Runtime};

#[test]
fn frame_driver_walks_idle_to_idle_through_all_phases() {
    Runtime::scope("frame_driver::phase_walk", || {
        let trace: Rc<RefCell<Vec<DrawPhase>>> =
            Rc::new(RefCell::new(vec![Runtime::current_phase()]));

        let layout_trace = Rc::clone(&trace);
        let paint_trace = Rc::clone(&trace);

        sigil::app::__drive_test_frame(
            move || layout_trace.borrow_mut().push(Runtime::current_phase()),
            move || paint_trace.borrow_mut().push(Runtime::current_phase()),
        );

        trace.borrow_mut().push(Runtime::current_phase());

        assert_eq!(
            trace.borrow().clone(),
            vec![
                DrawPhase::Idle,
                DrawPhase::Layout,
                DrawPhase::Paint,
                DrawPhase::Idle,
            ],
            "frame driver observes Layout inside layout pass, Paint inside paint pass, \
             and returns to Idle on exit",
        );
    });
}

#[test]
fn begin_effect_runs_before_layout_pass() {
    Runtime::scope("frame_driver::effect_before_layout", || {
        let observed_phase = Rc::new(Cell::new(DrawPhase::Idle));
        let phase_writer = Rc::clone(&observed_phase);
        let s = sigil::reactive::signal(0_i32);

        sigil::reactive::effect(move || {
            let _ = s.get();
            phase_writer.set(Runtime::current_phase());
        });

        assert_eq!(observed_phase.get(), DrawPhase::Idle);

        s.set(1);
        assert_eq!(
            observed_phase.get(),
            DrawPhase::Idle,
            "Idle-phase write fires effect synchronously in Idle",
        );

        sigil::app::__drive_test_frame(|| {}, || {});

        assert_eq!(
            Runtime::current_phase(),
            DrawPhase::Idle,
            "frame returns to Idle with no pending work",
        );
    });
}

#[cfg(debug_assertions)]
#[test]
#[should_panic(expected = "Signal::set")]
fn paint_phase_signal_set_panics_in_debug() {
    Runtime::scope("frame_driver::paint_signal_set_panics", || {
        let s = sigil::reactive::signal(0_i32);
        sigil::app::__drive_test_frame(
            || {},
            move || {
                s.set(7);
            },
        );
    });
}

#[test]
fn read_paths_in_paint_pass_are_legal() {
    Runtime::scope("frame_driver::read_in_paint", || {
        let s = sigil::reactive::signal(42_i32);
        let observed = Rc::new(Cell::new((0_i32, DrawPhase::Idle)));
        let writer = Rc::clone(&observed);

        sigil::app::__drive_test_frame(
            || {},
            move || {
                let s_val = s.get();
                writer.set((s_val, Runtime::current_phase()));
            },
        );

        let (s_val, phase) = observed.get();
        assert_eq!(s_val, 42, "Signal::get reads through during Paint");
        assert_eq!(phase, DrawPhase::Paint, "paint pass observed phase Paint");
    });
}
