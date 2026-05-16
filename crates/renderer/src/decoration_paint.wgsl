// Decoration paint shader — underline / strikethrough / wavy underline.
//
// Vertex stage: maps a unit quad to the band rect in logical pixels.
// Fragment stage: branches on `kind` (flat u32) to produce the correct mask.
// Per-fragment dither matches the R1 hash_xy approach.

const DECORATION_UNDERLINE: u32     = 0u;
const DECORATION_STRIKETHROUGH: u32 = 1u;
const DECORATION_WAVY: u32          = 2u;

const TAU: f32 = 6.283185307179586;

struct Uniforms {
    viewport: vec2<f32>,
    _pad: vec2<f32>,
}

@group(0) @binding(0) var<uniform> u: Uniforms;

struct InstanceIn {
    // band rect [x, y, w, h] in logical pixels
    @location(1) band: vec4<f32>,
    // linear RGBA [r, g, b, a] in [0, 1]
    @location(2) color: vec4<f32>,
    // [opacity_01, period, line_width, kind_f32]
    @location(3) params: vec4<f32>,
}

struct VertexIn {
    @location(0) unit: vec2<f32>,
}

struct VertexOut {
    @builtin(position)                    clip:      vec4<f32>,
    @location(0)                          color:     vec4<f32>,
    // band-relative pixel coords: x in [0, band.w], y in [0, band.h]
    @location(1)                          local:     vec2<f32>,
    @location(2)                          band_size: vec2<f32>,
    @location(3)                          opacity:   f32,
    @location(4)                          period:    f32,
    @location(5)                          line_width: f32,
    // flat: integer discriminant must not be interpolated
    @location(6) @interpolate(flat)       kind:      u32,
}

@vertex
fn vs_main(v: VertexIn, inst: InstanceIn) -> VertexOut {
    var out: VertexOut;
    let pos_px = inst.band.xy + v.unit * inst.band.zw;
    let ndc = vec2<f32>(
        (pos_px.x / u.viewport.x) * 2.0 - 1.0,
        1.0 - (pos_px.y / u.viewport.y) * 2.0,
    );
    out.clip      = vec4<f32>(ndc, 0.0, 1.0);
    out.color     = inst.color;
    out.local     = v.unit * inst.band.zw;
    out.band_size = inst.band.zw;
    out.opacity   = inst.params.x;
    out.period    = inst.params.y;
    out.line_width = inst.params.z;
    out.kind      = u32(inst.params.w);
    return out;
}

// Interleaved hash for sub-pixel dither (same approach as R1 rect shader).
fn hash_xy(p: vec2<f32>) -> f32 {
    let q = fract(p * vec2<f32>(0.1031, 0.1030));
    let r = q + dot(q, q.yx + 33.33);
    return fract((r.x + r.y) * r.x);
}

@fragment
fn fs_main(in: VertexOut) -> @location(0) vec4<f32> {
    let local = in.local;
    var mask = 0.0;

    if in.kind == DECORATION_UNDERLINE {
        let dist_y = abs(local.y - in.band_size.y * 0.5) - in.line_width * 0.5;
        mask = 1.0 - smoothstep(-0.5, 0.5, dist_y);
    } else if in.kind == DECORATION_STRIKETHROUGH {
        let dist_y = abs(local.y - in.band_size.y * 0.5) - in.line_width * 0.5;
        mask = 1.0 - smoothstep(-0.5, 0.5, dist_y);
    } else {
        // WavyUnderline. Period is CPU-clamped to >= 1.0 but we double-guard
        // here so any caller bypassing the builder still avoids NaN/Inf.
        let safe_period = max(in.period, 1.0);
        let centre_y  = in.band_size.y * 0.5;
        let amplitude = in.line_width * 0.75;
        let phase     = local.x / safe_period * TAU;
        let wave_y    = centre_y + cos(phase) * amplitude;
        let dist_wave = abs(local.y - wave_y) - in.line_width * 0.4;
        mask = 1.0 - smoothstep(-0.5, 0.5, dist_wave);
    }

    // Dither: ±0.5/255 to break 8-bit banding on smooth gradients.
    let dither = (hash_xy(in.clip.xy) - 0.5) / 255.0;
    let alpha  = clamp(in.color.a * in.opacity * mask + dither, 0.0, 1.0);
    return vec4<f32>(in.color.rgb * alpha, alpha);
}
