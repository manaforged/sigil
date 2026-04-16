// Slug fragment-shader text renderer — Phase 1.
//
// Each glyph is rendered as a screen-aligned quad covering its bounding box.
// UV [0,1]² maps to the glyph bbox with y=0 at the top (ascender) and y=1 at
// the bottom (descender). The fragment shader performs non-zero fill winding
// via ray-casting over preprocessed quadratic Bézier bands.
//
// Band-data layout (flat f32 storage buffer):
//   16 bands × (1 count + 16 curves × 6 floats) = 1552 f32 entries.
//   Each curve: (p0.x, p0.y, p1.x, p1.y, p2.x, p2.y) in normalised [0,1].

struct GlyphParams {
    origin:       vec2<f32>,   // top-left pixel corner of the glyph quad
    size:         vec2<f32>,   // pixel dimensions (width, height) of the quad
    color:        vec4<f32>,   // RGBA foreground colour (linear)
    surface_size: vec2<f32>,   // surface pixel dimensions for clip-space transform
    _pad:         vec2<f32>,
}

@group(0) @binding(0) var<uniform>          params:    GlyphParams;
@group(0) @binding(1) var<storage, read>    band_data: array<f32>;

const BANDS:           u32 = 16u;
const MAX_CURVES:      u32 = 16u;
const FLOATS_PER_BAND: u32 = 97u; // 1 + 16 * 6

struct VertexOutput {
    @builtin(position) clip_pos: vec4<f32>,
    @location(0)       uv:       vec2<f32>,
}

@vertex
fn vs_main(@location(0) uv: vec2<f32>) -> VertexOutput {
    let px = params.origin + uv * params.size;
    let clip = vec2(
         px.x / params.surface_size.x *  2.0 - 1.0,
        -px.y / params.surface_size.y *  2.0 + 1.0,
    );
    return VertexOutput(vec4(clip, 0.0, 1.0), uv);
}

// Winding contribution of quadratic Bézier (p0→p1→p2) for a +x ray from p.
// Returns ±1 per crossing (non-zero fill convention), 0 for none.
fn quad_winding(p0: vec2<f32>, p1: vec2<f32>, p2: vec2<f32>, p: vec2<f32>) -> i32 {
    // Solve B(t).y = p.y  →  a·t² + b·t + c = 0
    let a = p0.y - 2.0 * p1.y + p2.y;
    let b = 2.0 * (p1.y - p0.y);
    let c = p0.y - p.y;
    var w: i32 = 0;

    if abs(a) < 1e-6 {
        // Linear case
        if abs(b) > 1e-6 {
            let t = -c / b;
            if t >= 0.0 && t < 1.0 {
                let mt = 1.0 - t;
                let bx = mt * mt * p0.x + 2.0 * mt * t * p1.x + t * t * p2.x;
                if bx > p.x { w = select(-1, 1, b > 0.0); }
            }
        }
        return w;
    }

    let disc = b * b - 4.0 * a * c;
    if disc < 0.0 { return 0; }
    let sq     = sqrt(disc);
    let inv2a  = 0.5 / a;

    let t1 = (-b - sq) * inv2a;
    if t1 >= 0.0 && t1 < 1.0 {
        let mt1 = 1.0 - t1;
        let bx1 = mt1 * mt1 * p0.x + 2.0 * mt1 * t1 * p1.x + t1 * t1 * p2.x;
        if bx1 > p.x { w += select(-1, 1, (2.0 * a * t1 + b) > 0.0); }
    }

    let t2 = (-b + sq) * inv2a;
    if t2 >= 0.0 && t2 < 1.0 {
        let mt2 = 1.0 - t2;
        let bx2 = mt2 * mt2 * p0.x + 2.0 * mt2 * t2 * p1.x + t2 * t2 * p2.x;
        if bx2 > p.x { w += select(-1, 1, (2.0 * a * t2 + b) > 0.0); }
    }

    return w;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let uv    = in.uv;
    let band  = min(u32(uv.y * f32(BANDS)), BANDS - 1u);
    let base  = band * FLOATS_PER_BAND;
    let count = min(u32(band_data[base]), MAX_CURVES);

    var winding: i32 = 0i;
    for (var i = 0u; i < count; i++) {
        let o  = base + 1u + i * 6u;
        let p0 = vec2(band_data[o],      band_data[o + 1u]);
        let p1 = vec2(band_data[o + 2u], band_data[o + 3u]);
        let p2 = vec2(band_data[o + 4u], band_data[o + 5u]);
        winding += quad_winding(p0, p1, p2, uv);
    }

    if winding == 0i { return vec4(0.0); }
    return params.color;
}
