macro_rules! blend_wgsl {
    () => {
        r"
fn srgb_encode(c: vec3<f32>) -> vec3<f32> {
    let low = c * 12.92;
    let high = 1.055 * pow(c, vec3<f32>(1.0 / 2.4)) - 0.055;
    return select(high, low, c <= vec3<f32>(0.0031308));
}

fn srgb_decode(c: vec3<f32>) -> vec3<f32> {
    let low = c / 12.92;
    let high = pow((c + 0.055) / 1.055, vec3<f32>(2.4));
    return select(high, low, c <= vec3<f32>(0.04045));
}

fn encoded(c: vec4<f32>) -> vec3<f32> {
    return srgb_encode(clamp(c.rgb / max(c.a, 0.000001), vec3<f32>(0.0), vec3<f32>(1.0)));
}

fn screen(cb: vec3<f32>, cs: vec3<f32>) -> vec3<f32> {
    return cb + cs - cb * cs;
}

fn hard_light(cb: vec3<f32>, cs: vec3<f32>) -> vec3<f32> {
    return select(screen(cb, 2.0 * cs - 1.0), cb * 2.0 * cs, cs <= vec3<f32>(0.5));
}

fn soft_light(cb: vec3<f32>, cs: vec3<f32>) -> vec3<f32> {
    let d = select(sqrt(cb), ((16.0 * cb - 12.0) * cb + 4.0) * cb, cb <= vec3<f32>(0.25));
    let lift = cb + (2.0 * cs - 1.0) * (d - cb);
    return select(lift, cb - (1.0 - 2.0 * cs) * cb * (1.0 - cb), cs <= vec3<f32>(0.5));
}

fn color_dodge(cb: vec3<f32>, cs: vec3<f32>) -> vec3<f32> {
    let lift = min(vec3<f32>(1.0), cb / max(1.0 - cs, vec3<f32>(0.000001)));
    return select(select(lift, vec3<f32>(1.0), cs >= vec3<f32>(1.0)), vec3<f32>(0.0), cb <= vec3<f32>(0.0));
}

fn color_burn(cb: vec3<f32>, cs: vec3<f32>) -> vec3<f32> {
    let burnt = 1.0 - min(vec3<f32>(1.0), (1.0 - cb) / max(cs, vec3<f32>(0.000001)));
    return select(select(burnt, vec3<f32>(0.0), cs <= vec3<f32>(0.0)), vec3<f32>(1.0), cb >= vec3<f32>(1.0));
}

fn lum(c: vec3<f32>) -> f32 {
    return dot(c, vec3<f32>(0.3, 0.59, 0.11));
}

fn clip_color(c: vec3<f32>) -> vec3<f32> {
    let l = lum(c);
    let n = min(c.r, min(c.g, c.b));
    let x = max(c.r, max(c.g, c.b));
    var out = c;
    if n < 0.0 {
        out = l + (out - l) * l / (l - n);
    }
    if x > 1.0 {
        out = l + (out - l) * (1.0 - l) / (x - l);
    }
    return out;
}

fn set_lum(c: vec3<f32>, l: f32) -> vec3<f32> {
    return clip_color(c + (l - lum(c)));
}

fn sat(c: vec3<f32>) -> f32 {
    return max(c.r, max(c.g, c.b)) - min(c.r, min(c.g, c.b));
}

fn set_sat(c: vec3<f32>, s: f32) -> vec3<f32> {
    let low = min(c.r, min(c.g, c.b));
    let high = max(c.r, max(c.g, c.b));
    return select(vec3<f32>(0.0), (c - low) * s / max(high - low, 0.000001), high > low);
}

fn blend_color(cb: vec3<f32>, cs: vec3<f32>, mode: u32) -> vec3<f32> {
    switch mode {
        case 1u: { return cb * cs; }
        case 2u: { return screen(cb, cs); }
        case 3u: { return hard_light(cs, cb); }
        case 4u: { return min(cb, cs); }
        case 5u: { return max(cb, cs); }
        case 6u: { return color_dodge(cb, cs); }
        case 7u: { return color_burn(cb, cs); }
        case 8u: { return hard_light(cb, cs); }
        case 9u: { return soft_light(cb, cs); }
        case 10u: { return abs(cb - cs); }
        case 11u: { return cb + cs - 2.0 * cb * cs; }
        case 12u: { return set_lum(set_sat(cs, sat(cb)), lum(cb)); }
        case 13u: { return set_lum(set_sat(cb, sat(cs)), lum(cb)); }
        case 14u: { return set_lum(cs, lum(cb)); }
        case 15u: { return set_lum(cb, lum(cs)); }
        default: { return cs; }
    }
}

fn blend_over(backdrop: vec4<f32>, source: vec4<f32>, mode: u32) -> vec4<f32> {
    let ba = backdrop.a;
    let sa = source.a;
    let cb = encoded(backdrop);
    let cs = encoded(source);
    let mixed = (1.0 - ba) * cs + ba * blend_color(cb, cs, mode);
    let co = sa * mixed + (1.0 - sa) * ba * cb;
    let ao = sa + ba * (1.0 - sa);
    if ao <= 0.0 {
        return vec4<f32>(0.0);
    }
    return vec4<f32>(srgb_decode(clamp(co / ao, vec3<f32>(0.0), vec3<f32>(1.0))) * ao, ao);
}
"
    };
}

