pub(super) const LAYER_SHADER: &str = r"
struct Uniforms {
    viewport: vec2<f32>,
    _pad: vec2<f32>,
};

@group(0) @binding(0) var<uniform> u: Uniforms;
@group(1) @binding(0) var layer_tex: texture_2d<f32>;
@group(1) @binding(1) var layer_sampler: sampler;

struct VertexIn {
    @location(0) unit: vec2<f32>,
};

struct InstanceIn {
    @location(1) dest: vec4<f32>,
    @location(2) clip_radii: vec4<f32>,
    @location(3) uv_max: vec2<f32>,
    @location(4) opacity: f32,
    @location(5) transform_x: vec3<f32>,
    @location(6) transform_y: vec3<f32>,
    @location(7) round: vec4<f32>,
};

struct VertexOut {
    @builtin(position) clip: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) opacity: f32,
    @location(2) local: vec2<f32>,
    @location(3) half_size: vec2<f32>,
    @location(4) clip_radii: vec4<f32>,
};

@vertex
fn vs_main(v: VertexIn, inst: InstanceIn) -> VertexOut {
    var out: VertexOut;
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
    return out;
}

fn corner_radius(p: vec2<f32>, radii: vec4<f32>) -> f32 {
    let bottom = select(radii.w, radii.z, p.x > 0.0);
    let top = select(radii.x, radii.y, p.x > 0.0);
    return select(bottom, top, p.y < 0.0);
}

fn rounded_rect_sdf(p: vec2<f32>, half: vec2<f32>, radii: vec4<f32>) -> f32 {
    let radius = corner_radius(p, radii);
    let q = abs(p) - half + vec2<f32>(radius);
    return length(max(q, vec2<f32>(0.0))) + min(max(q.x, q.y), 0.0) - radius;
}

@fragment
fn fs_main(in: VertexOut) -> @location(0) vec4<f32> {
    let distance = rounded_rect_sdf(in.local, in.half_size, in.clip_radii);
    let aa = max(fwidth(distance) * 0.5, 0.0001);
    let coverage = 1.0 - smoothstep(-aa, aa, distance);
    let amount = in.opacity * coverage;
    let texel = textureSample(layer_tex, layer_sampler, in.uv);
    return texel * amount;
}
";
