#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::panic,
    clippy::ignored_unit_patterns
)]

use sigil::reactive::{DrawPhase, Runtime, effect, signal};

#[test]
fn frame_dirty_tracks_visual_mutations() {
    Runtime::scope("frame_dirty_tracks_visual_mutations", || {
        let tracked = signal(0_i32);
        effect(move || {
            let _ = tracked.get();
        });
        let untracked = signal(0_i32);

        Runtime::begin_effect();
        assert!(!Runtime::frame_dirty(), "begin_effect clears the flag");
        untracked.set(1);
        assert!(
            !Runtime::frame_dirty(),
            "a write with no subscribers changes nothing visible"
        );
        tracked.set(1);
        assert!(
            Runtime::frame_dirty(),
            "a write to a subscribed signal marks the frame dirty"
        );
        Runtime::begin_layout();
        Runtime::begin_paint();
        Runtime::end_frame();

        Runtime::begin_effect();
        assert!(!Runtime::frame_dirty(), "the next frame starts clean");
        Runtime::mark_frame_dirty();
        assert!(
            Runtime::frame_dirty(),
            "an explicit structural mark sets it"
        );
        Runtime::begin_layout();
        Runtime::begin_paint();
        Runtime::end_frame();
    });
}

#[test]
fn idle_phase_is_default() {
    Runtime::scope("idle_default", || {
        assert_eq!(
            Runtime::current_phase(),
            DrawPhase::Idle,
            "before any begin_* call the runtime is in Idle"
        );
    });
}

#[test]
fn phase_round_trips_through_idle() {
    Runtime::scope("round_trip", || {
        let trace: std::rc::Rc<std::cell::RefCell<Vec<DrawPhase>>> =
            std::rc::Rc::new(std::cell::RefCell::new(vec![Runtime::current_phase()]));
        Runtime::begin_effect();
        trace.borrow_mut().push(Runtime::current_phase());
        Runtime::begin_layout();
        trace.borrow_mut().push(Runtime::current_phase());
        Runtime::begin_paint();
        trace.borrow_mut().push(Runtime::current_phase());
        Runtime::end_frame();
        trace.borrow_mut().push(Runtime::current_phase());
        assert_eq!(
            trace.borrow().clone(),
            vec![
                DrawPhase::Idle,
                DrawPhase::Effect,
                DrawPhase::Layout,
                DrawPhase::Paint,
                DrawPhase::Idle,
            ],
            "Idle→Effect→Layout→Paint→Idle round-trip"
        );
    });
}

#[cfg(debug_assertions)]
#[test]
#[should_panic(expected = "Signal::set")]
fn signal_set_during_paint_panics_in_debug() {
    Runtime::scope("signal_set_paint", || {
        let s = signal(0_i32);
        Runtime::begin_effect();
        Runtime::begin_layout();
        Runtime::begin_paint();
        s.set(7);
    });
}

#[cfg(debug_assertions)]
#[test]
#[should_panic(expected = "Signal::update")]
fn signal_update_during_paint_panics_in_debug() {
    Runtime::scope("signal_update_paint", || {
        let s = signal(0_i32);
        Runtime::begin_effect();
        Runtime::begin_layout();
        Runtime::begin_paint();
        s.update(|n| *n += 1);
    });
}

#[cfg(debug_assertions)]
#[test]
#[should_panic(expected = "expected Layout")]
fn phase_transition_out_of_order_panics_in_debug() {
    Runtime::scope("ooo_transition", || {
        Runtime::begin_effect();
        Runtime::begin_paint();
    });
}

#[test]
fn read_paths_in_paint_are_legal() {
    Runtime::scope("read_paint_legal", || {
        let s = signal(42_i32);
        Runtime::begin_effect();
        Runtime::begin_layout();
        Runtime::begin_paint();
        let s_val = s.get();
        assert_eq!(s_val, 42, "Signal::get is legal during Paint");
        Runtime::end_frame();
    });
}

#[test]
fn end_frame_drains_to_idle_with_empty_queue() {
    Runtime::scope("end_frame_drain", || {
        Runtime::begin_effect();
        Runtime::begin_layout();
        Runtime::begin_paint();
        Runtime::end_frame();
        assert_eq!(Runtime::current_phase(), DrawPhase::Idle);
    });
}

#[test]
fn current_phase_observable_throughout_frame() {
    Runtime::scope("observable", || {
        assert_eq!(Runtime::current_phase(), DrawPhase::Idle);
        Runtime::begin_effect();
        assert_eq!(Runtime::current_phase(), DrawPhase::Effect);
        Runtime::begin_layout();
        assert_eq!(Runtime::current_phase(), DrawPhase::Layout);
        Runtime::begin_paint();
        assert_eq!(Runtime::current_phase(), DrawPhase::Paint);
        Runtime::end_frame();
        assert_eq!(Runtime::current_phase(), DrawPhase::Idle);
    });
}
