use crate::style::Rgba8;
use crate::style::scale::rgb;

pub(super) fn shift(c: Rgba8, delta: i16) -> Rgba8 {
    let nudge = |channel: u8| -> u8 {
        #[allow(
            clippy::cast_sign_loss,
            reason = "Clamped to 0..=255 before cast; no sign to lose."
        )]
        {
            i16::from(channel).saturating_add(delta).clamp(0, 255) as u8
        }
    };
    Rgba8 {
        r: nudge(c.r),
        g: nudge(c.g),
        b: nudge(c.b),
        a: c.a,
    }
}

pub(super) fn ramp_accent(surface: Rgba8, solid: Rgba8) -> crate::style::scale::ColorScale {
    let mut steps = [solid; 12];
    for (step, t) in steps.iter_mut().zip(1..=9u8).take(8) {
        *step = alpha_mix(surface, solid, f32::from(t) / 9.0);
    }
    steps[8] = solid;
    steps[9] = alpha_mix(solid, rgb(255, 255, 255), 0.18);
    steps[10] = alpha_mix(solid, rgb(255, 255, 255), 0.36);
    steps[11] = alpha_mix(solid, rgb(255, 255, 255), 0.55);
    crate::style::scale::ColorScale::from_steps(steps).with_on_solid(surface)
}

pub(super) fn alpha_mix(a: Rgba8, b: Rgba8, t: f32) -> Rgba8 {
    let t = t.clamp(0.0, 1.0);
    let blend = |x: u8, y: u8| -> u8 {
        let xf = f32::from(x);
        let yf = f32::from(y);
        let mixed = (1.0 - t).mul_add(xf, t * yf).round().clamp(0.0, 255.0);
        #[allow(
            clippy::cast_possible_truncation,
            clippy::cast_sign_loss,
            reason = "Value clamped to 0.0..=255.0 before cast — bounded."
        )]
        {
            mixed as u8
        }
    };
    Rgba8 {
        r: blend(a.r, b.r),
        g: blend(a.g, b.g),
        b: blend(a.b, b.b),
        a: a.a,
    }
}
