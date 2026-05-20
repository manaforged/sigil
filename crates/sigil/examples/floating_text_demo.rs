//! Floating combat text demo — particle damage numbers without layout.
//!
//! The demo wires a 60 Hz tick signal and spawns three particles per
//! tick at randomised positions, simulating a burst-damage scenario.
//! Particles age + drift over `duration` seconds, then dispose.
//!
//! Run with `cargo run -p sigil --example floating_text_demo`.

#![allow(missing_docs)]

// Examples inherit the host crate's dev-deps even when they don't
// touch them. Silence the `unused_crate_dependencies` lint for the
// ones this demo doesn't pull in directly.
use criterion as _;
use sigil_app as _;
use sigil_events as _;
use sigil_hal as _;
use sigil_kernel as _;
use sigil_layout as _;
use sigil_reactive as _;
use sigil_renderer as _;

use sigil::prelude::*;
use sigil_style::Rgba8;

fn main() -> sigil::Result<()> {
    sigil::launch(("Sigil - floating combat text", 960, 540), build)
}

fn build(cx: &mut Cx) -> NodeId {
    let time = signal(0.0_f32);
    let spawn = signal(FctSpawn::default());

    // 60 Hz tick driver: each frame nudges `time` forward and emits a
    // burst of three particles. Real apps would route shell frame
    // ticks here; this demo synthesises them via a self-driving effect.
    let mut frame: u32 = 0;
    effect(move || {
        let now = time.get();
        let next = now + 1.0 / 60.0;
        time.set(next);
        frame = frame.wrapping_add(1);
        if frame.is_multiple_of(3) {
            spawn.set(burst_spawn(frame));
        }
    });

    let widget = floating_text(spawn)
        .time(time)
        .duration(1.6)
        .label("floating combat text");

    cx.add(
        container()
            .role(Role::Main)
            .class("w-full h-full bg-surface")
            .child(widget),
    )
}

fn burst_spawn(frame: u32) -> FctSpawn {
    // Pseudo-random scatter using a small LCG of the frame counter —
    // deterministic, no `rand` dep, fits inside the example budget.
    let seed = frame.wrapping_mul(2_654_435_761);
    let x = f32::from((seed & 0x1ff) as u16) + 60.0;
    let y = f32::from(((seed >> 9) & 0xff) as u8) + 200.0;
    let amount = (seed % 999) + 1;
    let color = if seed & 0x4000 == 0 {
        Rgba8 {
            r: 240,
            g: 60,
            b: 60,
            a: 255,
        }
    } else {
        Rgba8 {
            r: 240,
            g: 200,
            b: 80,
            a: 255,
        }
    };
    FctSpawn {
        at: [x, y],
        text: amount.to_string(),
        color,
    }
}
