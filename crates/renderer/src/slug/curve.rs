//! Quadratic Bézier primitives for the Slug band preprocessor.
#![allow(clippy::suboptimal_flops)] // Bézier polynomial form is clearer than mul_add chains

/// A quadratic Bézier arc `p0 → p1 → p2`.
///
/// Control point `p1` is *off-curve* (the "handle"). `p0` and `p2` are on-curve
/// endpoints. All coordinates are in the caller's coordinate space (typically
/// unscaled font design units before normalisation, or normalised \[0,1\] after).
#[derive(Copy, Clone, Debug, PartialEq)]
pub(super) struct QuadBez {
    /// Start (on-curve).
    pub p0: [f32; 2],
    /// Control (off-curve handle).
    pub p1: [f32; 2],
    /// End (on-curve).
    pub p2: [f32; 2],
}

/// Convert a straight line segment `p0 → p2` to a degenerate quadratic.
///
/// The control point is placed at the midpoint so the quadratic exactly
/// traces the line (B(0.5) = midpoint by the quadratic formula).
#[inline]
pub(super) fn line_to_quad(p0: [f32; 2], p2: [f32; 2]) -> QuadBez {
    QuadBez {
        p0,
        p1: [lerp1(p0[0], p2[0], 0.5), lerp1(p0[1], p2[1], 0.5)],
        p2,
    }
}

/// Convert a cubic Bézier `(p0, p1, p2, p3)` to two quadratic approximations.
///
/// Splits the cubic at `t = 0.5` using de Casteljau, then applies the
/// L₂-optimal degree-reduction formula to each half:
///
/// ```text
/// q_ctrl = (–c0 + 3·c1 + 3·c2 – c3) / 4
/// ```
///
/// This gives sub-pixel accuracy for typical glyph outlines at display sizes.
#[allow(clippy::similar_names)]
pub(super) fn cubic_to_quads(
    p0: [f32; 2],
    p1: [f32; 2],
    p2: [f32; 2],
    p3: [f32; 2],
) -> [QuadBez; 2] {
    // de Casteljau split at t = 0.5
    let p01 = lerp2(p0, p1, 0.5);
    let p12 = lerp2(p1, p2, 0.5);
    let p23 = lerp2(p2, p3, 0.5);
    let p012 = lerp2(p01, p12, 0.5);
    let p123 = lerp2(p12, p23, 0.5);
    let mid = lerp2(p012, p123, 0.5);

    // L₂-optimal quad control for each half cubic.
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
