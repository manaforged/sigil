#![allow(
    clippy::cast_possible_wrap,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    reason = "noise lattice coordinates are floored floats that fit in i32"
)]

use crate::style::Rgba8;

/// The most pigments one wash carries.
pub const MAX_PIGMENTS: usize = 4;
const SEED_MASK: u32 = 0x00ff_ffff;

/// A watercolour wash: pigment pooled over paper, or light over a night sky when the paper is dark.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Wash {
    pub(crate) paper: Rgba8,
    pub(crate) pigments: [Rgba8; MAX_PIGMENTS],
    pub(crate) count: u8,
    pub(crate) seed: u32,
    pub(crate) scale: f32,
    pub(crate) density: f32,
    pub(crate) pools: f32,
    pub(crate) edges: f32,
    pub(crate) granulation: f32,
    pub(crate) grain: f32,
    pub(crate) specks: f32,
    pub(crate) band: bool,
}

impl Wash {
    /// A wash on `paper` with no pigment yet; the paper's alpha sets the opacity of the whole wash.
    #[must_use]
    pub const fn new(paper: Rgba8) -> Self {
        Self {
            paper,
            pigments: [paper; MAX_PIGMENTS],
            count: 0,
            seed: 7,
            scale: 260.0,
            density: 1.0,
            pools: 6.0,
            edges: 0.6,
            granulation: 0.5,
            grain: 0.5,
            specks: 0.0,
            band: false,
        }
    }

    /// Adds a pigment, as it looks at full strength; past four it replaces the last.
    #[must_use]
    pub const fn pigment(mut self, color: Rgba8) -> Self {
        let at = if (self.count as usize) < MAX_PIGMENTS {
            self.count += 1;
            self.count as usize - 1
        } else {
            MAX_PIGMENTS - 1
        };
        self.pigments[at] = color;
        self
    }

    /// Sets the seed that picks the shapes.
    #[must_use]
    pub const fn seed(mut self, seed: u32) -> Self {
        self.seed = seed & SEED_MASK;
        self
    }

    /// Sets the size of the largest shapes in pixels.
    #[must_use]
    pub const fn scale(mut self, pixels: f32) -> Self {
        self.scale = pixels;
        self
    }

    /// Sets how much pigment the paper takes, from 0 to 2.
    #[must_use]
    pub const fn density(mut self, density: f32) -> Self {
        self.density = density;
        self
    }

    /// Sets how many flat pools the pigment settles into; 0 keeps it smooth.
    #[must_use]
    pub const fn pools(mut self, pools: u8) -> Self {
        self.pools = pools as f32;
        self
    }

    /// Sets how dark pigment dries along the edge of each pool, from 0 to 1.
    #[must_use]
    pub const fn edges(mut self, edges: f32) -> Self {
        self.edges = edges;
        self
    }

    /// Sets how much pigment clumps into fine grains, from 0 to 1.
    #[must_use]
    pub const fn granulation(mut self, granulation: f32) -> Self {
        self.granulation = granulation;
        self
    }

    /// Sets how strongly the paper fibres show, from 0 to 1.
    #[must_use]
    pub const fn grain(mut self, grain: f32) -> Self {
        self.grain = grain;
        self
    }

    /// Sets the density of ink specks on light paper or stars on dark paper, from 0 to 1.
    #[must_use]
    pub const fn specks(mut self, specks: f32) -> Self {
        self.specks = specks;
        self
    }

    /// Gathers the pigment into one drifting band across the area, like a galaxy.
    #[must_use]
    pub const fn band(mut self, band: bool) -> Self {
        self.band = band;
        self
    }

