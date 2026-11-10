pub(super) fn gauss(x: f32) -> f32 {
    (-x * x).exp()
}

const fn hash(x: i32, y: i32, seed: u32) -> u32 {
    let mut n = (x as u32).wrapping_mul(0x9E37_79B1)
        ^ (y as u32).wrapping_mul(0x85EB_CA77)
        ^ seed.wrapping_mul(0xC2B2_AE3D);
    n ^= n >> 16;
    n = n.wrapping_mul(0x7FEB_352D);
    n ^= n >> 15;
    n = n.wrapping_mul(0x846C_A68B);
    n ^ (n >> 16)
}

pub(super) fn rand(x: i32, y: i32, seed: u32) -> f32 {
    (hash(x, y, seed) >> 8) as f32 / 16_777_216.0
}

pub(super) fn noise(x: f32, y: f32, seed: u32) -> f32 {
    let (cx, cy) = (x.floor(), y.floor());
    let (fx, fy) = (x - cx, y - cy);
    let (sx, sy) = (fade(fx), fade(fy));
    let (ix, iy) = (cx as i32, cy as i32);
    let top_left = rand(ix, iy, seed);
    let top_right = rand(ix + 1, iy, seed);
    let bottom_left = rand(ix, iy + 1, seed);
    let bottom_right = rand(ix + 1, iy + 1, seed);
    let top = (top_right - top_left).mul_add(sx, top_left);
    let bottom = (bottom_right - bottom_left).mul_add(sx, bottom_left);
    (bottom - top).mul_add(sy, top)
}

fn fade(t: f32) -> f32 {
    t * t * t * t.mul_add(t.mul_add(6.0, -15.0), 10.0)
}

fn turn(x: f32, y: f32, scale: f32) -> (f32, f32) {
    (
        scale * 0.8f32.mul_add(x, -0.6 * y),
        scale * 0.6f32.mul_add(x, 0.8 * y),
    )
}

pub(super) fn fbm(mut x: f32, mut y: f32, octaves: u32, seed: u32) -> f32 {
    let (mut sum, mut weight, mut total) = (0.0, 1.0, 0.0);
    for octave in 0..octaves {
        let shift = octave as f32;
        sum += noise(shift.mul_add(17.3, x), shift.mul_add(-9.1, y), seed) * weight;
        total += weight;
        weight *= 0.5;
        (x, y) = turn(x, y, 2.03);
    }
    sum / total
}

pub(super) fn ridged(mut x: f32, mut y: f32, seed: u32) -> f32 {
    let (mut sum, mut weight, mut total, mut gain) = (0.0, 1.0, 0.0, 1.0);
    for octave in 0..4 {
        let shift = octave as f32;
        let mut ridge = 1.0
            - 2.0f32
                .mul_add(
                    noise(shift.mul_add(23.1, x), shift.mul_add(5.7, y), seed),
                    -1.0,
                )
                .abs();
        ridge = ridge * ridge * gain;
        gain = (ridge * 1.6).clamp(0.0, 1.0);
        sum += ridge * weight;
        total += weight;
        weight *= 0.55;
        (x, y) = turn(x, y, 2.1);
    }
    sum / total
}
