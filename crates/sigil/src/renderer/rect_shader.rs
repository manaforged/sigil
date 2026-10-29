pub(super) const RECT_SHADER: &str = concat!(
    "\
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
    @location(7) xform_x: vec4<f32>,
    @location(8) xform_y: vec4<f32>,
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
    @location(7) dash_origin: vec2<f32>,
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
    out.clip = vec4<f32>(ndc, 0.0, 1.0);
    out.color = inst.color;
    let expanded_half = inst.rect.zw * 0.5;
    let outset = inst.params.y;
    let surface_half = max(expanded_half - vec2<f32>(outset, outset), vec2<f32>(0.0, 0.0));
    out.local = corner - expanded_half;
    out.half_size = surface_half;
    out.corner_radii = inst.corner_radii;
    out.opacity = inst.params.x;
    out.stroke_params = inst.stroke_params;
    out.stroke_meta = inst.stroke_meta;
    out.dash_origin = inst.params.zw;
    return out;
}

",
    crate::renderer::fill_paint::sdf_helpers!(),
    "fn overlay_stroke(fill: vec4<f32>, stroke: vec4<f32>, amount: f32) -> vec4<f32> {
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
    let aa = max(fwidth(d) * 0.5, 0.0001);
    let coverage = edge_coverage(d, aa);
    let fill = vec4<f32>(in.color.rgb, in.color.a * in.opacity * coverage);
    let amount = stroke_mask(d, in.local + in.dash_origin, in.stroke_meta, aa);
    let stroke = vec4<f32>(in.stroke_params.rgb, in.stroke_params.a * in.opacity);
    return overlay_stroke(fill, stroke, amount);
}
",
);
