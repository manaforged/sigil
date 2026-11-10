use super::Wash;
use super::noise::{fbm, gauss, noise, rand, ridged};
use crate::geometry::smoothstep;
use crate::style::Rgba8;

impl Wash {
    /// The colors the renderer reads: the paper, then each pigment.
    pub(crate) fn palette(&self) -> impl Iterator<Item = Rgba8> + '_ {
        std::iter::once(self.paper).chain(
            self.pigments
                .iter()
                .take(usize::from(self.count.max(1)))
                .copied(),
        )
    }

    pub(crate) fn dark(&self) -> bool {
        let [r, g, b] = [self.paper.r, self.paper.g, self.paper.b].map(f32::from);
        0.0722f32.mul_add(b, 0.7152f32.mul_add(g, 0.2126 * r)) / 255.0 < 0.5
    }

    /// The colour at `(x, y)` pixels from the top-left of an area centred on `centre`, in 0..1 sRGB.
    #[must_use]
    pub(crate) fn shade(&self, x: f32, y: f32, centre: (f32, f32)) -> [f32; 3] {
        let wash = self.normalized();
        let seed = wash.seed;
        let (u, v) = (x / wash.scale, y / wash.scale);
        let wx = 1.1f32.mul_add(
            fbm(u.mul_add(0.8, 3.0), v.mul_add(0.8, 7.0), 3, seed) - 0.5,
            u,
        );
        let wy = 1.1f32.mul_add(
            fbm(u.mul_add(0.8, -5.0), v.mul_add(0.8, 1.0), 3, seed) - 0.5,
            v,
        );
        let body = if wash.band {
            band(
                u - centre.0 / wash.scale,
                v - centre.1 / wash.scale,
                wx,
                wy,
                seed,
            )
        } else {
            smoothstep(0.38, 0.78, fbm(wx * 0.9, wy * 0.9, 5, seed))
        };
        let (amount, fibre) = wash.soak((x, y), body, (wx, wy));
        let hue = ((fbm(wx * 0.6 + 90.0, wy * 0.5, 3, seed) - 0.3) / 0.4).clamp(0.0, 1.0);
        let pigment = ramp(&wash, hue);
        let paper = rgb(wash.paper);
        let dark = wash.dark();
        let mut out = if dark {
            zip_with(paper, pigment, |p, g| {
                (1.0 - p).mul_add(-g.mul_add(-amount, 1.0), 1.0)
            })
        } else {
            zip_with(paper, pigment, |p, g| p * g.mul_add(amount, 1.0 - amount))
        };
        if !dark {
            let fine = rand(x.floor() as i32, y.floor() as i32, seed ^ 0x29);
            let texture = wash
                .grain
                .mul_add(-0.035f32.mul_add(fine, 0.045 * fibre), 1.0);
            out = out.map(|channel| channel * texture);
        }
        speck(&wash, (x, y), body, dark, out)
    }

    fn soak(&self, (x, y): (f32, f32), body: f32, (wx, wy): (f32, f32)) -> (f32, f32) {
        let seed = self.seed;
        let mut amount = pool(
            (body * self.density).clamp(0.0, 1.0),
            self.pools,
            self.edges,
        );
        let ridge = ridged(wx.mul_add(1.4, 31.0), wy.mul_add(1.4, -17.0), seed);
        let inside = smoothstep(0.66, 0.74, ridge);
        let fringe = smoothstep(0.60, 0.66, ridge) * (1.0 - inside);
        let wet = smoothstep(0.05, 0.5, amount);
        amount = amount.mul_add(
            (-0.35 * inside).mul_add(self.edges, 1.0),
            fringe * 0.12 * self.edges * wet,
        );
        let fibre = noise(
            0.8f32.mul_add(x, 0.6 * y) / 7.0,
            0.8f32.mul_add(y, -0.6 * x) / 3.0,
            seed ^ 0x13,
        );
        let grains = 0.6f32.mul_add(
            noise(x / 1.7, y / 1.7, seed ^ 0x51),
            0.4 * noise(x / 5.0, y / 5.0, seed ^ 0x77),
        );
        amount *= (self.granulation * (grains - 0.5) * 1.6).mul_add(wet, 1.0);
        amount *= (self.grain * 0.25).mul_add(fibre - 0.5, 1.0);
        (amount.clamp(0.0, 1.0), fibre)
    }
}

fn rgb(color: Rgba8) -> [f32; 3] {
    [color.r, color.g, color.b].map(|channel| f32::from(channel) / 255.0)
}

fn zip_with(left: [f32; 3], right: [f32; 3], blend: impl Fn(f32, f32) -> f32) -> [f32; 3] {
    let ([l0, l1, l2], [r0, r1, r2]) = (left, right);
    [blend(l0, r0), blend(l1, r1), blend(l2, r2)]
}

