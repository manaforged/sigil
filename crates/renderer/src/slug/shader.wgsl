// Slug fragment-shader text renderer — Phase 2 (M1 + M2 + M3).
//
// Analytic antialiasing, shared band-buffer atlas, instanced batching.
//
// Bindings:
//   group(0) binding(0) — SurfaceParams uniform (vertex + fragment)
//   group(0) binding(1) — atlas: packed BandData for all glyphs (fragment)
//
// Vertex buffers:
//   slot 0 — UV quad (2 × f32, step = Vertex)
//   slot 1 — GlyphInstance (40 bytes, step = Instance):
//     @location(1) origin       vec2<f32>  px top-left
//     @location(2) size         vec2<f32>  px width × height
//     @location(3) color        vec4<f32>  RGBA linear
//     @location(4) atlas_offset u32        f32 index into atlas[]
//     @location(5) polarity     f32        1.0 = light-on-dark, 0.0 = dark-on-light
//
// Band-data layout (per glyph, starting at atlas_offset):
//   16 bands × (1 count f32 + 16 curves × 6 f32) = 1552 f32 entries.
//   Each curve: (p0.x, p0.y, p1.x, p1.y, p2.x, p2.y) in normalised [0,1].

// Text-rendering tuning, mirrored from the CPU `TextTuning` snapshot each frame so
// a debug panel can A/B these live. See `slug/tuning.rs`.
//   • scale    — device pixel ratio (physical px per logical px); sizes the AA ramp.
//   • dilation — Slug dynamic dilation (Lengyel): grow the quad by this many PHYSICAL
//     pixels so the rasterizer emits fragments for partially-covered border pixels
//     (else the right/bottom AA ramp is clipped → thin, broken text at 1×).
//   • sharpen  — AA-ramp narrowing (GreenLightning's `antiAliasingWindowSize`,
//     inverted). >1 concentrates a fractional stem into fewer, higher-coverage
//     columns (crisper) at the cost of slight diagonal aliasing. 1.0 = reference.
//   • gamma_lod / gamma_dol — per-polarity grayscale stem darkening applied as
//     pow(coverage, 1/gamma). Linear-space sRGB coverage blending thins stems by
//     an amount that depends on contrast polarity, so a single global gamma is
//     right for only one polarity. `gamma_lod` corrects LIGHT-on-dark text (the
//     dominant thinning case — needs stronger darkening); `gamma_dol` corrects
//     DARK-on-light text. The per-instance `polarity` flag (1.0 = light-on-dark,
//     0.0 = dark-on-light, classified from text luminance CPU-side) selects
//     which one each glyph uses. 1.0 = off.
//   • enabled  — master A/B: 0.0 forces raw Slug (dilation 0, sharpen 1, gamma 1).
struct SurfaceParams {
    surface_size: vec2<f32>,
    scale:        f32,
    dilation:     f32,
    sharpen:      f32,
    gamma_lod:    f32,
    gamma_dol:    f32,
    enabled:      f32,
}

@group(0) @binding(0) var<uniform>       surface: SurfaceParams;
@group(0) @binding(1) var<storage, read> atlas:   array<f32>;

const BANDS:           u32 = 16u;
const MAX_CURVES:      u32 = 32u;  // lockstep with bands.rs MAX_CURVES_PER_BAND
const FLOATS_PER_BAND: u32 = 193u; // 1 + 32 * 6
// Atlas layout per glyph: BANDS horizontal bands (y-partitioned, +x ray) then
// BANDS vertical bands (x-partitioned, +y ray). HBAND_FLOATS is the offset to
// the vertical set.
const HBAND_FLOATS:    u32 = BANDS * FLOATS_PER_BAND; // 16 * 97 = 1552

struct VertexOutput {
    @builtin(position)              clip_pos:     vec4<f32>,
    @location(0)                    uv:           vec2<f32>,
    @location(1)                    color:        vec4<f32>,
    @location(2) @interpolate(flat) atlas_offset: u32,
    @location(3)                    glyph_width:  f32,
    @location(4)                    glyph_height: f32,
    @location(5) @interpolate(flat) polarity:     f32,
}

