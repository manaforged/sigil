use crate::style::Color;
use crate::style::srgb::{byte, decode_rgb, encode, unit};

const TO_LMS: [[f32; 3]; 3] = [
    [0.412_221_46, 0.536_332_55, 0.051_445_995],
    [0.211_903_5, 0.680_699_5, 0.107_396_96],
    [0.088_302_46, 0.281_718_85, 0.629_978_7],
];
const LMS_TO_LAB: [[f32; 3]; 3] = [
    [0.210_454_26, 0.793_617_8, -0.004_072_047],
    [1.977_998_5, -2.428_592_2, 0.450_593_7],
    [0.025_904_037, 0.782_771_77, -0.808_675_77],
];
const LAB_TO_LMS: [[f32; 3]; 3] = [
    [1.0, 0.396_337_78, 0.215_803_76],
    [1.0, -0.105_561_346, -0.063_854_17],
    [1.0, -0.089_484_18, -1.291_485_5],
];
const FROM_LMS: [[f32; 3]; 3] = [
    [4.076_741_7, -3.307_711_6, 0.230_969_94],
    [-1.268_438, 2.609_757_4, -0.341_319_38],
    [-0.004_196_086_3, -0.703_418_6, 1.707_614_7],
];

fn apply(matrix: [[f32; 3]; 3], [x, y, z]: [f32; 3]) -> [f32; 3] {
    matrix.map(|[a, b, c]| a.mul_add(x, b.mul_add(y, c * z)))
}

fn to_oklab(linear: [f32; 3]) -> [f32; 3] {
    apply(LMS_TO_LAB, apply(TO_LMS, linear).map(f32::cbrt))
}

fn from_oklab(lab: [f32; 3]) -> [f32; 3] {
    apply(FROM_LMS, apply(LAB_TO_LMS, lab).map(|v| v * v * v))
}

/// Mixes `from` toward `to` by `t` in premultiplied `OKLab`, as CSS `color-mix(in oklab, ..)` does.
pub(crate) fn mix(from: Color, to: Color, t: f32) -> Color {
    let t = if t.is_nan() { 0.0 } else { t.clamp(0.0, 1.0) };
    let (alpha_a, alpha_b) = (unit(from.a), unit(to.a));
    let alpha = (alpha_b - alpha_a).mul_add(t, alpha_a);
    if alpha <= 0.0 {
        return Color {
            r: 0,
            g: 0,
            b: 0,
            a: 0,
        };
    }
    let lab_a = to_oklab(decode_rgb(from)).map(|v| v * alpha_a);
    let lab_b = to_oklab(decode_rgb(to)).map(|v| v * alpha_b);
    let mut lab = lab_a;
    for (out, (a, b)) in lab.iter_mut().zip(lab_a.into_iter().zip(lab_b)) {
        *out = (b - a).mul_add(t, a) / alpha;
    }
    let [r, g, b] = from_oklab(lab).map(|v| byte(encode(v) * 255.0));
    Color {
        r,
        g,
        b,
        a: byte(alpha * 255.0),
    }
}
