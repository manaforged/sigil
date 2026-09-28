struct SurfaceParams {
    surface_size: vec2<f32>,
    scale:        f32,
    dilation:     f32,
    sharpen:      f32,
    enabled:      f32,
}

@group(0) @binding(0) var<uniform>       surface: SurfaceParams;
@group(0) @binding(1) var<storage, read> atlas:   array<f32>;

const BANDS:           u32 = 16u;
const MAX_CURVES:      u32 = 32u;
const FLOATS_PER_BAND: u32 = 193u;
const HBAND_FLOATS:    u32 = BANDS * FLOATS_PER_BAND;

struct VertexOutput {
    @builtin(position)              clip_pos:     vec4<f32>,
    @location(0)                    uv:           vec2<f32>,
    @location(1)                    color:        vec4<f32>,
    @location(2) @interpolate(flat) atlas_offset: u32,
    @location(3)                    glyph_width:  f32,
    @location(4)                    glyph_height: f32,
    @location(5) @interpolate(flat) gamma:        f32,
}

@vertex
fn vs_main(
    @location(0) uv:           vec2<f32>,
    @location(1) origin:       vec2<f32>,
    @location(2) axis_x:       vec2<f32>,
    @location(3) axis_y:       vec2<f32>,
    @location(4) color:        vec4<f32>,
    @location(5) atlas_offset: u32,
    @location(6) gamma:        f32,
) -> VertexOutput {
    let glyph_w = max(length(axis_x) * surface.scale, 1.0);
    let glyph_h = max(length(axis_y) * surface.scale, 1.0);

    let dilation = surface.dilation * surface.enabled;
    let ex = dilation / glyph_w;
    let ey = dilation / glyph_h;
    let uv_d = vec2(uv.x * (1.0 + 2.0 * ex) - ex, uv.y * (1.0 + 2.0 * ey) - ey);

    let px   = origin + uv_d.x * axis_x + uv_d.y * axis_y;
    let clip = vec2(
         px.x / surface.surface_size.x * 2.0 - 1.0,
        -px.y / surface.surface_size.y * 2.0 + 1.0,
    );
    return VertexOutput(
        vec4(clip, 0.0, 1.0),
        uv_d,
        color,
        atlas_offset,
        glyph_w,
        glyph_h,
        gamma,
    );
}

struct RayHit {
    cov:    f32,
    weight: f32,
    dist:   f32,
}

fn edge_coverage(edge_x: f32, sample_x: f32, width: f32) -> f32 {
    return clamp((edge_x - sample_x) * width + 0.5, 0.0, 1.0);
}

fn root_hit(p0: vec2<f32>, p1: vec2<f32>, p2: vec2<f32>, p: vec2<f32>, px: vec2<f32>, t: f32, hit: ptr<function, RayHit>) {
    let mt = 1.0 - t;
    let bx = mt * mt * p0.x + 2.0 * mt * t * p1.x + t * t * p2.x;
    let d  = ((2.0 * t - 2.0) * p0 + (2.0 - 4.0 * t) * p1 + 2.0 * t * p2) * px;
    let dy = d.y;
    (*hit).cov += edge_coverage(bx, p.x, px.x) * sign(dy);
    let dist = abs(bx - p.x) * px.x;
    if dist < (*hit).dist {
        (*hit).dist   = dist;
        (*hit).weight = abs(dy) / max(abs(d.x) + abs(dy), 1e-6);
    }
}

fn quad_coverage(p0: vec2<f32>, p1: vec2<f32>, p2: vec2<f32>, p: vec2<f32>, px: vec2<f32>, hit: ptr<function, RayHit>) {
    let a = p0.y - 2.0 * p1.y + p2.y;
    let b = 2.0 * (p1.y - p0.y);
    let c = p0.y - p.y;

    if abs(a) < 1e-6 {
        if abs(b) > 1e-6 {
            let t = -c / b;
            if t >= 0.0 && t < 1.0 {
                root_hit(p0, p1, p2, p, px, t, hit);
            }
        }
        return;
    }

    let disc = b * b - 4.0 * a * c;
    if disc < 0.0 { return; }
    let sq    = sqrt(disc);
    let inv2a = 0.5 / a;

    let t1 = (-b - sq) * inv2a;
    if t1 >= 0.0 && t1 < 1.0 {
        root_hit(p0, p1, p2, p, px, t1, hit);
    }
    let t2 = (-b + sq) * inv2a;
    if t2 >= 0.0 && t2 < 1.0 {
        root_hit(p0, p1, p2, p, px, t2, hit);
    }
}

fn rotate(v: vec2<f32>) -> vec2<f32> {
    return vec2(v.y, -v.x);
}

fn band_coverage(base: u32, sample: vec2<f32>, px: vec2<f32>, do_rotate: bool) -> RayHit {
    let count = min(u32(atlas[base]), MAX_CURVES);
    var hit = RayHit(0.0, 0.0, 1e9);
    for (var i = 0u; i < count; i++) {
        let o  = base + 1u + i * 6u;
        var p0 = vec2(atlas[o],      atlas[o + 1u]);
        var p1 = vec2(atlas[o + 2u], atlas[o + 3u]);
        var p2 = vec2(atlas[o + 4u], atlas[o + 5u]);
        if do_rotate {
            p0 = rotate(p0);
            p1 = rotate(p1);
            p2 = rotate(p2);
        }
        quad_coverage(p0, p1, p2, sample, px, &hit);
    }
    hit.cov = abs(hit.cov);
    return hit;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let sharpen = mix(1.0, surface.sharpen, surface.enabled);

    let uy = clamp(in.uv.y, 0.0, 0.999999);
    let ux = clamp(in.uv.x, 0.0, 0.999999);
    let w = in.glyph_width * sharpen;
    let h = in.glyph_height * sharpen;

    let hband = min(u32(uy * f32(BANDS)), BANDS - 1u);
    let hx = band_coverage(in.atlas_offset + hband * FLOATS_PER_BAND, in.uv, vec2(w, h), false);

    let vband = min(u32(ux * f32(BANDS)), BANDS - 1u);
    let vy = band_coverage(
        in.atlas_offset + HBAND_FLOATS + vband * FLOATS_PER_BAND,
        rotate(in.uv), vec2(h, w), true);

    let wh = select(0.0, hx.weight, hx.dist <= 1.0);
    let wv = select(0.0, vy.weight, vy.dist <= 1.0);
    let sum = wh + wv;
    let blended = select(0.5 * (hx.cov + vy.cov), (hx.cov * wh + vy.cov * wv) / max(sum, 1e-6), sum > 1e-6);
    let cov = clamp(blended, 0.0, 1.0);
    if cov < 1e-6 { return vec4(0.0); }
    let alpha = pow(cov, 1.0 / max(in.gamma, 1e-6));
    return vec4(in.color.rgb, in.color.a * alpha);
}
