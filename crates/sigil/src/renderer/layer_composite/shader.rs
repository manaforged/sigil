pub(super) const LAYER_SHADER: &str = concat!(
    viewport_uniforms!(),
    vertex_in!(),
    crate::renderer::fill_paint::sdf_helpers!(),
    r"
@group(1) @binding(0) var layer_tex: texture_2d<f32>;
@group(1) @binding(1) var layer_sampler: sampler;
@group(2) @binding(0) var mask_tex: texture_2d<f32>;
@group(2) @binding(1) var mask_sampler: sampler;


struct LayerInstance {
    @location(1) dest: vec4<f32>,
    @location(2) clip_radii: vec4<f32>,
    @location(3) uv_max: vec2<f32>,
    @location(4) opacity: f32,
    @location(5) transform_x: vec3<f32>,
    @location(6) transform_y: vec3<f32>,
    @location(7) round: vec4<f32>,
    @location(8) mask: vec4<f32>,
    @location(9) luminance: f32,
};

struct LayerVertexOut {
    @builtin(position) clip: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) opacity: f32,
    @location(2) local: vec2<f32>,
    @location(3) half_size: vec2<f32>,
    @location(4) clip_radii: vec4<f32>,
    @location(5) @interpolate(flat) mask: vec4<f32>,
    @location(6) @interpolate(flat) luminance: f32,
};

@vertex
fn vs_main(v: VertexIn, inst: LayerInstance) -> LayerVertexOut {
    var out: LayerVertexOut;
    let local = vec3<f32>(inst.dest.xy + v.unit * inst.dest.zw, 1.0);
    let pos_px = vec2<f32>(dot(inst.transform_x, local), dot(inst.transform_y, local));
    let ndc = vec2<f32>(
        (pos_px.x / u.viewport.x) * 2.0 - 1.0,
        1.0 - (pos_px.y / u.viewport.y) * 2.0,
    );
    out.clip = vec4<f32>(ndc, 0.0, 1.0);
    out.uv = v.unit * inst.uv_max;
    out.opacity = inst.opacity;
    out.local = local.xy - inst.round.xy;
    out.half_size = inst.round.zw;
    out.clip_radii = inst.clip_radii;
    out.mask = inst.mask;
    out.luminance = inst.luminance;
    return out;
}

fn layer_color(in: LayerVertexOut) -> vec4<f32> {
    let distance = rounded_rect_sdf(in.local, in.half_size, in.clip_radii);
    let aa = max(fwidth(distance) * 0.5, 0.0001);
    let coverage = 1.0 - smoothstep(-aa, aa, distance);
    let amount = in.opacity * coverage;
    let texel = textureSample(layer_tex, layer_sampler, in.uv);
    return texel * amount;
}

@fragment
fn fs_main(in: LayerVertexOut) -> @location(0) vec4<f32> {
    return layer_color(in);
}

@fragment
fn fs_masked(in: LayerVertexOut) -> @location(0) vec4<f32> {
    let color = layer_color(in);
    let uv = (in.clip.xy - in.mask.xy) / max(in.mask.zw, vec2<f32>(1.0));
    let inside = all(uv >= vec2<f32>(0.0)) && all(uv <= vec2<f32>(1.0));
    let texel = textureSampleLevel(mask_tex, mask_sampler, uv, 0.0);
    let cover = select(texel.a, mask_luminance(texel), in.luminance > 0.5);
    return color * select(0.0, cover, inside);
}

fn mask_luminance(texel: vec4<f32>) -> f32 {
    let linear = clamp(texel.rgb / max(texel.a, 0.0001), vec3<f32>(0.0), vec3<f32>(1.0));
    let low = linear * 12.92;
    let high = 1.055 * pow(linear, vec3<f32>(1.0 / 2.4)) - 0.055;
    let encoded = select(high, low, linear <= vec3<f32>(0.0031308));
    return dot(encoded, vec3<f32>(0.2125, 0.7154, 0.0721)) * texel.a;
}
",
);
