#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]
#![allow(clippy::cast_precision_loss)]
#![allow(clippy::imprecise_flops)]
#![allow(clippy::indexing_slicing)]
#![allow(clippy::print_stdout)]
#![allow(clippy::suboptimal_flops)]

use proptest::prelude::*;

use sigil_icons::{QuadCurve, cubic_to_quadratics};

#[allow(dead_code)]
const PROPTEST_CASES: u32 = 32;

const COORD_MIN: f32 = -1024.0;
const COORD_MAX: f32 = 1024.0;

const TOL_MIN: f32 = 0.01;
const TOL_MAX: f32 = 10.0;

const EPSILON: f32 = 1e-3;

const ERROR_BOUND_MULTIPLIER: f32 = 4.0;

#[inline]
fn point_distance(p1: (f32, f32), p2: (f32, f32)) -> f32 {
    let dx = p1.0 - p2.0;
    let dy = p1.1 - p2.1;
    (dx * dx + dy * dy).sqrt()
}

#[inline]
fn eval_quad(quad: &QuadCurve, t: f32) -> (f32, f32) {
    let t = t.clamp(0.0, 1.0);
    let mt = 1.0 - t;
    let x = mt * mt * quad.start.0 + 2.0 * mt * t * quad.control.0 + t * t * quad.end.0;
    let y = mt * mt * quad.start.1 + 2.0 * mt * t * quad.control.1 + t * t * quad.end.1;
    (x, y)
}

#[inline]
fn eval_cubic(
    p0: (f32, f32),
    p1: (f32, f32),
    p2: (f32, f32),
    p3: (f32, f32),
    t: f32,
) -> (f32, f32) {
    let t = t.clamp(0.0, 1.0);
    let mt = 1.0 - t;
    let mt2 = mt * mt;
    let mt3 = mt2 * mt;
    let t2 = t * t;
    let t3 = t2 * t;

    let x = mt3 * p0.0 + 3.0 * mt2 * t * p1.0 + 3.0 * mt * t2 * p2.0 + t3 * p3.0;
    let y = mt3 * p0.1 + 3.0 * mt2 * t * p1.1 + 3.0 * mt * t2 * p2.1 + t3 * p3.1;
    (x, y)
}

fn closest_cubic_distance(
    p0: (f32, f32),
    p1: (f32, f32),
    p2: (f32, f32),
    p3: (f32, f32),
    point: (f32, f32),
) -> f32 {
    const SAMPLES: usize = 65_536;
    let mut min_dist = f32::INFINITY;

    for i in 0..=SAMPLES {
        let t = i as f32 / SAMPLES as f32;
        let cubic_point = eval_cubic(p0, p1, p2, p3, t);
        let dist = point_distance(point, cubic_point);
        if dist < min_dist {
            min_dist = dist;
        }
    }

    min_dist
}

proptest! {
    #[test]
    fn prop_endpoints_preserved(
        coords in prop::array::uniform4(COORD_MIN..=COORD_MAX)
    ) {
        let p0 = (coords[0], coords[1]);
        let p1 = (coords[0] + 1.0, coords[1] + 1.0);
        let p2 = (coords[0] + 2.0, coords[1] + 2.0);
        let p3 = (coords[2], coords[3]);
        let tolerance = 1.0;

        let quads = cubic_to_quadratics(p0, p1, p2, p3, tolerance);

        prop_assert!(
            !quads.is_empty() || (point_distance(p0, p3) < 1e-6),
            "Expected quads or degenerate cubic; got empty result"
        );

        if !quads.is_empty() {
            let first_start = quads[0].start;
            prop_assert!(
                point_distance(first_start, p0) < EPSILON,
                "First quad start {first_start:?} != cubic p0 {p0:?}"
            );

            let last_end = quads[quads.len() - 1].end;
            prop_assert!(
                point_distance(last_end, p3) < EPSILON,
                "Last quad end {last_end:?} != cubic p3 {p3:?}"
            );
        }
    }
}

proptest! {
    #[test]
    fn prop_chain_continuity(
        coords in prop::array::uniform4(COORD_MIN..=COORD_MAX),
        tolerance in TOL_MIN..=TOL_MAX
    ) {
        let p0 = (coords[0], coords[1]);
        let p1 = (coords[0] + 1.0, coords[1] + 1.0);
        let p2 = (coords[0] + 2.0, coords[1] + 2.0);
        let p3 = (coords[2], coords[3]);

        let quads = cubic_to_quadratics(p0, p1, p2, p3, tolerance);

        for i in 0..quads.len().saturating_sub(1) {
            let end_i = quads[i].end;
            let start_next = quads[i + 1].start;
            prop_assert!(
                point_distance(end_i, start_next) < EPSILON,
                "Discontinuity at quad[{i}]: end {end_i:?} != quad[{}].start {start_next:?}",
                i + 1
            );
        }
    }
}