@vertex
fn vs_main(
    @location(0) uv:           vec2<f32>,
    @location(1) origin:       vec2<f32>,
    @location(2) size:         vec2<f32>,
    @location(3) color:        vec4<f32>,
    @location(4) atlas_offset: u32,
    @location(5) polarity:     f32,
) -> VertexOutput {
    // glyph_width / glyph_height feed the fragment coverage ramps (one per axis);
    // express them in physical pixels (× scale) so each 1-px antialias window is one
    // physical pixel, not one logical pixel.
    let glyph_w = max(size.x * surface.scale, 1.0);
    let glyph_h = max(size.y * surface.scale, 1.0);

    // Dynamic dilation: grow the quad outward by DILATION_PX physical pixels on each
    // side. In UV that is DILATION_PX / glyph_physical_size; mapping the unit quad
    // uv∈[0,1] to [-e, 1+e] both expands the rasterized polygon AND moves each
    // corner's em-space sample coordinate outward by the same amount, so the glyph
    // does not move or scale — only its partially-covered border pixels now get
    // fragments. The fragment shader clamps the band index for uv outside [0,1].
    // `enabled` (0/1) gates the A/B baseline: 0 → no dilation (raw Slug).
    let dilation = surface.dilation * surface.enabled;
    let ex = dilation / glyph_w;
    let ey = dilation / glyph_h;
    let uv_d = vec2(uv.x * (1.0 + 2.0 * ex) - ex, uv.y * (1.0 + 2.0 * ey) - ey);

    let px   = origin + uv_d * size;
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
        polarity,
    );
}

// Analytic coverage of quadratic Bézier (p0→p1→p2) for a +x ray from p, plus
// the reliability of that ray at this pixel.
//
// For each root t of B(t).y = p.y in [0,1) the signed fractional contribution is
//   clamp((B(t).x − p.x) × width + 0.5, 0.0, 1.0) × sign(B′(t).y)
// (Lengyel 2017 eq. 3; `width` puts the 1-px clamp window in pixel space).
//
// A ray only antialiases edges it crosses at an angle: a +x ray gives an exact
// ramp on a vertical stem and a hard step on a horizontal one. Averaging the two
// axis rays therefore leaves stair-steps on near-axis edges. Each ray reports
// how perpendicular the nearest crossing is (|B′.y| against |B′.x|, in pixels),
// and the fragment blends the two coverages by that weight
// (Lund 2020 "Rendering Resolution Independent Fonts in Games" §4.2.1 eq. 4.4–4.5).
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

// Rotate a point 90°. Casting the +x ray against rotated curves is equivalent to
// a +y ray against the originals — how the vertical pass reuses `quad_coverage`.
fn rotate(v: vec2<f32>) -> vec2<f32> {
    return vec2(v.y, -v.x);
}

// Signed coverage and reliability accumulated over the curves of one band.
// `sample` is the (already rotated, for the vertical pass) ray origin; `px` is the
// pixel size of the local frame along (ramp axis, cross axis).
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
    // A/B gating: when `enabled` is 0 fall back to raw Slug — unit-width AA ramp
    // (sharpen 1) and no stem darkening (gamma 1).
    let sharpen = mix(1.0, surface.sharpen, surface.enabled);
    let gamma_sel = mix(surface.gamma_dol, surface.gamma_lod, in.polarity);
    let gamma     = mix(1.0, gamma_sel, surface.enabled);

    // uv can fall slightly outside [0,1] (dilation), so clamp the band-index
    // selection — u32() of a negative float is undefined in WGSL. The coverage
    // SAMPLE stays the raw uv so the border AA ramp still computes correctly.
    let uy = clamp(in.uv.y, 0.0, 0.999999);
    let ux = clamp(in.uv.x, 0.0, 0.999999);
    let w = in.glyph_width * sharpen;
    let h = in.glyph_height * sharpen;

    // Horizontal +x ray over the y-partitioned bands (antialiases vertical edges).
    let hband = min(u32(uy * f32(BANDS)), BANDS - 1u);
    let hx = band_coverage(in.atlas_offset + hband * FLOATS_PER_BAND, in.uv, vec2(w, h), false);

    // Vertical +y ray over the x-partitioned bands (antialiases horizontal edges).
    let vband = min(u32(ux * f32(BANDS)), BANDS - 1u);
    let vy = band_coverage(
        in.atlas_offset + HBAND_FLOATS + vband * FLOATS_PER_BAND,
        rotate(in.uv), vec2(h, w), true);

    // A ray whose nearest crossing is farther than the 1-px ramp reports a hard
    // in/out value; only rays inside the ramp carry edge information. Blend the
    // informative rays by their perpendicularity, fall back to the plain average
    // when neither (deep inside or outside the glyph, both agree).
    let wh = select(0.0, hx.weight, hx.dist <= 1.0);
    let wv = select(0.0, vy.weight, vy.dist <= 1.0);
    let sum = wh + wv;
    let blended = select(0.5 * (hx.cov + vy.cov), (hx.cov * wh + vy.cov * wv) / max(sum, 1e-6), sum > 1e-6);
    let cov = clamp(blended, 0.0, 1.0);
    if cov < 1e-6 { return vec4(0.0); }
    // Per-polarity stem darkening for linear-space sRGB blending (1.0 = off).
    let alpha = pow(cov, 1.0 / gamma);
    return vec4(in.color.rgb, in.color.a * alpha);
}