    /// Returns this wash with its values in range.
    #[must_use]
    pub const fn normalized(mut self) -> Self {
        self.scale = if self.scale.is_finite() { self.scale.max(8.0) } else { 260.0 };
        self.density = finite_clamp(self.density, 0.0, 2.0);
        self.pools = finite_clamp(self.pools, 0.0, 16.0);
        self.edges = finite_clamp(self.edges, 0.0, 1.0);
        self.granulation = finite_clamp(self.granulation, 0.0, 1.0);
        self.grain = finite_clamp(self.grain, 0.0, 1.0);
        self.specks = finite_clamp(self.specks, 0.0, 1.0);
        self
    }

    /// The colors the renderer reads: the paper, then each pigment.
    pub(crate) fn palette(&self) -> impl Iterator<Item = Rgba8> + '_ {
        std::iter::once(self.paper).chain(self.pigments.iter().take(usize::from(self.count.max(1))).copied())
    }

    pub(crate) fn dark(&self) -> bool {
        let [r, g, b] = [self.paper.r, self.paper.g, self.paper.b].map(f32::from);
        (0.2126 * r + 0.7152 * g + 0.0722 * b) / 255.0 < 0.5
    }

    /// The colour at `(x, y)` pixels from the top-left of the area, in 0..1 sRGB.
    #[must_use]
    pub(crate) fn shade(&self, x: f32, y: f32) -> [f32; 3] {
        let wash = self.normalized();
        let seed = wash.seed;
        let (u, v) = (x / wash.scale, y / wash.scale);
        let wx = u + 1.1 * (fbm(u * 0.8 + 3.0, v * 0.8 + 7.0, 3, seed) - 0.5);
        let wy = v + 1.1 * (fbm(u * 0.8 - 5.0, v * 0.8 + 1.0, 3, seed) - 0.5);
        let body = if wash.band {
            band(u, v, wx, wy, seed)
        } else {
            smoothstep(0.38, 0.78, fbm(wx * 0.9, wy * 0.9, 5, seed))
        };
        let mut amount = pool((body * wash.density).clamp(0.0, 1.0), wash.pools, wash.edges);
        let ridge = ridged(wx * 1.4 + 31.0, wy * 1.4 - 17.0, seed);
        let inside = smoothstep(0.66, 0.74, ridge);
        let fringe = smoothstep(0.60, 0.66, ridge) * (1.0 - inside);
        let wet = smoothstep(0.05, 0.5, amount);
        amount = amount * (1.0 - 0.35 * inside * wash.edges) + fringe * 0.12 * wash.edges * wet;
        let fibre = noise(x / 9.0, y / 2.2, seed ^ 0x13);
        let grains = 0.6 * noise(x / 1.7, y / 1.7, seed ^ 0x51) + 0.4 * noise(x / 5.0, y / 5.0, seed ^ 0x77);
        amount *= 1.0 + wash.granulation * (grains - 0.5) * 1.6 * wet;
        amount *= 1.0 + wash.grain * 0.25 * (fibre - 0.5);
        let amount = amount.clamp(0.0, 1.0);
        let hue = ((fbm(wx * 0.6 + 90.0, wy * 0.5, 3, seed) - 0.3) / 0.4).clamp(0.0, 1.0);
        let pigment = ramp(&wash, hue);
        let paper = rgb(wash.paper);
        let dark = wash.dark();
        let mut out = if dark {
            std::array::from_fn(|i| 1.0 - (1.0 - paper[i]) * (1.0 - pigment[i] * amount))
        } else {
            std::array::from_fn(|i| paper[i] * (1.0 - amount + pigment[i] * amount))
        };
        if !dark {
            let fine = rand(x.floor() as i32, y.floor() as i32, seed ^ 0x29);
            let texture = 1.0 - wash.grain * (0.035 * fine + 0.045 * fibre);
            out = out.map(|channel| channel * texture);
        }
        speck(&wash, x, y, body, dark, out)
    }
}

const fn finite_clamp(value: f32, lo: f32, hi: f32) -> f32 {
    if value.is_finite() { value.clamp(lo, hi) } else { lo }
}

