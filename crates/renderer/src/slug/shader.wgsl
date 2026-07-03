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
//   • gamma    — grayscale stem darkening: pow(coverage, 1/gamma) restores the
//     perceptual stem weight that linear-space sRGB coverage blending thins. 1.0 = off.
//   • enabled  — master A/B: 0.0 forces raw Slug (dilation 0, sharpen 1, gamma 1).
struct SurfaceParams {
    surface_size: vec2<f32>,
    scale:        f32,
    dilation:     f32,
    sharpen:      f32,
    gamma:        f32,
    enabled:      f32,
    _pad:         f32,
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
}

@vertex
fn vs_main(
    @location(0) uv:           vec2<f32>,
    @location(1) origin:       vec2<f32>,
    @location(2) size:         vec2<f32>,
    @location(3) color:        vec4<f32>,
    @location(4) atlas_offset: u32,
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
    );
}

// Analytic coverage contribution of quadratic Bézier (p0→p1→p2) for a +x ray from p.
//
// For each root t of B(t).y = p.y in [0,1) the signed fractional contribution is:
//   clamp((B(t).x − p.x) × width + 0.5, 0.0, 1.0) × sign(B′(t).y)
// where `width` is the glyph pixel width so the 1-px clamp window is in pixel space.
// The +0.5 term converts fragment-center UVs to a pixel-edge coverage ramp.
//
// Summing over all curves and taking abs(sum) yields coverage ∈ [0,1].
// Ref: Lengyel (2017) §4.3, eqs. (17)–(21).
fn edge_coverage(edge_x: f32, sample_x: f32, width: f32) -> f32 {
    return clamp((edge_x - sample_x) * width + 0.5, 0.0, 1.0);
}

fn quad_coverage(p0: vec2<f32>, p1: vec2<f32>, p2: vec2<f32>, p: vec2<f32>, width: f32) -> f32 {
    let a = p0.y - 2.0 * p1.y + p2.y;
    let b = 2.0 * (p1.y - p0.y);
    let c = p0.y - p.y;
    var cov: f32 = 0.0;

    if abs(a) < 1e-6 {
        if abs(b) > 1e-6 {
            let t = -c / b;
            if t >= 0.0 && t < 1.0 {
                let mt = 1.0 - t;
                let bx = mt * mt * p0.x + 2.0 * mt * t * p1.x + t * t * p2.x;
                cov += edge_coverage(bx, p.x, width) * sign(b);
            }
        }
        return cov;
    }

    let disc = b * b - 4.0 * a * c;
    if disc < 0.0 { return 0.0; }
    let sq    = sqrt(disc);
    let inv2a = 0.5 / a;

    let t1 = (-b - sq) * inv2a;
    if t1 >= 0.0 && t1 < 1.0 {
        let mt1 = 1.0 - t1;
        let bx1 = mt1 * mt1 * p0.x + 2.0 * mt1 * t1 * p1.x + t1 * t1 * p2.x;
        cov += edge_coverage(bx1, p.x, width) * sign(2.0 * a * t1 + b);
    }

    let t2 = (-b + sq) * inv2a;
    if t2 >= 0.0 && t2 < 1.0 {
        let mt2 = 1.0 - t2;
        let bx2 = mt2 * mt2 * p0.x + 2.0 * mt2 * t2 * p1.x + t2 * t2 * p2.x;
        cov += edge_coverage(bx2, p.x, width) * sign(2.0 * a * t2 + b);
    }

    return cov;
}

// Rotate a point 90°. Casting the +x ray against rotated curves is equivalent to
// a +y ray against the originals — how the vertical pass reuses `quad_coverage`.
fn rotate(v: vec2<f32>) -> vec2<f32> {
    return vec2(v.y, -v.x);
}

// Signed coverage accumulated over the curves of one band. `sample` is the (already
// rotated, for the vertical pass) ray origin; `width` is the px-per-uv span along
// the ramp axis. `do_rotate` rotates each curve so the +x ray becomes a +y ray.
fn band_coverage(base: u32, sample: vec2<f32>, width: f32, do_rotate: bool) -> f32 {
    let count = min(u32(atlas[base]), MAX_CURVES);
    var cov: f32 = 0.0;
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
        cov += quad_coverage(p0, p1, p2, sample, width);
    }
    return cov;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    // A/B gating: when `enabled` is 0 fall back to raw Slug — unit-width AA ramp
    // (sharpen 1) and no stem darkening (gamma 1).
    let sharpen = mix(1.0, surface.sharpen, surface.enabled);
    let gamma   = mix(1.0, surface.gamma,   surface.enabled);

    // uv can fall slightly outside [0,1] now (dilation), so clamp the band-index
    // selection — u32() of a negative float is undefined in WGSL. The coverage
    // SAMPLE stays the raw uv so the border AA ramp still computes correctly.
    let uy = clamp(in.uv.y, 0.0, 0.999999);
    let ux = clamp(in.uv.x, 0.0, 0.999999);

    // Horizontal +x ray over the y-partitioned bands (antialiases vertical edges).
    let hband = min(u32(uy * f32(BANDS)), BANDS - 1u);
    let cov_h = band_coverage(
        in.atlas_offset + hband * FLOATS_PER_BAND, in.uv,
        in.glyph_width * sharpen, false);

    // Vertical +y ray over the x-partitioned bands (antialiases horizontal edges) —
    // the missing second axis whose absence made near-horizontal strokes jagged.
    let vband = min(u32(ux * f32(BANDS)), BANDS - 1u);
    let cov_v = band_coverage(
        in.atlas_offset + HBAND_FLOATS + vband * FLOATS_PER_BAND,
        rotate(in.uv), in.glyph_height * sharpen, true);

    // Average the two rays (Slug's 2D coverage), per Lengyel (2017) / GreenLightning.
    let cov = clamp(0.5 * (abs(cov_h) + abs(cov_v)), 0.0, 1.0);
    if cov < 1e-6 { return vec4(0.0); }
    // Stem darkening for linear-space sRGB blending (1.0 = off).
    let alpha = pow(cov, 1.0 / gamma);
    return vec4(in.color.rgb, in.color.a * alpha);
}
