#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CubicBezier {
    x1: f32,
    y1: f32,
    x2: f32,
    y2: f32,
}

impl CubicBezier {
    #[must_use]
    pub fn new(x1: f32, y1: f32, x2: f32, y2: f32) -> Self {
        assert!((0.0..=1.0).contains(&x1) && (0.0..=1.0).contains(&x2));
        assert!(y1.is_finite() && y2.is_finite());
        Self { x1, y1, x2, y2 }
    }

    pub(super) fn apply(self, x: f32) -> f32 {
        if x <= 0.0 || x >= 1.0 {
            return x;
        }
        let (mut low, mut high) = (0.0, 1.0);
        for _ in 0..20 {
            let t = f32::midpoint(low, high);
            if coordinate(t, self.x1, self.x2) < x {
                low = t;
            } else {
                high = t;
            }
        }
        coordinate(f32::midpoint(low, high), self.y1, self.y2)
    }
}

fn coordinate(t: f32, first: f32, second: f32) -> f32 {
    let inverse = 1.0 - t;
    (3.0 * inverse * inverse * t).mul_add(first, (3.0 * inverse * t * t).mul_add(second, t * t * t))
}
