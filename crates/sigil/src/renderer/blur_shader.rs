pub(crate) const BLUR_SHADER: &str = concat!(
    vertex_in!(),
    crate::renderer::fill_paint::srgb_wgsl!(),
    r"
struct BlurUniforms {
    viewport: vec2<f32>,
    params: vec2<f32>,
};

@group(0) @binding(0) var<uniform> u: BlurUniforms;
@group(1) @binding(0) var src: texture_2d<f32>;
@group(1) @binding(1) var samp: sampler;

struct BlurInstance {
    @location(1) dest: vec4<f32>,
    @location(2) uv_max: vec2<f32>,
    @location(3) opacity: f32,
};


struct BlurVertexOut {
    @builtin(position) clip: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@vertex
fn vs_main(v: VertexIn, inst: BlurInstance) -> BlurVertexOut {
    var out: BlurVertexOut;
    let pos_px = inst.dest.xy + v.unit * inst.dest.zw;
    let ndc = vec2<f32>(
        (pos_px.x / u.viewport.x) * 2.0 - 1.0,
        1.0 - (pos_px.y / u.viewport.y) * 2.0,
    );
    out.clip = vec4<f32>(ndc, 0.0, 1.0);
    out.uv = v.unit * inst.uv_max;
    return out;
}

fn encoded(texel: vec4<f32>) -> vec4<f32> {
    if texel.a <= 0.0 {
        return vec4<f32>(0.0);
    }
    let color = clamp(texel.rgb / texel.a, vec3<f32>(0.0), vec3<f32>(1.0));
    return vec4<f32>(srgb_encode(color) * texel.a, texel.a);
}

fn decoded(texel: vec4<f32>) -> vec4<f32> {
    if texel.a <= 0.0 {
        return vec4<f32>(0.0);
    }
    let color = clamp(texel.rgb / texel.a, vec3<f32>(0.0), vec3<f32>(1.0));
    return vec4<f32>(srgb_decode(color) * texel.a, texel.a);
}

@fragment
fn fs_box(in: BlurVertexOut) -> @location(0) vec4<f32> {
    let size = vec2<i32>(textureDimensions(src, 0));
    let center = vec2<i32>(floor(in.clip.xy));
    let radius = i32(u.params.x);
    let flags = u32(u.params.y);
    let step = select(vec2<i32>(0, 1), vec2<i32>(1, 0), (flags & 1u) != 0u);
    let encode_in = (flags & 2u) != 0u;
    let at = dot(center, step);
    let reach = min(radius, i32(flags >> 3u));
    let first = max(-reach, -at);
    let last = min(reach, dot(size, step) - 1 - at);
    var sum = vec4<f32>(0.0);
    for (var i = first; i <= last; i = i + 1) {
        let texel = textureLoad(src, center + step * i, 0);
        sum = sum + select(texel, encoded(texel), encode_in);
    }
    let mean = sum / f32(2 * reach + 1);
    return select(mean, decoded(mean), (flags & 4u) != 0u);
}

fn tap(at: vec2<i32>) -> vec4<f32> {
    let size = vec2<i32>(textureDimensions(src, 0));
    if any(at < vec2<i32>(0)) || any(at >= size) {
        return vec4<f32>(0.0);
    }
    return textureLoad(src, at, 0);
}

@fragment
fn fs_down(in: BlurVertexOut) -> @location(0) vec4<f32> {
    let origin = vec2<i32>(floor(in.clip.xy)) * 2;
    let encode_in = (u32(u.params.y) & 2u) != 0u;
    var sum = vec4<f32>(0.0);
    for (var i = 0; i < 4; i = i + 1) {
        let texel = tap(origin + vec2<i32>(i & 1, i >> 1u));
        sum = sum + select(texel, encoded(texel), encode_in);
    }
    return sum * 0.25;
}

@fragment
fn fs_up(in: BlurVertexOut) -> @location(0) vec4<f32> {
    let pos = in.clip.xy / u.params.x - 0.5;
    let base = vec2<i32>(floor(pos));
    let t = pos - floor(pos);
    let top = mix(tap(base), tap(base + vec2<i32>(1, 0)), t.x);
    let bottom = mix(tap(base + vec2<i32>(0, 1)), tap(base + vec2<i32>(1, 1)), t.x);
    let value = mix(top, bottom, t.y);
    return select(value, decoded(value), (u32(u.params.y) & 4u) != 0u);
}
",
);
