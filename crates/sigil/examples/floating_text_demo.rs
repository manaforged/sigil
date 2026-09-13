#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]
#![allow(missing_docs)]

use sigil::prelude::*;
use sigil::style::Rgba8;

fn main() -> sigil::AppResult<()> {
    sigil::launch(("Sigil - floating combat text", 960, 540), build)
}

fn build(cx: &mut Cx) -> NodeId {
    let time = signal(0.0_f32);
    let spawn = signal(FctSpawn::default());

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
            .class(classes!("w-full h-full bg-surface"))
            .child(widget),
    )
}

fn burst_spawn(frame: u32) -> FctSpawn {
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
