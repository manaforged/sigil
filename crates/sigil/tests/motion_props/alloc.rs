use std::f32::consts::FRAC_PI_2;
use std::time::Duration;

use sigil::animation::{Easing, Keyframes, Property};
use sigil::app::FrameHooks;
use sigil::prelude::*;
use sigil::reactive::Owner;

use super::{BLACK, BLUE, VIEW, WHITE, gradient_square, mount, shadow, start};

const FRAMES: u32 = 32;

fn styled() -> NodeBuilder {
    gradient_square()
        .blur(1.0)
        .brightness(1.2)
        .hue_rotate(10.0)
        .drop_shadow(shadow(2.0, 1.0, BLACK))
        .rotate_y(10.0)
        .translate_z(5.0)
}

fn opacity_only() -> Keyframes {
    Keyframes::new(Duration::from_secs(4))
        .ease(Easing::Linear)
        .keyframe(0.0, Property::Opacity(0.2))
        .keyframe(1.0, Property::Opacity(0.9))
}

fn every_property() -> Keyframes {
    Keyframes::new(Duration::from_secs(4))
        .ease(Easing::IN_OUT)
        .keyframe(0.0, Property::Opacity(0.2))
        .keyframe(1.0, Property::Opacity(0.9))
        .keyframe(0.0, Property::Blur(0.5))
        .keyframe(1.0, Property::Blur(6.0))
        .keyframe(1.0, Property::Brightness(2.0))
        .keyframe(1.0, Property::HueRotate(200.0))
        .keyframe(1.0, Property::DropShadow(shadow(8.0, 4.0, WHITE)))
        .keyframe(0.0, Property::GradientFrom(WHITE.into()))
        .keyframe(1.0, Property::GradientTo(BLUE.into()))
        .keyframe(1.0, Property::RotateY(FRAC_PI_2))
        .keyframe(1.0, Property::TranslateZ(60.0))
}

fn frame_calls(keyframes: Keyframes) -> u64 {
    Owner::scope("props-alloc", || {
        let flag = signal(false);
        let mut ui = mount(styled().paint_when(flag, move |state| state.animate(keyframes)));
        start(&mut ui, flag);
        let mut step = || {
            let _ = ui.host().frame(VIEW, 1.0 / 60.0, FrameHooks::default());
            drop(ui.host().emit(VIEW));
        };
        for _ in 0..8 {
            step();
        }
        allocation_counter::measure(|| {
            for _ in 0..FRAMES {
                step();
            }
        })
        .count_total
    })
}

#[test]
fn animating_every_paint_property_allocates_no_more_per_frame_than_opacity() {
    let _keyframes = crate::serial::keyframes();
    let baseline = frame_calls(opacity_only());
    let animated = frame_calls(every_property());
    println!("{FRAMES} frames: opacity {baseline} allocations, every property {animated}");
    assert!(
        animated <= baseline,
        "animating filters, stops, and 3D added {} allocations over {FRAMES} frames",
        animated.saturating_sub(baseline)
    );
}
