pub(super) const RECT_SHADER: &str = "\
const STROKE_STYLE_SOLID: f32 = 0.0;
const STROKE_STYLE_DASHED: f32 = 1.0;
const STROKE_ALIGN_CENTERED: f32 = 0.0;
const STROKE_ALIGN_INSIDE: f32 = 1.0;
const STROKE_ALIGN_OUTSIDE: f32 = 2.0;

struct Uniforms {
    viewport: vec2<f32>,
    _pad: vec2<f32>,
};

@group(0) @binding(0) var<uniform> u: Uniforms;

struct VertexIn {
    @location(0) unit: vec2<f32>,
};

struct InstanceIn {
    @location(1) rect: vec4<f32>,
    @location(2) color: vec4<f32>,
    @location(3) corner_radii: vec4<f32>,
    @location(4) params: vec4<f32>,
    @location(5) stroke_params: vec4<f32>,
    @location(6) stroke_meta: vec4<f32>,
};

struct VertexOut {
    @builtin(position) clip: vec4<f32>,
    @location(0) color: vec4<f32>,
    @location(1) local: vec2<f32>,
    @location(2) half_size: vec2<f32>,
    @location(3) corner_radii: vec4<f32>,
    @location(4) opacity: f32,
    @location(5) stroke_params: vec4<f32>,
    @location(6) stroke_meta: vec4<f32>,
};

@vertex
fn vs_main(v: VertexIn, inst: InstanceIn) -> VertexOut {
    var out: VertexOut;
    let pos_px = inst.rect.xy + v.unit * inst.rect.zw;
    let ndc = vec2<f32>(
        (pos_px.x / u.viewport.x) * 2.0 - 1.0,
        1.0 - (pos_px.y / u.viewport.y) * 2.0,
    );
    out.clip = vec4<f32>(ndc, 0.0, 1.0);
    out.color = inst.color;
    let expanded_half = inst.rect.zw * 0.5;
    let outset = inst.params.y;
    let surface_half = max(expanded_half - vec2<f32>(outset, outset), vec2<f32>(0.0, 0.0));
    out.local = v.unit * inst.rect.zw - expanded_half;
    out.half_size = surface_half;
    out.corner_radii = inst.corner_radii;
    out.opacity = inst.params.x;
    out.stroke_params = inst.stroke_params;
    out.stroke_meta = inst.stroke_meta;
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
    if out_alpha <= 0.0 {
        return vec4<f32>(0.0);
    }
    let rgb = (stroke.rgb * stroke_alpha + fill.rgb * fill.a * (1.0 - stroke_alpha)) / out_alpha;
    return vec4<f32>(rgb, out_alpha);
}

@fragment
fn fs_main(in: VertexOut) -> @location(0) vec4<f32> {
    let d = rounded_rect_sdf(in.local, in.half_size, in.corner_radii);
    let coverage = 1.0 - smoothstep(-0.5, 0.5, d);
    let fill = vec4<f32>(in.color.rgb, in.color.a * in.opacity * coverage);
    let amount = stroke_mask(d, in.local, in.stroke_meta);
    let stroke = vec4<f32>(in.stroke_params.rgb, in.stroke_params.a * in.opacity);
    return overlay_stroke(fill, stroke, amount);
}
";
