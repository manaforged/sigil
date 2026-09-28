const FILL_KIND_GRADIENT: u32 = 0u;
const FILL_KIND_SLASH: u32 = 1u;
const FILL_KIND_CHECKER: u32 = 2u;

struct Uniforms {
    viewport: vec2<f32>,
    _pad: vec2<f32>,
};

struct GpuStop {
    offset_bits: u32,
    rgba01: u32,
    rgba23: u32,
    _pad: u32,
};

struct StopData {
    offset: f32,
    color: vec4<f32>,
};

@group(0) @binding(0) var<uniform> u: Uniforms;
@group(1) @binding(0) var<storage, read> stops: array<GpuStop>;

struct VertexIn {
    @location(0) unit: vec2<f32>,
};

struct InstanceIn {
    @location(1) rect: vec4<f32>,
    @location(2) line: vec4<f32>,
    @location(3) corner_radii: vec4<f32>,
    @location(4) params: vec4<f32>,
    @location(5) stop_range: vec4<u32>,
    @location(6) stroke_params: vec4<f32>,
    @location(7) stroke_meta: vec4<f32>,
    @location(8) pattern_a: vec4<f32>,
    @location(9) pattern_b: vec4<f32>,
    @location(10) xform_x: vec4<f32>,
    @location(11) xform_y: vec4<f32>,
};

struct VertexOut {
    @builtin(position) clip: vec4<f32>,
    @location(0) pos_px: vec2<f32>,
    @location(1) local: vec2<f32>,
    @location(2) half_size: vec2<f32>,
    @location(3) corner_radii: vec4<f32>,
    @location(4) opacity: f32,
    @location(5) start: vec2<f32>,
    @location(6) axis: vec2<f32>,
    @location(7) @interpolate(flat) stops_meta: vec2<u32>,
    @location(8) stroke_params: vec4<f32>,
    @location(9) stroke_meta: vec4<f32>,
    @location(10) pattern_a: vec4<f32>,
    @location(11) pattern_b: vec4<f32>,
    @location(12) @interpolate(flat) fill_kind: u32,
    @location(13) dash_origin: vec2<f32>,
};

@vertex
fn vs_main(v: VertexIn, inst: InstanceIn) -> VertexOut {
    var out: VertexOut;
    let pad = aa_pad(inst.xform_x.xy, inst.xform_y.xy);
    let corner = v.unit * (inst.rect.zw + 2.0 * pad) - pad;
    let local_px = inst.rect.xy + corner;
    let pos_px = vec2<f32>(
        dot(inst.xform_x.xy, local_px) + inst.xform_x.z,
        dot(inst.xform_y.xy, local_px) + inst.xform_y.z,
    );
    let ndc = vec2<f32>(
        (pos_px.x / u.viewport.x) * 2.0 - 1.0,
        1.0 - (pos_px.y / u.viewport.y) * 2.0,
    );
    let expanded_half = inst.rect.zw * 0.5;
    let outset = inst.params.y;
    let surface_half = max(expanded_half - vec2<f32>(outset, outset), vec2<f32>(0.0, 0.0));
    out.clip = vec4<f32>(ndc, 0.0, 1.0);
    out.pos_px = local_px;
    out.local = corner - expanded_half;
    out.half_size = surface_half;
    out.corner_radii = inst.corner_radii;
    out.opacity = inst.params.x;
    out.start = inst.line.xy;
    out.axis = inst.line.zw - inst.line.xy;
    out.stops_meta = inst.stop_range.xy;
    out.stroke_params = inst.stroke_params;
    out.stroke_meta = inst.stroke_meta;
    out.dash_origin = inst.params.zw;
    out.pattern_a = inst.pattern_a;
    out.pattern_b = inst.pattern_b;
    out.fill_kind = inst.stop_range.z;
    return out;
}

fn aa_pad(row_x: vec2<f32>, row_y: vec2<f32>) -> vec2<f32> {
    let column_x = length(vec2<f32>(row_x.x, row_y.x));
    let column_y = length(vec2<f32>(row_x.y, row_y.y));
    return vec2<f32>(1.0 / max(column_x, 1e-6), 1.0 / max(column_y, 1e-6));
}

fn corner_radius(p: vec2<f32>, radii: vec4<f32>) -> f32 {
    let bottom = select(radii.w, radii.z, p.x > 0.0);
    let top = select(radii.x, radii.y, p.x > 0.0);
    return select(bottom, top, p.y < 0.0);
}

fn rounded_rect_sdf(p: vec2<f32>, half: vec2<f32>, radii: vec4<f32>) -> f32 {
    let r = corner_radius(p, radii);
    let q = abs(p) - half + vec2<f32>(r, r);
    return length(max(q, vec2<f32>(0.0, 0.0))) + min(max(q.x, q.y), 0.0) - r;
}

fn read_stop(base: u32, index: u32) -> StopData {
    let raw = stops[base + index];
    let rg = unpack2x16float(raw.rgba01);
    let ba = unpack2x16float(raw.rgba23);
    var out: StopData;
    out.offset = bitcast<f32>(raw.offset_bits);
    out.color = vec4<f32>(rg.x, rg.y, ba.x, ba.y);
    return out;
}

