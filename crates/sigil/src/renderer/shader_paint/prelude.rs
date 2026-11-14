use super::Form;

const BASE: &str = concat!(
    viewport_uniforms!(),
    vertex_in!(),
    crate::renderer::fill_paint::sdf_helpers!(),
    crate::renderer::fill_paint::srgb_wgsl!(),
    r"
struct ShaderInstance {
    @location(1) rect: vec4<f32>,
    @location(2) radii: vec4<f32>,
    @location(3) vars_low: vec4<f32>,
    @location(4) vars_high: vec4<f32>,
    @location(5) color0: vec4<f32>,
    @location(6) color1: vec4<f32>,
    @location(7) color2: vec4<f32>,
    @location(8) color3: vec4<f32>,
    @location(9) params: vec4<f32>,
    @location(10) layer_uv: vec4<f32>,
    @location(11) xform_x: vec4<f32>,
    @location(12) xform_y: vec4<f32>,
};

struct ShaderVertex {
    @builtin(position) clip: vec4<f32>,
    @location(0) pixel: vec2<f32>,
    @location(1) @interpolate(flat) size: vec2<f32>,
    @location(2) @interpolate(flat) radii: vec4<f32>,
    @location(3) @interpolate(flat) vars_low: vec4<f32>,
    @location(4) @interpolate(flat) vars_high: vec4<f32>,
    @location(5) @interpolate(flat) color0: vec4<f32>,
    @location(6) @interpolate(flat) color1: vec4<f32>,
    @location(7) @interpolate(flat) color2: vec4<f32>,
    @location(8) @interpolate(flat) color3: vec4<f32>,
    @location(9) @interpolate(flat) params: vec4<f32>,
    @location(10) @interpolate(flat) layer_uv: vec4<f32>,
};

struct Shading {
    uv: vec2<f32>,
    pixel: vec2<f32>,
    size: vec2<f32>,
    scale: f32,
    vars: array<f32, 8>,
    colors: array<vec4<f32>, 4>,
};

var<private> shading_layer_uv: vec2<f32>;

@vertex
fn vs_main(v: VertexIn, inst: ShaderInstance) -> ShaderVertex {
    var out: ShaderVertex;
    let pad = aa_pad(inst.xform_x.xy, inst.xform_y.xy);
    let corner = v.unit * (inst.rect.zw + 2.0 * pad) - pad;
    let local_px = inst.rect.xy + corner;
    let pos_px = vec2<f32>(
        dot(inst.xform_x.xy, local_px) + inst.xform_x.z,
        dot(inst.xform_y.xy, local_px) + inst.xform_y.z,
    );
    out.clip = vec4<f32>(
        (pos_px.x / u.viewport.x) * 2.0 - 1.0,
        1.0 - (pos_px.y / u.viewport.y) * 2.0,
        0.0,
        1.0,
    );
    out.pixel = corner;
    out.size = inst.rect.zw;
    out.radii = inst.radii;
    out.vars_low = inst.vars_low;
    out.vars_high = inst.vars_high;
    out.color0 = inst.color0;
    out.color1 = inst.color1;
    out.color2 = inst.color2;
    out.color3 = inst.color3;
    out.params = inst.params;
    out.layer_uv = inst.layer_uv;
    return out;
}

@fragment
fn fs_main(in: ShaderVertex) -> @location(0) vec4<f32> {
    let half = in.size * 0.5;
    let d = rounded_rect_sdf(in.pixel - half, half, in.radii);
    let aa = max(fwidth(d) * 0.5, 0.0001);
    let coverage = edge_coverage(d, aa);
    shading_layer_uv = in.layer_uv.xy;
    var s: Shading;
    s.uv = in.pixel / max(in.size, vec2<f32>(0.000001));
    s.pixel = in.pixel;
    s.size = in.size;
    s.scale = in.params.y;
    s.vars = array<f32, 8>(
        in.vars_low.x, in.vars_low.y, in.vars_low.z, in.vars_low.w,
        in.vars_high.x, in.vars_high.y, in.vars_high.z, in.vars_high.w,
    );
    s.colors = array<vec4<f32>, 4>(in.color0, in.color1, in.color2, in.color3);
    let color = clamp(shade(s), vec4<f32>(0.0), vec4<f32>(1.0));
    let alpha = color.a * in.params.x * coverage;
    let rgb = select(color.rgb, srgb_encode(color.rgb), in.params.z > 0.5);
    return vec4<f32>(rgb * alpha, alpha);
}
"
);

const FILL: &str = r"
fn layer(uv: vec2<f32>) -> vec4<f32> {
    return vec4<f32>(0.0);
}
";

const LAYER: &str = r"
@group(1) @binding(0) var layer_texture: texture_2d<f32>;
@group(1) @binding(1) var layer_sampler: sampler;

fn layer(uv: vec2<f32>) -> vec4<f32> {
    let at = clamp(uv, vec2<f32>(0.0), vec2<f32>(1.0)) * shading_layer_uv;
    let texel = textureSampleLevel(layer_texture, layer_sampler, at, 0.0);
    return vec4<f32>(texel.rgb / max(texel.a, 0.000001), texel.a);
}
";

pub(super) fn assemble(form: Form, source: &str) -> String {
    let tail = match form {
        Form::Fill => FILL,
        Form::Layer => LAYER,
    };
    [BASE, tail, source].concat()
}
