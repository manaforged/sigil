#![allow(clippy::suboptimal_flops)] // Bézier polynomial form is clearer than mul_add chains

#[derive(Copy, Clone, Debug, PartialEq)]
pub(super) struct QuadBez {
    pub p0: [f32; 2],
    pub p1: [f32; 2],
    pub p2: [f32; 2],
}

#[inline]
pub(super) fn line_to_quad(p0: [f32; 2], p2: [f32; 2]) -> QuadBez {
    QuadBez {
        p0,
        p1: [lerp1(p0[0], p2[0], 0.5), lerp1(p0[1], p2[1], 0.5)],
        p2,
    }
}

#[allow(
    clippy::similar_names,
    reason = "the paired names are the domain vocabulary"
)]
pub(super) fn cubic_to_quads(
    p0: [f32; 2],
    p1: [f32; 2],
    p2: [f32; 2],
    p3: [f32; 2],
) -> [QuadBez; 2] {
    let p01 = lerp2(p0, p1, 0.5);
    let p12 = lerp2(p1, p2, 0.5);
    let p23 = lerp2(p2, p3, 0.5);
    let p012 = lerp2(p01, p12, 0.5);
    let p123 = lerp2(p12, p23, 0.5);
    let mid = lerp2(p012, p123, 0.5);

    let q1_a = [
        (-p0[0] + 3.0 * p01[0] + 3.0 * p012[0] - mid[0]) * 0.25,
        (-p0[1] + 3.0 * p01[1] + 3.0 * p012[1] - mid[1]) * 0.25,
    ];
    let q1_b = [
        (-mid[0] + 3.0 * p123[0] + 3.0 * p23[0] - p3[0]) * 0.25,
        (-mid[1] + 3.0 * p123[1] + 3.0 * p23[1] - p3[1]) * 0.25,
    ];

    [
        QuadBez {
            p0,
            p1: q1_a,
            p2: mid,
        },
        QuadBez {
            p0: mid,
            p1: q1_b,
            p2: p3,
        },
    ]
}

#[inline]
fn lerp1(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

#[inline]
fn lerp2(a: [f32; 2], b: [f32; 2], t: f32) -> [f32; 2] {
    [lerp1(a[0], b[0], t), lerp1(a[1], b[1], t)]
}
