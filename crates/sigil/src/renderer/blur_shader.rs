#![allow(unreachable_pub, clippy::redundant_pub_crate)]

#[allow(unreachable_pub, reason = "intentional, reviewed trade-off")]
pub(crate) const BLUR_SHADER: &str = r"
struct Uniforms {
    viewport: vec2<f32>,
    params: vec2<f32>,
};

@group(0) @binding(0) var<uniform> u: Uniforms;
@group(1) @binding(0) var src: texture_2d<f32>;
@group(1) @binding(1) var samp: sampler;

struct InstanceIn {
    @location(1) dest: vec4<f32>,
    @location(2) uv_max: vec2<f32>,
    @location(3) opacity: f32,
};

struct VertexIn {
    @location(0) unit: vec2<f32>,
};

struct VertexOut {
    @builtin(position) clip: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@vertex
fn vs_main(v: VertexIn, inst: InstanceIn) -> VertexOut {
    var out: VertexOut;
    let pos_px = inst.dest.xy + v.unit * inst.dest.zw;
    let ndc = vec2<f32>(
        (pos_px.x / u.viewport.x) * 2.0 - 1.0,
        1.0 - (pos_px.y / u.viewport.y) * 2.0,
    );
    out.clip = vec4<f32>(ndc, 0.0, 1.0);
    out.uv = v.unit * inst.uv_max;
    return out;
}

@fragment
fn fs_box(in: VertexOut) -> @location(0) vec4<f32> {
    let size = vec2<i32>(textureDimensions(src, 0));
    let center = vec2<i32>(floor(in.clip.xy));
    let radius = i32(u.params.x);
    let step = select(vec2<i32>(0, 1), vec2<i32>(1, 0), u.params.y > 0.5);
    var sum = vec4<f32>(0.0);
    for (var i = -radius; i <= radius; i = i + 1) {
        let p = center + step * i;
        if all(p >= vec2<i32>(0)) && all(p < size) {
            sum = sum + textureLoad(src, p, 0);
        }
    }
    return sum / f32(2 * radius + 1);
}
";