fn rgb(color: Rgba8) -> [f32; 3] {
    [color.r, color.g, color.b].map(|channel| f32::from(channel) / 255.0)
}

fn ramp(wash: &Wash, t: f32) -> [f32; 3] {
    let count = usize::from(wash.count.max(1));
    if count == 1 || wash.count == 0 {
        return rgb(wash.pigments[0]);
    }
    let at = t * (count - 1) as f32;
    let index = (at.floor() as usize).min(count - 2);
    let mix = at - index as f32;
    let (a, b) = (rgb(wash.pigments[index]), rgb(wash.pigments[index + 1]));
    std::array::from_fn(|i| a[i] + (b[i] - a[i]) * mix)
}

fn speck(wash: &Wash, x: f32, y: f32, body: f32, dark: bool, out: [f32; 3]) -> [f32; 3] {
    if wash.specks <= 0.0 {
        return out;
    }
    let (cx, cy) = ((x / 4.0).floor(), (y / 4.0).floor());
    let seed = wash.seed ^ 0xabc;
    let roll = rand(cx as i32, cy as i32, seed);
    if roll >= wash.specks * 0.05 * (0.35 + body) {
        return out;
    }
    let ox = cx * 4.0 + 4.0 * rand(cx as i32, cy as i32, seed ^ 1);
    let oy = cy * 4.0 + 4.0 * rand(cx as i32, cy as i32, seed ^ 2);
    let radius = 0.6 + 0.7 * rand(cx as i32, cy as i32, seed ^ 3);
    let distance = (x - ox).hypot(y - oy);
    let dot = 1.0 - smoothstep(radius - 0.6, radius + 0.6, distance);
    if dark {
        let glow = 0.55 + 0.45 * rand(cx as i32, cy as i32, seed ^ 4);
        out.map(|channel| channel + (1.0 - channel) * dot * glow)
    } else {
        let ink = ramp(wash, 1.0).map(|channel| channel * 0.45);
        std::array::from_fn(|i| out[i] + (ink[i] - out[i]) * dot * 0.55)
    }
}

fn pool(amount: f32, pools: f32, edges: f32) -> f32 {
    if pools < 1.0 {
        return amount;
    }
    let x = amount * pools;
    let level = x.floor();
    let rest = x - level;
    let settled = (level + smoothstep(0.80, 1.0, rest)) / pools;
    let rim = (1.0 - smoothstep(0.0, 0.16, rest)) * if level >= 1.0 { 1.0 } else { 0.0 };
    settled + edges * rim * (0.9 / pools)
}

fn band(u: f32, v: f32, wx: f32, wy: f32, seed: u32) -> f32 {
    let roll = |salt: i32| rand(salt, 0x5eed, seed);
    let flip = if roll(2) > 0.5 { std::f32::consts::PI } else { 0.0 };
    let angle = (roll(1) - 0.5) * 1.8 + flip;
    let (sin, cos) = angle.sin_cos();
    let along = u * cos + v * sin;
    let curve = (roll(30) - 0.5) * 0.45 * along * along + (roll(31) - 0.5) * 0.35 * (along * (1.2 + 1.6 * roll(32))).sin();
    let across = v * cos - u * sin - (roll(3) - 0.5) * 0.6 - curve;
    let swell = ((fbm(along * 0.55 + 9.0, 2.0, 3, seed) - 0.5) * 3.0 + 0.5).clamp(0.0, 1.0);
    let width = 0.42 * (0.8 + 0.4 * roll(6)) * (0.7 + 0.65 * swell);
    let ragged = across + 0.1 * (fbm(wx * 3.0 + 70.0, wy * 3.0, 4, seed) - 0.5);
    let main = gauss(ragged / width);
    let patch = ((fbm(wx * 0.9 + 140.0, wy * 0.7 + 3.0, 4, seed) - 0.56) * 4.0).clamp(0.0, 1.0);
    let cloud = patch * gauss(across / (width * 3.2)) * (1.0 - main) * 0.9;
    let profile = main.max(cloud);
    let clouds = ((fbm(wx * 1.7, wy * 2.4, 5, seed) - 0.3) * 1.8).clamp(0.0, 1.0);
    let lit = profile * (0.38 + 0.72 * clouds);
    let bu = (along - (roll(4) - 0.5) * 1.8) / 0.5;
    let bv = across / 0.24;
    let edge = 0.45 + 1.1 * fbm(wx * 2.2 + 40.0, wy * 2.2, 4, seed);
    let bulge = (-(bu * bu + bv * bv) / edge).exp() * (0.35 + 0.65 * swell);
    let broken = ((fbm(wx * 0.9 + 11.0, wy * 2.4, 5, seed) - 0.34) * 2.4).clamp(0.0, 1.0);
    let breadth = width * 0.22 + 0.025;
    let rift = gauss((across - 0.02) / breadth) * broken * 0.85;
    ((lit * 0.85 + bulge * 0.95) * 1.15 * (1.0 - 0.8 * rift)).clamp(0.0, 1.0)
}

