#![allow(unreachable_pub, clippy::redundant_pub_crate)]

#[allow(unreachable_pub, reason = "intentional, reviewed trade-off")]
pub(crate) const BLUR_SHADER: &str = r"
struct Uniforms {
    viewport: vec2<f32>,
    _pad: vec2<f32>,
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
fn fs_down(in: VertexOut) -> @location(0) vec4<f32> {
    let tex_size = vec2<f32>(textureDimensions(src, 0));
    let half_pixel = 0.5 / tex_size;
    let center = textureSample(src, samp, in.uv) * 4.0;
    let corners =
          textureSample(src, samp, in.uv - half_pixel)
        + textureSample(src, samp, in.uv + half_pixel)
        + textureSample(src, samp, in.uv + vec2<f32>( half_pixel.x, -half_pixel.y))
        + textureSample(src, samp, in.uv + vec2<f32>(-half_pixel.x,  half_pixel.y));
    return (center + corners) / 8.0;
}

@fragment
fn fs_up(in: VertexOut) -> @location(0) vec4<f32> {
    let tex_size = vec2<f32>(textureDimensions(src, 0));
    let half_pixel = 0.5 / tex_size;
    let cardinals =
          textureSample(src, samp, in.uv + vec2<f32>(-half_pixel.x * 2.0, 0.0))
        + textureSample(src, samp, in.uv + vec2<f32>( half_pixel.x * 2.0, 0.0))
        + textureSample(src, samp, in.uv + vec2<f32>(0.0,  half_pixel.y * 2.0))
        + textureSample(src, samp, in.uv + vec2<f32>(0.0, -half_pixel.y * 2.0));
    let corners =
          textureSample(src, samp, in.uv + vec2<f32>(-half_pixel.x,  half_pixel.y))
        + textureSample(src, samp, in.uv + vec2<f32>( half_pixel.x,  half_pixel.y))
        + textureSample(src, samp, in.uv + vec2<f32>(-half_pixel.x, -half_pixel.y))
        + textureSample(src, samp, in.uv + vec2<f32>( half_pixel.x, -half_pixel.y));
    return (cardinals * 2.0 + corners) / 12.0;
}
";
