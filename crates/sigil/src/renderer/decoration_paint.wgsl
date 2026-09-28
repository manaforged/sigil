const DECORATION_WAVY: u32          = 2u;

const TAU: f32 = 6.283185307179586;

struct Uniforms {
    viewport: vec2<f32>,
    _pad: vec2<f32>,
}

@group(0) @binding(0) var<uniform> u: Uniforms;

struct InstanceIn {
    @location(1) band: vec4<f32>,
    @location(2) color: vec4<f32>,
    @location(3) params: vec4<f32>,
    @location(4) axes: vec4<f32>,
    @location(5) offset: vec4<f32>,
}

struct VertexIn {
    @location(0) unit: vec2<f32>,
}

struct VertexOut {
    @builtin(position)                    clip:      vec4<f32>,
    @location(0)                          color:     vec4<f32>,
    @location(1)                          local:     vec2<f32>,
    @location(2)                          band_size: vec2<f32>,
    @location(3)                          opacity:   f32,
    @location(4)                          period:    f32,
    @location(5)                          thickness: f32,
    @location(6)                          amplitude: f32,
    @location(7) @interpolate(flat)       kind:      u32,
}

@vertex
fn vs_main(v: VertexIn, inst: InstanceIn) -> VertexOut {
    var out: VertexOut;
    let pad = vec2<f32>(1.0 / max(length(inst.axes.xy), 1e-6), 1.0 / max(length(inst.axes.zw), 1e-6));
    let corner = v.unit * (inst.band.zw + 2.0 * pad) - pad;
    let local_px = inst.band.xy + corner;
    let pos_px = inst.axes.xy * local_px.x + inst.axes.zw * local_px.y + inst.offset.xy;
    let ndc = vec2<f32>(
        (pos_px.x / u.viewport.x) * 2.0 - 1.0,
        1.0 - (pos_px.y / u.viewport.y) * 2.0,
    );
    out.clip      = vec4<f32>(ndc, 0.0, 1.0);
    out.color     = inst.color;
    out.local     = corner;
    out.band_size = inst.band.zw;
    out.opacity   = inst.params.x;
    out.period    = max(inst.params.y, 1.0);
    out.thickness = inst.params.z;
    out.amplitude = inst.offset.z;
    out.kind      = u32(inst.params.w);
    return out;
}

fn hash_xy(p: vec2<f32>) -> f32 {
    let q = fract(p * vec2<f32>(0.1031, 0.1030));
    let r = q + dot(q, q.yx + 33.33);
    return fract((r.x + r.y) * r.x);
}

fn coverage(offset: f32, half_width: f32) -> f32 {
    let pixel = max(length(vec2<f32>(dpdx(offset), dpdy(offset))), 1e-4);
    let inside = clamp((half_width + 0.5 * pixel - abs(offset)) / pixel, 0.0, 1.0);
    let span = clamp(2.0 * half_width / pixel, 0.0, 1.0);
    return min(inside, span);
}

@fragment
fn fs_main(in: VertexOut) -> @location(0) vec4<f32> {
    let local = in.local;
    let centre_y = in.band_size.y * 0.5;
    var offset = local.y - centre_y;
    let half_width = in.thickness * 0.5;
    if in.kind == DECORATION_WAVY {
        let phase = local.x / in.period * TAU;
        let slope = in.amplitude * TAU / in.period * cos(phase);
        offset = (local.y - (centre_y + sin(phase) * in.amplitude)) / sqrt(1.0 + slope * slope);
    }
    let half_length = in.band_size.x * 0.5;
    let mask = coverage(offset, half_width) * coverage(local.x - half_length, half_length);

    let dither = (hash_xy(in.clip.xy) - 0.5) / 255.0;
    let alpha  = clamp(in.color.a * in.opacity * mask + dither, 0.0, 1.0);
    return vec4<f32>(in.color.rgb * alpha, alpha);
}