proptest! {
    #[test]
    fn prop_approximation_error_bounded(
        coords in prop::array::uniform4(COORD_MIN..=COORD_MAX),
        tolerance in TOL_MIN..=TOL_MAX
    ) {
        let p0 = (coords[0], coords[1]);
        let p1 = (coords[0] + 1.0, coords[1] + 1.0);
        let p2 = (coords[0] + 2.0, coords[1] + 2.0);
        let p3 = (coords[2], coords[3]);

        let quads = cubic_to_quadratics(p0, p1, p2, p3, tolerance);

        let eval_ts = [0.0, 0.25, 0.5, 0.75, 1.0];

        for (quad_idx, quad) in quads.iter().enumerate() {
            for &t in &eval_ts {
                let quad_point = eval_quad(quad, t);
                let closest_dist =
                    closest_cubic_distance(p0, p1, p2, p3, quad_point);

                let bound = tolerance * ERROR_BOUND_MULTIPLIER;
                prop_assert!(
                    closest_dist <= bound || closest_dist.is_nan(),
                    "Quad[{quad_idx}] at t={t}: distance {closest_dist:.6} > bound {bound:.6}"
                );
            }
        }
    }
}

proptest! {
    #[test]
    fn prop_tolerance_monotone(
        coords in prop::array::uniform4(COORD_MIN..=COORD_MAX)
    ) {
        let p0 = (coords[0], coords[1]);
        let p1 = (coords[0] + 1.0, coords[1] + 1.0);
        let p2 = (coords[0] + 2.0, coords[1] + 2.0);
        let p3 = (coords[2], coords[3]);

        let tight_tol = 0.1;
        let loose_tol = 1.0;

        let tight_quads = cubic_to_quadratics(p0, p1, p2, p3, tight_tol);
        let loose_quads = cubic_to_quadratics(p0, p1, p2, p3, loose_tol);

        prop_assert!(
            tight_quads.len() >= loose_quads.len(),
            "Tighter tolerance ({tight_tol}) produced fewer quads ({}) than looser tolerance ({loose_tol}) with {} quads",
            tight_quads.len(),
            loose_quads.len()
        );
    }
}

proptest! {
    #[test]
    fn prop_no_spurious_subdivision_on_line(
        p0_x in COORD_MIN..=COORD_MAX,
        p0_y in COORD_MIN..=COORD_MAX,
        p3_x in COORD_MIN..=COORD_MAX,
        p3_y in COORD_MIN..=COORD_MAX,
        t1 in 0.0f32..=1.0,
        t2 in 0.0f32..=1.0
    ) {
        let p0 = (p0_x, p0_y);
        let p3 = (p3_x, p3_y);

        let p1 = (
            p0.0 + t1 * (p3.0 - p0.0),
            p0.1 + t1 * (p3.1 - p0.1),
        );
        let p2 = (
            p0.0 + t2 * (p3.0 - p0.0),
            p0.1 + t2 * (p3.1 - p0.1),
        );

        let tolerance = 1.0;
        let quads = cubic_to_quadratics(p0, p1, p2, p3, tolerance);

        prop_assert!(
            quads.len() <= 8,
            "Straight-line cubic produced {count} quads (expected ≤ 8)",
            count = quads.len()
        );
    }
}

proptest! {
    #[test]
    fn prop_degenerate_all_identical(
        pt in prop::array::uniform2(COORD_MIN..=COORD_MAX)
    ) {
        let p = (pt[0], pt[1]);
        let quads = cubic_to_quadratics(p, p, p, p, 1.0);

        prop_assert!(
            quads.is_empty() || quads.len() == 1,
            "All-identical cubic produced {} quads (expected 0 or 1)",
            quads.len()
        );

        for quad in &quads {
            prop_assert!(!quad.start.0.is_nan() && !quad.start.1.is_nan());
            prop_assert!(!quad.control.0.is_nan() && !quad.control.1.is_nan());
            prop_assert!(!quad.end.0.is_nan() && !quad.end.1.is_nan());
        }
    }
}

proptest! {
    #[test]
    fn prop_degenerate_p0_equals_p3(
        p0 in prop::array::uniform2(COORD_MIN..=COORD_MAX),
        p1 in prop::array::uniform2(COORD_MIN..=COORD_MAX),
        p2 in prop::array::uniform2(COORD_MIN..=COORD_MAX)
    ) {
        let p0_pt = (p0[0], p0[1]);
        let p1_pt = (p1[0], p1[1]);
        let p2_pt = (p2[0], p2[1]);

        let quads = cubic_to_quadratics(p0_pt, p1_pt, p2_pt, p0_pt, 1.0);

        prop_assert!(
            quads.len() < 1000,
            "p0==p3 degenerate produced {} quads (sanity: should be <<1000)",
            quads.len()
        );

        for quad in &quads {
            prop_assert!(!quad.start.0.is_nan() && !quad.start.1.is_nan());
            prop_assert!(!quad.control.0.is_nan() && !quad.control.1.is_nan());
            prop_assert!(!quad.end.0.is_nan() && !quad.end.1.is_nan());
        }
    }
}
