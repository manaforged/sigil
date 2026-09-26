use skrifa::outline::OutlinePen;

pub(crate) struct Oblique<'a, P: OutlinePen> {
    pen: &'a mut P,
    skew: f32,
}

impl<'a, P: OutlinePen> Oblique<'a, P> {
    pub(crate) const fn new(pen: &'a mut P, skew: f32) -> Self {
        Self { pen, skew }
    }

    #[allow(
        clippy::missing_const_for_fn,
        reason = "const f32::mul_add is newer than the 1.90 MSRV"
    )]
    fn x(&self, x: f32, y: f32) -> f32 {
        self.skew.mul_add(y, x)
    }
}

impl<P: OutlinePen> OutlinePen for Oblique<'_, P> {
    fn move_to(&mut self, x: f32, y: f32) {
        let x = self.x(x, y);
        self.pen.move_to(x, y);
    }

    fn line_to(&mut self, x: f32, y: f32) {
        let x = self.x(x, y);
        self.pen.line_to(x, y);
    }

    fn quad_to(&mut self, cx: f32, cy: f32, x: f32, y: f32) {
        let (cx, x) = (self.x(cx, cy), self.x(x, y));
        self.pen.quad_to(cx, cy, x, y);
    }

    fn curve_to(&mut self, c0x: f32, c0y: f32, c1x: f32, c1y: f32, x: f32, y: f32) {
        let (c0x, c1x, x) = (self.x(c0x, c0y), self.x(c1x, c1y), self.x(x, y));
        self.pen.curve_to(c0x, c0y, c1x, c1y, x, y);
    }

    fn close(&mut self) {
        self.pen.close();
    }
}