fn ramp(wash: &Wash, t: f32) -> [f32; 3] {
    let count = usize::from(wash.count.max(1));
    let [first, ..] = wash.pigments;
    if count == 1 || wash.count == 0 {
        return rgb(first);
    }
    let at = t * (count - 1) as f32;
    let index = (at.floor() as usize).min(count - 2);
    let mix = at - index as f32;
    let Some(&[from, to]) = wash.pigments.get(index..index + 2) else {
        return rgb(first);
    };
    zip_with(rgb(from), rgb(to), |a, b| (b - a).mul_add(mix, a))
}

fn speck(wash: &Wash, (x, y): (f32, f32), body: f32, dark: bool, out: [f32; 3]) -> [f32; 3] {
    if wash.specks <= 0.0 {
        return out;
    }
    let (cx, cy) = ((x / 4.0).floor(), (y / 4.0).floor());
    let seed = wash.seed ^ 0xabc;
    let roll = rand(cx as i32, cy as i32, seed);
    if roll >= wash.specks * 0.05 * (0.35 + body) {
        return out;
    }
    let ox = cx.mul_add(4.0, 4.0 * rand(cx as i32, cy as i32, seed ^ 1));
    let oy = cy.mul_add(4.0, 4.0 * rand(cx as i32, cy as i32, seed ^ 2));
    let radius = 0.7f32.mul_add(rand(cx as i32, cy as i32, seed ^ 3), 0.6);
    let distance = (x - ox).hypot(y - oy);
    let dot = 1.0 - smoothstep(radius - 0.6, radius + 0.6, distance);
    if dark {
        let glow = 0.45f32.mul_add(rand(cx as i32, cy as i32, seed ^ 4), 0.55);
        out.map(|channel| ((1.0 - channel) * dot).mul_add(glow, channel))
    } else {
        let ink = ramp(wash, 1.0).map(|channel| channel * 0.45);
        zip_with(out, ink, |o, k| ((k - o) * dot).mul_add(0.55, o))
    }
}

const UNDERWASH: f32 = 0.7;

fn pool(amount: f32, pools: f32, edges: f32) -> f32 {
    if pools < 1.0 {
        return amount;
    }
    let x = amount * pools;
    let level = x.floor();
    let rest = x - level;
    let settled = (level + smoothstep(0.80, 1.0, rest)) / pools;
    let rim = (1.0 - smoothstep(0.0, 0.16, rest)) * if level >= 1.0 { 1.0 } else { 0.0 };
    (edges * rim)
        .mul_add(0.9 / pools, settled)
        .max(UNDERWASH * amount.sqrt())
}

fn band(u: f32, v: f32, wx: f32, wy: f32, seed: u32) -> f32 {
    let roll = |salt: i32| rand(salt, 0x5eed, seed);
    let flip = if roll(2) > 0.5 {
        std::f32::consts::PI
    } else {
        0.0
    };
    let angle = (roll(1) - 0.5).mul_add(1.8, flip);
    let (sin, cos) = angle.sin_cos();
    let along = u.mul_add(cos, v * sin);
    let curve = ((roll(30) - 0.5) * 0.45 * along).mul_add(
        along,
        (roll(31) - 0.5) * 0.35 * (along * 1.6f32.mul_add(roll(32), 1.2)).sin(),
    );
    let across = (roll(3) - 0.5).mul_add(-0.6, v.mul_add(cos, -(u * sin))) - curve;
    let swell = (fbm(along.mul_add(0.55, 9.0), 2.0, 3, seed) - 0.5)
        .mul_add(3.0, 0.5)
        .clamp(0.0, 1.0);
    let width = 0.42 * 0.4f32.mul_add(roll(6), 0.8) * 0.65f32.mul_add(swell, 0.7);
    let ragged = 0.1f32.mul_add(fbm(wx.mul_add(3.0, 70.0), wy * 3.0, 4, seed) - 0.5, across);
    let main = gauss(ragged / width);
    let patch =
        ((fbm(wx.mul_add(0.9, 140.0), wy.mul_add(0.7, 3.0), 4, seed) - 0.56) * 4.0).clamp(0.0, 1.0);
    let cloud = patch * gauss(across / (width * 3.2)) * (1.0 - main) * 0.9;
    let profile = main.max(cloud);
    let clouds = ((fbm(wx * 1.7, wy * 2.4, 5, seed) - 0.3) * 1.8).clamp(0.0, 1.0);
    let lit = profile * 0.72f32.mul_add(clouds, 0.38);
    let bu = (roll(4) - 0.5).mul_add(-1.8, along) / 0.5;
    let bv = across / 0.24;
    let edge = 1.1f32.mul_add(fbm(wx.mul_add(2.2, 40.0), wy * 2.2, 4, seed), 0.45);
    let bulge = (-(bu * bu + bv * bv) / edge).exp() * 0.65f32.mul_add(swell, 0.35);
    let broken = ((fbm(wx.mul_add(0.9, 11.0), wy * 2.4, 5, seed) - 0.34) * 2.4).clamp(0.0, 1.0);
    let breadth = width * 0.22 + 0.025;
    let rift = gauss((across - 0.02) / breadth) * broken * 0.85;
    ((lit * 0.85 + bulge * 0.95) * 1.15 * 0.8f32.mul_add(-rift, 1.0)).clamp(0.0, 1.0)
}
