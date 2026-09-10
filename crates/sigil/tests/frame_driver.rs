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

use sigil_actions::DeriveAction;
use sigil_reactive::{DrawPhase, Entity, Runtime};

#[derive(Clone, DeriveAction)]
struct Tick;

#[test]
fn frame_driver_walks_idle_to_idle_through_all_phases() {
    Runtime::scope("frame_driver::phase_walk", || {
        let trace: Rc<RefCell<Vec<DrawPhase>>> =
            Rc::new(RefCell::new(vec![Runtime::current_phase()]));

        let layout_trace = Rc::clone(&trace);
        let paint_trace = Rc::clone(&trace);

        sigil_app::__drive_test_frame(
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
fn action_registration_survives_frame_cycle() {
    let fired = Rc::new(Cell::new(0_u32));
    let fired_clone = Rc::clone(&fired);

    Runtime::scope("frame_driver::action_survives_frame", || {
        let host = Entity::<()>::new("host", |_| ());
        host.update(|_, cx| {
            let handle = cx.focus_handle();
            cx.action::<Tick>(&handle, move |_a, _dcx| {
                fired_clone.set(fired_clone.get() + 1);
            });
            cx.focus(&handle);
        });

        sigil_app::__drive_test_frame(|| {}, || {});

        Runtime::begin_effect();
        host.update(|_, cx| cx.dispatch(&Tick));
        assert_eq!(
            fired.get(),
            1,
            "handler registered before frame N fires on Effect-phase dispatch in frame N+1",
        );
        Runtime::begin_layout();
        Runtime::begin_paint();
        Runtime::end_frame();

        assert_eq!(
            Runtime::current_phase(),
            DrawPhase::Idle,
            "`end_frame` returns to Idle after second frame",
        );
    });
}

#[cfg(not(debug_assertions))]
#[test]
fn g3_deferred_action_fires_at_end_of_frame() {
    let fired = Rc::new(Cell::new(0_u32));
    let fired_clone = Rc::clone(&fired);

    Runtime::scope("frame_driver::deferred_action", || {
        let host = Entity::<()>::new("host", |_| ());
        host.update(|_, cx| {
            let handle = cx.focus_handle();
            cx.action::<Tick>(&handle, move |_a, _dcx| {
                fired_clone.set(fired_clone.get() + 1);
            });
            cx.focus(&handle);
        });

        let saw_during_paint = Rc::new(Cell::new(0_u32));
        let saw_clone = Rc::clone(&saw_during_paint);
        let fired_during = Rc::clone(&fired);
        let host_for_paint = host.clone();

        sigil_app::__drive_test_frame(
            || {},
            move || {
                host_for_paint.update(|_, cx| cx.dispatch(&Tick));
                saw_clone.set(fired_during.get());
            },
        );

        assert_eq!(
            saw_during_paint.get(),
            0,
            "handler does not fire mid-Paint (defer path)",
        );
        assert_eq!(
            fired.get(),
            1,
            "deferred handler fires exactly once at `end_frame`",
        );
    });
}

#[test]
fn begin_effect_runs_before_layout_pass() {
    Runtime::scope("frame_driver::effect_before_layout", || {
        let observed_phase = Rc::new(Cell::new(DrawPhase::Idle));
        let phase_writer = Rc::clone(&observed_phase);
        let s = sigil_reactive::signal(0_i32);

        sigil_reactive::effect(move || {
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

        sigil_app::__drive_test_frame(|| {}, || {});

        assert_eq!(
            Runtime::current_phase(),
            DrawPhase::Idle,
            "frame returns to Idle with no pending work",
        );
    });
}

#[cfg(debug_assertions)]
#[test]
#[should_panic(expected = "DrawPhase::Paint")]
fn paint_phase_write_panics_in_debug() {
    Runtime::scope("frame_driver::paint_write_panics", || {
        let counter = Entity::<u32>::new("counter", |_| 0_u32);
        sigil_app::__drive_test_frame(
            || {},
            move || {
                counter.update(|n, _cx| {
                    *n += 1;
                });
            },
        );
    });
}

#[cfg(debug_assertions)]
#[test]
#[should_panic(expected = "Signal::set")]
fn paint_phase_signal_set_panics_in_debug() {
    Runtime::scope("frame_driver::paint_signal_set_panics", || {
        let s = sigil_reactive::signal(0_i32);
        sigil_app::__drive_test_frame(
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
        let s = sigil_reactive::signal(42_i32);
        let e = Entity::<u32>::new("e", |_| 99_u32);
        let observed = Rc::new(Cell::new((0_i32, 0_u32, DrawPhase::Idle)));
        let writer = Rc::clone(&observed);

        sigil_app::__drive_test_frame(
            || {},
            move || {
                let s_val = s.get();
                let e_val = e.read(|n| *n);
                writer.set((s_val, e_val, Runtime::current_phase()));
            },
        );

        let (s_val, e_val, phase) = observed.get();
        assert_eq!(s_val, 42, "Signal::get reads through during Paint");
        assert_eq!(e_val, 99, "Entity::read reads through during Paint");
        assert_eq!(phase, DrawPhase::Paint, "paint pass observed phase Paint");
    });
}
