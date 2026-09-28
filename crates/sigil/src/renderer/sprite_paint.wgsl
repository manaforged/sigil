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
    @location(1) dest: vec4<f32>,
    @location(2) uv_rect: vec4<f32>,
    @location(3) color: vec4<f32>,
    @location(4) params: vec4<f32>,
    @location(5) xform_x: vec4<f32>,
    @location(6) xform_y: vec4<f32>,
};

struct VertexOut {
    @builtin(position) clip: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) color: vec4<f32>,
    @location(2) opacity: f32,
    @location(3) @interpolate(flat) sprite_kind: u32,
    @location(4) @interpolate(flat) uv_bounds: vec4<f32>,
};

fn texel(bounds: vec4<f32>, uv: vec2<f32>) -> vec4<f32> {
    return textureSampleLevel(atlas_tex, atlas_sampler, clamp(uv, bounds.xy, bounds.zw), 0.0);
}

@vertex
fn vs_main(v: VertexIn, inst: InstanceIn) -> VertexOut {
    var out: VertexOut;
    let local_px = inst.dest.xy + v.unit * inst.dest.zw;
    let pos_px = vec2<f32>(
        dot(inst.xform_x.xy, local_px) + inst.xform_x.z,
        dot(inst.xform_y.xy, local_px) + inst.xform_y.z,
    );
    let ndc = vec2<f32>(
        (pos_px.x / u.viewport.x) * 2.0 - 1.0,
        1.0 - (pos_px.y / u.viewport.y) * 2.0,
    );
    out.clip = vec4<f32>(ndc, 0.0, 1.0);
    let sprite_kind = u32(inst.params.y);
    let uv = inst.uv_rect.xy + v.unit * (inst.uv_rect.zw - inst.uv_rect.xy);
    let dx = 1.0 / u.atlas_size.x;
    out.uv = uv;
    if sprite_kind == SPRITE_KIND_SUBPIXEL {
        out.uv = vec2<f32>(uv.x + inst.params.z * dx, uv.y);
    }
    let half_texel = 0.5 / u.atlas_size;
    let lo = min(inst.uv_rect.xy, inst.uv_rect.zw);
    let hi = max(inst.uv_rect.xy, inst.uv_rect.zw);
    let mid = (lo + hi) * 0.5;
    out.uv_bounds = vec4<f32>(min(lo + half_texel, mid), max(hi - half_texel, mid));
    out.color = inst.color;
    out.opacity = inst.params.x;
    out.sprite_kind = sprite_kind;
    return out;
}

@fragment
fn fs_main(in: VertexOut) -> @location(0) vec4<f32> {
    if in.sprite_kind == SPRITE_KIND_MONO {
        let coverage = texel(in.uv_bounds, in.uv).a;
        let a = in.color.a * coverage * in.opacity;
        return vec4<f32>(in.color.rgb * a, a);
    }
    if in.sprite_kind == SPRITE_KIND_SUBPIXEL {
        let dx = 1.0 / u.atlas_size.x;
        let cr = texel(in.uv_bounds, in.uv - vec2<f32>(dx / 3.0, 0.0)).a;
        let cg = texel(in.uv_bounds, in.uv).a;
        let cb = texel(in.uv_bounds, in.uv + vec2<f32>(dx / 3.0, 0.0)).a;
        let rgb = in.color.rgb * vec3<f32>(cr, cg, cb);
        let a = in.color.a * max(cr, max(cg, cb)) * in.opacity;
        return vec4<f32>(rgb * a, a);
    }
    let px = texel(in.uv_bounds, in.uv);
    let alpha = px.a * in.opacity;
    return vec4<f32>(px.rgb * alpha, alpha);
}
