// Sprite pipeline shader — three fragment branches keyed on sprite_kind.
//
// SPRITE_KIND_MONO (0):
//   coverage = atlas.a; output = premul(color.rgb, color.a * coverage * opacity)
// SPRITE_KIND_SUBPIXEL (1):
//   three phase-offset alpha samples for R/G/B subpixel coverage
// SPRITE_KIND_POLY (2):
//   full RGBA from atlas; color tint ignored; output premultiplied by opacity

const SPRITE_KIND_MONO: u32 = 0u;
const SPRITE_KIND_SUBPIXEL: u32 = 1u;
const SPRITE_KIND_POLY: u32 = 2u;

struct Uniforms {
    viewport: vec2<f32>,
    atlas_size: vec2<f32>,
};

@group(0) @binding(0) var<uniform> u: Uniforms;
@group(1) @binding(0) var atlas_tex: texture_2d<f32>;
@group(1) @binding(1) var atlas_sampler: sampler;

struct VertexIn {
    @location(0) unit: vec2<f32>,
};

struct InstanceIn {
    // dest rect [x, y, w, h] in logical pixels
    @location(1) dest: vec4<f32>,
    // atlas uv rect [u0, v0, u1, v1] in normalised [0,1] atlas space
    @location(2) uv_rect: vec4<f32>,
    // [r, g, b, a] linear tint colour in [0,1]
    @location(3) color: vec4<f32>,
    // [opacity_01, sprite_kind_f32, subpixel_offset_x, 0]
    @location(4) params: vec4<f32>,
};

struct VertexOut {
    @builtin(position) clip: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) color: vec4<f32>,
    @location(2) opacity: f32,
    @location(3) @interpolate(flat) sprite_kind: u32,
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
    // interpolate UV across the atlas region
    let sprite_kind = u32(inst.params.y);
    let uv = inst.uv_rect.xy + v.unit * (inst.uv_rect.zw - inst.uv_rect.xy);
    let dx = 1.0 / u.atlas_size.x;
    out.uv = uv;
    if sprite_kind == SPRITE_KIND_SUBPIXEL {
        out.uv = vec2<f32>(uv.x + inst.params.z * dx, uv.y);
    }
    out.color = inst.color;
    out.opacity = inst.params.x;
    out.sprite_kind = sprite_kind;
    return out;
}

@fragment
fn fs_main(in: VertexOut) -> @location(0) vec4<f32> {
    if in.sprite_kind == SPRITE_KIND_MONO {
        let coverage = textureSampleLevel(atlas_tex, atlas_sampler, in.uv, 0.0).a;
        let a = in.color.a * coverage * in.opacity;
        return vec4<f32>(in.color.rgb * a, a);
    }
    if in.sprite_kind == SPRITE_KIND_SUBPIXEL {
        let dx = 1.0 / u.atlas_size.x;
        let cr = textureSampleLevel(atlas_tex, atlas_sampler, in.uv - vec2<f32>(dx / 3.0, 0.0), 0.0).a;
        let cg = textureSampleLevel(atlas_tex, atlas_sampler, in.uv, 0.0).a;
        let cb = textureSampleLevel(atlas_tex, atlas_sampler, in.uv + vec2<f32>(dx / 3.0, 0.0), 0.0).a;
        let rgb = in.color.rgb * vec3<f32>(cr, cg, cb);
        let a = in.color.a * max(cr, max(cg, cb)) * in.opacity;
        return vec4<f32>(rgb * a, a);
    }
    // SPRITE_KIND_POLY — atlas RGBA, tint ignored, premultiplied by opacity
    let texel = textureSampleLevel(atlas_tex, atlas_sampler, in.uv, 0.0);
    let alpha = texel.a * in.opacity;
    return vec4<f32>(texel.rgb * alpha, alpha);
}