fn sample_gradient(base: u32, count_in: u32, t: f32) -> vec4<f32> {
    let count = clamp(count_in, 1u, 8u);
    var prev = read_stop(base, 0u);
    if t < prev.offset {
        return premultiply(prev.color);
    }
    for (var i = 1u; i < count; i = i + 1u) {
        let next = read_stop(base, i);
        if t < next.offset {
            let amount = (t - prev.offset) / (next.offset - prev.offset);
            return premultiply(mix(prev.color, next.color, amount));
        }
        prev = next;
    }
    return premultiply(prev.color);
}

fn premultiply(color: vec4<f32>) -> vec4<f32> {
    return vec4<f32>(color.rgb * color.a, color.a);
}

fn hash_xy(p: vec2<f32>) -> f32 {
    return fract(sin(dot(p, vec2<f32>(127.1, 311.7))) * 43758.5453123);
}

fn edge_coverage(d: f32, aa: f32) -> f32 {
    return clamp(0.5 - d / (2.0 * aa), 0.0, 1.0);
}

fn dash_mask(local: vec2<f32>, dash_len: f32, dash_gap: f32) -> f32 {
    if dash_len <= 0.0 {
        return 1.0;
    }
    let period = dash_len + dash_gap;
    let arc = abs(local.x) + abs(local.y);
    return select(0.0, 1.0, arc - floor(arc / period) * period < dash_len);
}

fn stroke_mask(d: f32, local: vec2<f32>, band: vec4<f32>, aa: f32) -> f32 {
    if band.x + band.y <= 0.0 {
        return 0.0;
    }
    let stroke_d = max(-band.x - d, d - band.y);
    let alpha = edge_coverage(stroke_d, aa);
    return alpha * dash_mask(local, band.z, band.w);
}

fn overlay_stroke(fill: vec4<f32>, stroke: vec4<f32>, amount: f32) -> vec4<f32> {
    let stroke_alpha = stroke.a * amount;
    let out_alpha = stroke_alpha + fill.a * (1.0 - stroke_alpha);
    let rgb = stroke.rgb * stroke_alpha + fill.rgb * (1.0 - stroke_alpha);
    return vec4<f32>(rgb, out_alpha);
}

fn slash_color(local: vec2<f32>, geometry: vec2<f32>, color: vec4<f32>) -> vec4<f32> {
    let period = geometry.x;
    let line_width = geometry.y;
    let p = (local.x - local.y) / period;
    let phase = (p - floor(p)) * period;
    let edge_dist = min(phase, period - phase) - line_width * 0.5;
    let aa = fwidth(local.x - local.y) * 0.5;
    let soft = 1.0 - smoothstep(-aa, aa, edge_dist);
    let full = select(soft, 1.0, line_width >= period);
    let mask = select(full, 0.0, line_width <= 0.0);
    return color * mask;
}

fn checker_color(local: vec2<f32>, geometry: vec2<f32>, color_a: vec4<f32>, color_b: vec4<f32>) -> vec4<f32> {
    let cell = geometry.x;
    let cx = floor(local.x / cell);
    let cy = floor(local.y / cell);
    let parity = (cx + cy) - 2.0 * floor((cx + cy) * 0.5);
    return select(color_b, color_a, parity == 0.0);
}

@fragment
fn fs_main(in: VertexOut) -> @location(0) vec4<f32> {
    let d = rounded_rect_sdf(in.local, in.half_size, in.corner_radii);
    let aa = max(fwidth(d) * 0.5, 0.0001);
    let coverage = edge_coverage(d, aa);

    var color: vec4<f32>;
    if in.fill_kind == FILL_KIND_SLASH {
        color = slash_color(in.local + in.half_size + in.start + in.axis, in.start, in.pattern_a);
    } else if in.fill_kind == FILL_KIND_CHECKER {
        color = checker_color(in.local + in.half_size + in.start + in.axis, in.start, in.pattern_a, in.pattern_b);
    } else {
        let axis_len = max(dot(in.axis, in.axis), 0.0001);
        let t = dot(in.pos_px - in.start, in.axis) / axis_len;
        color = sample_gradient(in.stops_meta.x, in.stops_meta.y, t);
    }

    let alpha = color.a * in.opacity * coverage;
    let scale = in.opacity * coverage;
    let dither = hash_xy(in.clip.xy) * (2.0 / 255.0) - (1.0 / 255.0);
    let rgb = clamp(color.rgb * scale + vec3<f32>(dither * alpha), vec3<f32>(0.0), vec3<f32>(1.0));
    let fill = vec4<f32>(rgb, alpha);
    let amount = stroke_mask(d, in.local + in.dash_origin, in.stroke_meta, aa);
    let stroke = vec4<f32>(in.stroke_params.rgb, in.stroke_params.a * in.opacity);
    return overlay_stroke(fill, stroke, amount);
}