fn gauss(x: f32) -> f32 {
    (-x * x).exp()
}

fn smoothstep(lo: f32, hi: f32, x: f32) -> f32 {
    let t = ((x - lo) / (hi - lo)).clamp(0.0, 1.0);
    t * t * 2.0f32.mul_add(-t, 3.0)
}

fn hash(x: i32, y: i32, seed: u32) -> u32 {
    let mut n = (x as u32).wrapping_mul(0x9E37_79B1)
        ^ (y as u32).wrapping_mul(0x85EB_CA77)
        ^ seed.wrapping_mul(0xC2B2_AE3D);
    n ^= n >> 16;
    n = n.wrapping_mul(0x7FEB_352D);
    n ^= n >> 15;
    n = n.wrapping_mul(0x846C_A68B);
    n ^ (n >> 16)
}

fn rand(x: i32, y: i32, seed: u32) -> f32 {
    (hash(x, y, seed) >> 8) as f32 / 16_777_216.0
}

fn noise(x: f32, y: f32, seed: u32) -> f32 {
    let (cx, cy) = (x.floor(), y.floor());
    let (fx, fy) = (x - cx, y - cy);
    let (sx, sy) = (fx * fx * (3.0 - 2.0 * fx), fy * fy * (3.0 - 2.0 * fy));
    let (ix, iy) = (cx as i32, cy as i32);
    let a = rand(ix, iy, seed);
    let b = rand(ix + 1, iy, seed);
    let c = rand(ix, iy + 1, seed);
    let d = rand(ix + 1, iy + 1, seed);
    let top = a + (b - a) * sx;
    let bottom = c + (d - c) * sx;
    top + (bottom - top) * sy
}

fn fbm(mut x: f32, mut y: f32, octaves: u32, seed: u32) -> f32 {
    let (mut sum, mut weight, mut total) = (0.0, 1.0, 0.0);
    for octave in 0..octaves {
        let shift = octave as f32;
        sum += noise(x + shift * 17.3, y - shift * 9.1, seed) * weight;
        total += weight;
        weight *= 0.5;
        x *= 2.03;
        y *= 2.03;
    }
    sum / total
}

fn ridged(mut x: f32, mut y: f32, seed: u32) -> f32 {
    let (mut sum, mut weight, mut total, mut gain) = (0.0, 1.0, 0.0, 1.0);
    for octave in 0..4 {
        let shift = octave as f32;
        let mut ridge = 1.0 - (2.0 * noise(x + shift * 23.1, y + shift * 5.7, seed) - 1.0).abs();
        ridge = ridge * ridge * gain;
        gain = (ridge * 1.6).clamp(0.0, 1.0);
        sum += ridge * weight;
        total += weight;
        weight *= 0.55;
        x *= 2.1;
        y *= 2.1;
    }
    sum / total
}
