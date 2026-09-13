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

use std::any::Any;
use std::cell::Cell;
use std::rc::Rc;

use sigil::actions::Action;
use sigil::reactive::{DrawPhase, Entity, Runtime, effect, set_global, signal};

#[derive(Clone)]
struct Tick;

impl Action for Tick {
    fn debug_name(&self) -> &'static str {
        "Tick"
    }
    fn boxed_clone(&self) -> Box<dyn Action> {
        Box::new(self.clone())
    }
    fn as_any(&self) -> &dyn Any {
        self
    }
}

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
        let trace: Rc<std::cell::RefCell<Vec<DrawPhase>>> =
            Rc::new(std::cell::RefCell::new(vec![Runtime::current_phase()]));
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
#[should_panic(expected = "DrawPhase::Paint")]
fn update_during_paint_panics_in_debug() {
    Runtime::scope("update_during_paint", || {
        let counter = Entity::<u32>::new("counter", |_| 0_u32);
        Runtime::begin_effect();
        Runtime::begin_layout();
        Runtime::begin_paint();
        counter.update(|n, _cx| {
            *n += 1;
        });
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
#[should_panic(expected = "DrawPhase::Paint")]
fn notify_during_paint_panics_in_debug() {
    Runtime::scope("notify_paint", || {
        let counter = Entity::<u32>::new("counter", |_| 0_u32);
        Runtime::begin_effect();
        Runtime::begin_layout();
        Runtime::begin_paint();
        counter.update(|_n, cx| {
            cx.notify();
        });
    });
}

#[cfg(debug_assertions)]
#[test]
#[should_panic(expected = "DrawPhase::Paint")]
fn dispatch_action_during_paint_panics_in_debug() {
    Runtime::scope("dispatch_paint", || {
        let host = Entity::<()>::new("host", |_| ());
        Runtime::begin_effect();
        Runtime::begin_layout();
        Runtime::begin_paint();
        host.update(|_, cx| {
            cx.dispatch(&Tick);
        });
    });
}

#[cfg(not(debug_assertions))]
#[test]
fn dispatch_action_during_paint_defers_in_release() {
    let fired = Rc::new(Cell::new(0_u32));
    let fired_clone = Rc::clone(&fired);

    Runtime::scope("dispatch_defer", || {
        let host = Entity::<()>::new("host", |_| ());
        host.update(|_, cx| {
            let handle = cx.focus_handle();
            cx.action::<Tick>(&handle, move |_a, _dcx| {
                fired_clone.set(fired_clone.get() + 1);
            });
            cx.focus(&handle);
        });
        Runtime::begin_effect();
        Runtime::begin_layout();
        Runtime::begin_paint();
        host.update(|_, cx| {
            cx.dispatch(&Tick);
        });
        assert_eq!(fired.get(), 0, "handler is queued, not fired, mid-Paint");
        Runtime::end_frame();
        assert_eq!(fired.get(), 1, "deferred handler fires once at end_frame");
    });
}

#[test]
fn dispatch_outside_paint_fires_immediately() {
    let fired = Rc::new(Cell::new(0_u32));
    let fired_clone = Rc::clone(&fired);
    Runtime::scope("immediate_dispatch", || {
        let host = Entity::<()>::new("host", |_| ());
        host.update(|_, cx| {
            let handle = cx.focus_handle();
            cx.action::<Tick>(&handle, move |_a, _dcx| {
                fired_clone.set(fired_clone.get() + 1);
            });
            cx.focus(&handle);
            cx.dispatch(&Tick);
        });
        assert_eq!(fired.get(), 1, "dispatch outside Paint fires immediately");
        Runtime::begin_effect();
        Runtime::begin_layout();
        Runtime::begin_paint();
        Runtime::end_frame();
        assert_eq!(Runtime::current_phase(), DrawPhase::Idle);
        assert_eq!(fired.get(), 1, "no extra handler fires when queue empty");
    });
}

#[test]
fn observer_fires_in_effect_phase() {
    let phase_at_fire = Rc::new(Cell::new(DrawPhase::Idle));
    let capture = Rc::clone(&phase_at_fire);
    Runtime::scope("observer_in_effect", || {
        let observed = Entity::<u32>::new("observed", |_| 0_u32);
        let watcher = Entity::<()>::new("watcher", |_| ());
        let _sub = watcher.update(|_, cx| {
            cx.observe::<u32>(&observed, move |_w, _e, _cx| {
                capture.set(Runtime::current_phase());
            })
        });
        Runtime::begin_effect();
        observed.update(|n, cx| {
            *n += 1;
            cx.notify();
        });
        assert_eq!(
            phase_at_fire.get(),
            DrawPhase::Effect,
            "observer fires during Effect phase (drain at outermost-update exit)"
        );
        Runtime::begin_layout();
        Runtime::begin_paint();
        Runtime::end_frame();
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
fn begin_layout_drains_pending_observers() {
    let fired = Rc::new(Cell::new(false));
    let fired_clone = Rc::clone(&fired);
    Runtime::scope("layout_drain", || {
        let observed = Entity::<u32>::new("observed", |_| 0_u32);
        let watcher = Entity::<()>::new("watcher", |_| ());
        let _sub = watcher.update(|_, cx| {
            cx.observe::<u32>(&observed, move |_, _, _| {
                fired_clone.set(true);
            })
        });
        Runtime::begin_effect();
        observed.update(|n, cx| {
            *n += 1;
            cx.notify();
        });
        Runtime::begin_layout();
        assert!(
            fired.get(),
            "observer fired by the time begin_layout returns (Effect drain bracket)"
        );
        Runtime::begin_paint();
        Runtime::end_frame();
    });
}

#[test]
fn read_paths_in_paint_are_legal() {
    Runtime::scope("read_paint_legal", || {
        let s = signal(42_i32);
        let e = Entity::<u32>::new("e", |_| 99_u32);
        Runtime::begin_effect();
        Runtime::begin_layout();
        Runtime::begin_paint();
        let s_val = s.get();
        let e_val = e.read(|n| *n);
        assert_eq!(s_val, 42, "Signal::get is legal during Paint");
        assert_eq!(e_val, 99, "Entity::read is legal during Paint");
        Runtime::end_frame();
    });
}

#[test]
fn is_painting_reflects_current_phase() {
    let saw_effect = Rc::new(Cell::new(true));
    let saw_layout = Rc::new(Cell::new(true));
    let saw_effect_c = Rc::clone(&saw_effect);
    let saw_layout_c = Rc::clone(&saw_layout);
    Runtime::scope("is_painting", || {
        let host = Entity::<()>::new("host", |_| ());
        Runtime::begin_effect();
        host.update(|_, cx| saw_effect_c.set(cx.painting()));
        Runtime::begin_layout();
        host.update(|_, cx| saw_layout_c.set(cx.painting()));
        Runtime::begin_paint();
        assert_eq!(
            Runtime::current_phase(),
            DrawPhase::Paint,
            "current_phase reads Paint"
        );
        Runtime::end_frame();
    });
    assert!(
        !saw_effect.get(),
        "is_painting() is false during Effect phase"
    );
    assert!(
        !saw_layout.get(),
        "is_painting() is false during Layout phase"
    );
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

#[cfg(debug_assertions)]
#[test]
#[should_panic(expected = "DrawPhase::Paint")]
fn focus_during_paint_panics_in_debug() {
    Runtime::scope("focus_paint", || {
        let host = Entity::<()>::new("host", |_| ());
        Runtime::begin_effect();
        Runtime::begin_layout();
        Runtime::begin_paint();
        host.update(|_, cx| {
            let handle = cx.focus_handle();
            cx.focus(&handle);
        });
    });
}

#[cfg(debug_assertions)]
#[test]
#[should_panic(expected = "set_global")]
fn set_global_during_paint_panics_in_debug() {
    #[derive(Default)]
    struct MyGlobal(#[allow(dead_code)] u32);
    Runtime::scope("set_global_paint", || {
        Runtime::begin_effect();
        Runtime::begin_layout();
        Runtime::begin_paint();
        set_global(MyGlobal(7));
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
