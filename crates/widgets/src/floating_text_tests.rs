#![allow(clippy::unwrap_used, clippy::panic, clippy::indexing_slicing)]

use std::rc::Rc;

use sigil_reactive::{Runtime, signal};

use super::*;

fn red() -> ::sigil_style::Rgba8 {
    ::sigil_style::Rgba8 {
        r: 255,
        g: 32,
        b: 32,
        a: 255,
    }
}

#[test]
fn floating_text_returns_buildable_node() {
    Runtime::scope("fct-build-node", || {
        let time = signal(0.0_f32);
        let spawn = signal(FctSpawn::default());
        let _builder = floating_text(spawn).time(time).duration(1.0);
        spawn.set(FctSpawn::new([10.0, 20.0], "100", red()));
        spawn.set(FctSpawn::new([30.0, 40.0], "50", red()));
    });
}

#[test]
fn tick_advances_position_and_alpha() {
    Runtime::scope("fct-tick-advance", || {
        let time = signal(0.0_f32);
        let state = Rc::new(FctState::new(1.0, [10.0, -20.0], time, FCT_POOL_CAPACITY));
        assert!(state.spawn(FctSpawn::new([0.0, 0.0], "1", red()), 0.0));
        state.tick(0.5);
        let snap = state.snapshot();
        assert_eq!(snap.len(), 1);
        assert!((snap[0].position[0] - 5.0).abs() < 0.001);
        assert!((snap[0].position[1] + 10.0).abs() < 0.001);
        assert!((snap[0].alpha - 0.5).abs() < 0.001);
    });
}

#[test]
fn expired_particles_are_reclaimed() {
    Runtime::scope("fct-expire", || {
        let time = signal(0.0_f32);
        let state = Rc::new(FctState::new(0.5, [0.0, -32.0], time, FCT_POOL_CAPACITY));
        assert!(state.spawn(FctSpawn::new([0.0, 0.0], "9", red()), 0.0));
        assert_eq!(state.alive_count(), 1);
        state.tick(0.4);
        assert_eq!(state.alive_count(), 1);
        state.tick(0.6);
        assert_eq!(state.alive_count(), 0, "particle expired after duration");
    });
}

#[test]
fn empty_text_does_not_spawn() {
    Runtime::scope("fct-empty", || {
        let time = signal(0.0_f32);
        let state = Rc::new(FctState::new(1.2, [0.0, -32.0], time, FCT_POOL_CAPACITY));
        let inert = FctSpawn {
            at: [0.0, 0.0],
            text: String::new(),
            color: red(),
        };
        assert!(!state.spawn(inert, 0.0));
        assert_eq!(state.alive_count(), 0);
    });
}

#[test]
fn pool_capacity_is_capped() {
    Runtime::scope("fct-cap", || {
        let time = signal(0.0_f32);
        let state = Rc::new(FctState::new(10.0, [0.0, -32.0], time, FCT_POOL_CAPACITY));
        for i in 0..FCT_POOL_CAPACITY {
            #[allow(
                clippy::cast_precision_loss,
                reason = "pixel and char-count magnitudes are bounded; sub-unit precision is irrelevant at render scale"
            )]
            let f = i as f32;
            assert!(state.spawn(FctSpawn::new([f, f], "x", red()), 0.0));
        }
        assert_eq!(state.alive_count(), FCT_POOL_CAPACITY);
        assert!(!state.spawn(FctSpawn::new([0.0, 0.0], "y", red()), 0.0));
    });
}

#[test]
fn reactive_spawn_writes_into_pool() {
    Runtime::scope("fct-reactive-spawn", || {
        let time = signal(0.0_f32);
        let spawn = signal(FctSpawn::default());
        let state = Rc::new(FctState::new(1.2, [0.0, -32.0], time, FCT_POOL_CAPACITY));
        let effect = Rc::clone(&state);
        sigil_reactive::effect(move || {
            let payload = spawn.get();
            if !payload.text.is_empty() {
                let _ok = effect.spawn(payload, time.peek());
            }
        });
        spawn.set(FctSpawn::new([1.0, 2.0], "crit", red()));
        spawn.set(FctSpawn::new([3.0, 4.0], "miss", red()));
        assert_eq!(state.alive_count(), 2);
    });
}
