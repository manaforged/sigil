pub(crate) const FILTER_SHADER: &str = concat!(
    vertex_in!(),
    r"
struct FilterUniforms {
    viewport: vec2<f32>,
    offset: vec2<f32>,
    tint: vec4<f32>,
    count: vec4<u32>,
    rows: array<vec4<f32>, 40>,
};

@group(0) @binding(0) var<uniform> u: FilterUniforms;
@group(1) @binding(0) var src: texture_2d<f32>;
@group(1) @binding(1) var src_sampler: sampler;
@group(2) @binding(0) var shadow: texture_2d<f32>;
@group(2) @binding(1) var shadow_sampler: sampler;

struct FilterInstance {
    @location(1) dest: vec4<f32>,
};

struct FilterOut {
    @builtin(position) clip: vec4<f32>,
};

@vertex
fn vs_main(v: VertexIn, inst: FilterInstance) -> FilterOut {
    var out: FilterOut;
    let pos_px = inst.dest.xy + v.unit * inst.dest.zw;
    let ndc = vec2<f32>(
        (pos_px.x / u.viewport.x) * 2.0 - 1.0,
        1.0 - (pos_px.y / u.viewport.y) * 2.0,
    );
    out.clip = vec4<f32>(ndc, 0.0, 1.0);
    return out;
}

fn encode(c: vec3<f32>) -> vec3<f32> {
    let v = clamp(c, vec3<f32>(0.0), vec3<f32>(1.0));
    return select(1.055 * pow(v, vec3<f32>(1.0 / 2.4)) - 0.055, v * 12.92, v <= vec3<f32>(0.0031308));
}

fn decode(c: vec3<f32>) -> vec3<f32> {
    return select(pow((c + 0.055) / 1.055, vec3<f32>(2.4)), c / 12.92, c <= vec3<f32>(0.04045));
}

fn transform(c: vec4<f32>, base: u32) -> vec4<f32> {
    let bias = u.rows[base + 4u];
    let out = vec4<f32>(
        dot(u.rows[base], c),
        dot(u.rows[base + 1u], c),
        dot(u.rows[base + 2u], c),
        dot(u.rows[base + 3u], c),
    ) + bias;
    return clamp(out, vec4<f32>(0.0), vec4<f32>(1.0));
}

@fragment
fn fs_matrix(in: FilterOut) -> @location(0) vec4<f32> {
    let px = textureLoad(src, vec2<i32>(floor(in.clip.xy)), 0);
    if px.a <= 0.0 {
        return vec4<f32>(0.0);
    }
    let alpha = min(px.a, 1.0);
    var c = vec4<f32>(encode(px.rgb / alpha), alpha);
    for (var i = 0u; i < u.count.x; i = i + 1u) {
        c = transform(c, i * 5u);
    }
    return vec4<f32>(decode(c.rgb) * c.a, c.a);
}

@fragment
fn fs_shadow(in: FilterOut) -> @location(0) vec4<f32> {
    let p = vec2<i32>(floor(in.clip.xy));
    let top = textureLoad(src, p, 0);
    let q = p - vec2<i32>(u.offset);
    let size = vec2<i32>(textureDimensions(shadow, 0));
    var coverage = 0.0;
    if all(q >= vec2<i32>(0)) && all(q < size) {
        coverage = textureLoad(shadow, q, 0).a;
    }
    return top + u.tint * coverage * (1.0 - top.a);
}
",
);
