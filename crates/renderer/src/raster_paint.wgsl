// Raster glyph-atlas text renderer — Wave-3 D2 (DR-055).
//
// Samples pre-hinted, pre-antialiased 8-bit coverage masks from a shared R8
// atlas (see raster_paint/atlas.rs). No stem-darkening gamma: hinted masks
// are already grid-fit at rasterization time (DR-055 §4) — the fragment
// shader only multiplies atlas coverage by the instance color alpha.
//
// Bindings:
//   group(0) binding(0) — SurfaceParams uniform (vertex)
//   group(0) binding(1) — atlas texture, R8Unorm (fragment)
//   group(0) binding(2) — atlas sampler, nearest (fragment)
//
// Vertex buffers:
//   slot 0 — unit UV quad (2 x f32, step = Vertex)
//   slot 1 — RasterInstance (48 bytes, step = Instance):
//     @location(1) quad_rect vec4<f32>  logical px [x, y, w, h]
//     @location(2) uv_rect   vec4<f32>  atlas UV [u0, v0, u1, v1]
//     @location(3) color     vec4<f32>  RGBA linear, alpha pre-multiplies coverage

struct SurfaceParams {
    surface_size: vec2<f32>,
    _pad:         vec2<f32>,
}

@group(0) @binding(0) var<uniform> surface:       SurfaceParams;
@group(0) @binding(1) var          atlas_tex:      texture_2d<f32>;
@group(0) @binding(2) var          atlas_sampler:  sampler;

struct VertexOutput {
    @builtin(position) clip_pos: vec4<f32>,
    @location(0)       uv:       vec2<f32>,
    @location(1)       color:    vec4<f32>,
}

@vertex
fn vs_main(
    @location(0) uv:        vec2<f32>,
    @location(1) quad_rect: vec4<f32>,
    @location(2) uv_rect:   vec4<f32>,
    @location(3) color:     vec4<f32>,
) -> VertexOutput {
    let px = quad_rect.xy + uv * quad_rect.zw;
    let clip = vec2(
         px.x / surface.surface_size.x * 2.0 - 1.0,
        -px.y / surface.surface_size.y * 2.0 + 1.0,
    );
    let sample_uv = mix(uv_rect.xy, uv_rect.zw, uv);
    return VertexOutput(vec4(clip, 0.0, 1.0), sample_uv, color);
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    let coverage = textureSample(atlas_tex, atlas_sampler, in.uv).r;
    if coverage < 1e-4 {
        return vec4(0.0);
    }
    return vec4(in.color.rgb, in.color.a * coverage);
}
