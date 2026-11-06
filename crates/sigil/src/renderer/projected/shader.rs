pub(super) const PROJECTED_SHADER: &str = concat!(
    viewport_uniforms!(),
    vertex_in!(),
    r"
@group(1) @binding(0) var layer_tex: texture_2d<f32>;
@group(1) @binding(1) var layer_sampler: sampler;

struct ProjectedInstance {
    @location(1) dest: vec4<f32>,
    @location(2) uv_max: vec2<f32>,
    @location(3) opacity: f32,
    @location(4) row_x: vec3<f32>,
    @location(5) row_y: vec3<f32>,
    @location(6) row_w: vec3<f32>,
};

struct ProjectedOut {
    @builtin(position) clip: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) opacity: f32,
    @location(2) local: vec2<f32>,
    @location(3) half_size: vec2<f32>,
};

@vertex
fn vs_main(v: VertexIn, inst: ProjectedInstance) -> ProjectedOut {
    var out: ProjectedOut;
    let local = vec3<f32>(inst.dest.xy + v.unit * inst.dest.zw, 1.0);
    let x = dot(inst.row_x, local);
    let y = dot(inst.row_y, local);
    let w = dot(inst.row_w, local);
    out.clip = vec4<f32>(x / u.viewport.x * 2.0 - w, w - y / u.viewport.y * 2.0, 0.0, w);
    out.uv = v.unit * inst.uv_max;
    out.opacity = inst.opacity;
    out.half_size = inst.dest.zw * 0.5;
    out.local = local.xy - inst.dest.xy - out.half_size;
    return out;
}

@fragment
fn fs_main(in: ProjectedOut) -> @location(0) vec4<f32> {
    let q = abs(in.local) - in.half_size;
    let distance = max(q.x, q.y);
    let aa = max(fwidth(distance) * 0.5, 0.0001);
    let coverage = 1.0 - smoothstep(-aa, aa, distance);
    return textureSample(layer_tex, layer_sampler, in.uv) * (in.opacity * coverage);
}
",
);
