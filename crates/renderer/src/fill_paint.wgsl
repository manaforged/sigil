const STROKE_STYLE_SOLID: f32 = 0.0;
const STROKE_STYLE_DASHED: f32 = 1.0;
const STROKE_ALIGN_CENTERED: f32 = 0.0;
const STROKE_ALIGN_INSIDE: f32 = 1.0;
const STROKE_ALIGN_OUTSIDE: f32 = 2.0;

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
};

@vertex
fn vs_main(v: VertexIn, inst: InstanceIn) -> VertexOut {
    var out: VertexOut;
    let pos_px = inst.rect.xy + v.unit * inst.rect.zw;
    let ndc = vec2<f32>(
        (pos_px.x / u.viewport.x) * 2.0 - 1.0,
        1.0 - (pos_px.y / u.viewport.y) * 2.0,
    );
    // The CPU inflated the draw quad by `outset` (params.y) so the
    // stroke's outside half has fragments to paint into. Recover the
    // original surface half-size so the rounded-rect SDF measures
    // distance from the surface edge, not the expanded quad edge.
    let expanded_half = inst.rect.zw * 0.5;
    let outset = inst.params.y;
    let surface_half = max(expanded_half - vec2<f32>(outset, outset), vec2<f32>(0.0, 0.0));
    out.clip = vec4<f32>(ndc, 0.0, 1.0);
    out.pos_px = pos_px;
    out.local = v.unit * inst.rect.zw - expanded_half;
    out.half_size = surface_half;
    out.corner_radii = inst.corner_radii;
    out.opacity = inst.params.x;
    out.start = inst.line.xy;
    out.axis = inst.line.zw - inst.line.xy;
    out.stops_meta = inst.stop_range.xy;
    out.stroke_params = inst.stroke_params;
    out.stroke_meta = inst.stroke_meta;
    out.pattern_a = inst.pattern_a;
    out.pattern_b = inst.pattern_b;
    out.fill_kind = inst.stop_range.z;
    return out;
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
    let count = min(count_in, 8u);
    var prev = read_stop(base, 0u);
    if count <= 1u {
        return prev.color;
    }
    var i = 1u;
    loop {
        if i >= count || i >= 8u {
            break;
        }
        let next = read_stop(base, i);
        if t <= next.offset {
            let amount = smoothstep(prev.offset, next.offset, t);
            return mix(prev.color, next.color, amount);
        }
        prev = next;
        i = i + 1u;
    }
    return prev.color;
}

fn hash_xy(p: vec2<f32>) -> f32 {
    return fract(sin(dot(p, vec2<f32>(127.1, 311.7))) * 43758.5453123);
}

fn stroke_offset(width: f32, align: f32) -> f32 {
    if align == STROKE_ALIGN_INSIDE {
        return -width * 0.5;
    }
    if align == STROKE_ALIGN_OUTSIDE {
        return width * 0.5;
    }
    return 0.0;
}

fn dash_mask(local: vec2<f32>, width: f32, style: f32, dash_gap: f32) -> f32 {
    if style != STROKE_STYLE_DASHED || dash_gap <= 0.0 {
        return 1.0;
    }
    let dash_len = max(width * 3.0, 1.0);
    let period = dash_len + dash_gap;
    let arc = abs(local.x) + abs(local.y);
    return select(0.0, 1.0, arc - floor(arc / period) * period < dash_len);
}

fn stroke_mask(d: f32, local: vec2<f32>, stroke_data: vec4<f32>) -> f32 {
    let width = stroke_data.x;
    if width <= 0.0 {
        return 0.0;
    }
    let offset = stroke_offset(width, stroke_data.z);
    let stroke_d = abs(d - offset) - width * 0.5;
    let alpha = 1.0 - smoothstep(-0.5, 0.5, stroke_d);
    return alpha * dash_mask(local, width, stroke_data.y, stroke_data.w);
}

fn overlay_stroke(fill: vec4<f32>, stroke: vec4<f32>, amount: f32) -> vec4<f32> {
    let stroke_alpha = stroke.a * amount;
    let out_alpha = stroke_alpha + fill.a * (1.0 - stroke_alpha);
    let rgb = stroke.rgb * stroke_alpha + fill.rgb * (1.0 - stroke_alpha);
    return vec4<f32>(rgb, out_alpha);
}

// Slash fill: 45-degree diagonal stripe in logical pixels relative to rect centre.
// pattern_a = [r, g, b, a] linear colour of the stripe.
// pattern_b = [period, line_width, 0, 0].
fn slash_color(local: vec2<f32>, pa: vec4<f32>, pb: vec4<f32>) -> vec4<f32> {
    let period = pb.x;
    let line_width = pb.y;
    let p = (local.x - local.y) / period;
    let phase = (p - floor(p)) * period;
    let edge_dist = min(phase, period - phase) - line_width * 0.5;
    let aa = fwidth(phase) * 0.5;
    let mask = 1.0 - smoothstep(-aa, aa, edge_dist);
    return mix(vec4<f32>(0.0, 0.0, 0.0, 0.0), pa, mask);
}

// Checker fill: alternating cells in logical pixels relative to rect centre.
// pattern_a = [r_a, g_a, b_a, a_a] linear colour A.
// pattern_b = [r_b, g_b, b_b, cell_size] colour B + cell size packed in w.
fn checker_color(local: vec2<f32>, pa: vec4<f32>, pb: vec4<f32>) -> vec4<f32> {
    let cell = pb.w;
    let color_b = vec4<f32>(pb.rgb, 1.0);
    let cx = floor(local.x / cell);
    let cy = floor(local.y / cell);
    let parity = (cx + cy) - 2.0 * floor((cx + cy) * 0.5);
    return select(color_b, pa, parity == 0.0);
}

@fragment
fn fs_main(in: VertexOut) -> @location(0) vec4<f32> {
    let d = rounded_rect_sdf(in.local, in.half_size, in.corner_radii);
    let coverage = 1.0 - smoothstep(-0.5, 0.5, d);

    var color: vec4<f32>;
    if in.fill_kind == FILL_KIND_SLASH {
        color = slash_color(in.local, in.pattern_a, in.pattern_b);
    } else if in.fill_kind == FILL_KIND_CHECKER {
        color = checker_color(in.local, in.pattern_a, in.pattern_b);
    } else {
        // FILL_KIND_GRADIENT (default)
        let axis_len = max(dot(in.axis, in.axis), 0.0001);
        let t = clamp(dot(in.pos_px - in.start, in.axis) / axis_len, 0.0, 1.0);
        color = sample_gradient(in.stops_meta.x, in.stops_meta.y, t);
    }

    let alpha = color.a * in.opacity * coverage;
    let scale = in.opacity * coverage;
    let dither = hash_xy(in.clip.xy) * (2.0 / 255.0) - (1.0 / 255.0);
    let rgb = clamp(color.rgb * scale + vec3<f32>(dither * alpha), vec3<f32>(0.0), vec3<f32>(1.0));
    let fill = vec4<f32>(rgb, alpha);
    let amount = stroke_mask(d, in.local, in.stroke_meta);
    let stroke = vec4<f32>(in.stroke_params.rgb, in.stroke_params.a * in.opacity);
    return overlay_stroke(fill, stroke, amount);
}
