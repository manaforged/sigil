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

struct SurfaceParams {
    surface_size: vec2<f32>,
    _pad:         vec2<f32>,
}

@group(0) @binding(0) var<uniform>       surface: SurfaceParams;
@group(0) @binding(1) var<storage, read> atlas:   array<f32>;

const BANDS:           u32 = 16u;
const MAX_CURVES:      u32 = 16u;
const FLOATS_PER_BAND: u32 = 97u; // 1 + 16 * 6

struct VertexOutput {
    @builtin(position)              clip_pos:     vec4<f32>,
    @location(0)                    uv:           vec2<f32>,
    @location(1)                    color:        vec4<f32>,
    @location(2) @interpolate(flat) atlas_offset: u32,
    @location(3)                    glyph_width:  f32,
}

@vertex
fn vs_main(
    @location(0) uv:           vec2<f32>,
    @location(1) origin:       vec2<f32>,
    @location(2) size:         vec2<f32>,
    @location(3) color:        vec4<f32>,
    @location(4) atlas_offset: u32,
) -> VertexOutput {
    let px   = origin + uv * size;
    let clip = vec2(
         px.x / surface.surface_size.x * 2.0 - 1.0,
        -px.y / surface.surface_size.y * 2.0 + 1.0,
    );
    return VertexOutput(vec4(clip, 0.0, 1.0), uv, color, atlas_offset, size.x);
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

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let band  = min(u32(in.uv.y * f32(BANDS)), BANDS - 1u);
    let base  = in.atlas_offset + band * FLOATS_PER_BAND;
    let count = min(u32(atlas[base]), MAX_CURVES);

    var coverage: f32 = 0.0;
    for (var i = 0u; i < count; i++) {
        let o  = base + 1u + i * 6u;
        let p0 = vec2(atlas[o],      atlas[o + 1u]);
        let p1 = vec2(atlas[o + 2u], atlas[o + 3u]);
        let p2 = vec2(atlas[o + 4u], atlas[o + 5u]);
        coverage += quad_coverage(p0, p1, p2, in.uv, in.glyph_width);
    }

    let alpha = abs(coverage);
    if alpha < 1e-6 { return vec4(0.0); }
    return vec4(in.color.rgb, in.color.a * alpha);
}