pub(super) const LAYER_SHADER: &str = concat!(
    viewport_uniforms!(),
    vertex_in!(),
    crate::renderer::fill_paint::sdf_helpers!(),
    r"
@group(1) @binding(0) var layer_tex: texture_2d<f32>;
@group(1) @binding(1) var layer_sampler: sampler;
@group(2) @binding(0) var mask_tex: texture_2d<f32>;
@group(2) @binding(1) var mask_sampler: sampler;
@group(3) @binding(0) var backdrop_tex: texture_2d<f32>;
@group(3) @binding(1) var backdrop_sampler: sampler;


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
    @location(10) backdrop: vec2<f32>,
    @location(11) blend: u32,
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
    @location(7) @interpolate(flat) backdrop: vec2<f32>,
    @location(8) @interpolate(flat) blend: u32,
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
    out.backdrop = inst.backdrop;
    out.blend = inst.blend;
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

fn mask_cover(in: LayerVertexOut) -> f32 {
    let uv = (in.clip.xy - in.mask.xy) / max(in.mask.zw, vec2<f32>(1.0));
    let inside = all(uv >= vec2<f32>(0.0)) && all(uv <= vec2<f32>(1.0));
    let texel = textureSampleLevel(mask_tex, mask_sampler, uv, 0.0);
    let cover = select(texel.a, mask_luminance(texel), in.luminance > 0.5);
    return select(0.0, cover, inside);
}

@fragment
fn fs_masked(in: LayerVertexOut) -> @location(0) vec4<f32> {
    return layer_color(in) * mask_cover(in);
}

@fragment
fn fs_blend(in: LayerVertexOut) -> @location(0) vec4<f32> {
    let color = layer_color(in);
    let source = select(color, color * mask_cover(in), in.mask.z > 0.0);
    let last = vec2<i32>(textureDimensions(backdrop_tex)) - vec2<i32>(1);
    let at = clamp(vec2<i32>(floor(in.clip.xy - in.backdrop)), vec2<i32>(0), last);
    let behind = textureLoad(backdrop_tex, at, 0);
    return blend_over(behind, source, in.blend);
}

fn mask_luminance(texel: vec4<f32>) -> f32 {
    let linear = clamp(texel.rgb / max(texel.a, 0.0001), vec3<f32>(0.0), vec3<f32>(1.0));
    let low = linear * 12.92;
    let high = 1.055 * pow(linear, vec3<f32>(1.0 / 2.4)) - 0.055;
    let encoded = select(high, low, linear <= vec3<f32>(0.0031308));
    return dot(encoded, vec3<f32>(0.2125, 0.7154, 0.0721)) * texel.a;
}
",
    blend_wgsl!(),
);
