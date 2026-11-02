pub(super) const SHADOW_SHADER: &str = concat!(
    viewport_uniforms!(),
    vertex_in!(),
    crate::renderer::fill_paint::sdf_helpers!(),
    r"
const TAU: f32 = 6.283185307179586;



struct ShadowInstance {
    @location(1) rect: vec4<f32>,
    @location(2) surface_rect: vec4<f32>,
    @location(3) corner_radii: vec4<f32>,
    @location(4) color: vec4<f32>,
    @location(5) shadow_params: vec4<f32>,
    @location(6) shadow_meta: vec4<f32>,
};

struct ShadowVertexOut {
    @builtin(position) clip: vec4<f32>,
    @location(0) pos_px: vec2<f32>,
    @location(1) surface_rect: vec4<f32>,
    @location(2) corner_radii: vec4<f32>,
    @location(3) color: vec4<f32>,
    @location(4) shadow_params: vec4<f32>,
    @location(5) shadow_meta: vec4<f32>,
};

@vertex
fn vs_main(v: VertexIn, inst: ShadowInstance) -> ShadowVertexOut {
    var out: ShadowVertexOut;
    let pos_px = inst.rect.xy + v.unit * inst.rect.zw;
    let ndc = vec2<f32>(
        (pos_px.x / u.viewport.x) * 2.0 - 1.0,
        1.0 - (pos_px.y / u.viewport.y) * 2.0,
    );
    out.clip = vec4<f32>(ndc, 0.0, 1.0);
    out.pos_px = pos_px;
    out.surface_rect = inst.surface_rect;
    out.corner_radii = inst.corner_radii;
    out.color = inst.color;
    out.shadow_params = inst.shadow_params;
    out.shadow_meta = inst.shadow_meta;
    return out;
}

fn erf2(x: vec2<f32>) -> vec2<f32> {
    let s = sign(x);
    let a = abs(x);
    var v = 1.0 + (0.278393 + (0.230389 + 0.078108 * (a * a)) * a) * a;
    v = v * v;
    return s - s / (v * v);
}

fn gaussian_weight(x: f32, sigma: f32) -> f32 {
    return exp(-(x * x) / (2.0 * sigma * sigma)) / (sqrt(TAU) * sigma);
}

fn shadow_x(x: f32, y: f32, sigma: f32, radius: f32, half_size: vec2<f32>) -> f32 {
    let dy = min(half_size.y - radius - abs(y), 0.0);
    let curved = half_size.x - radius + sqrt(max(0.0, radius * radius - dy * dy));
    let integral = 0.5 + 0.5 * erf2((x + vec2<f32>(-curved, curved)) * (0.70710678118 / sigma));
    return integral.y - integral.x;
}

fn blurred_coverage(p: vec2<f32>, sigma: f32, radii: vec4<f32>, half_size: vec2<f32>, spread: f32) -> f32 {
    let low = p.y - half_size.y;
    let high = p.y + half_size.y;
    let start = clamp(-3.0 * sigma, low, high);
    let end = clamp(3.0 * sigma, low, high);
    let step = (end - start) * 0.25;
    var y = start + step * 0.5;
    var sum = 0.0;
    for (var i = 0u; i < 4u; i = i + 1u) {
        let sample_p = vec2<f32>(p.x, p.y - y);
        let radius = max(0.0, corner_radius(sample_p, radii) + spread);
        sum += shadow_x(p.x, sample_p.y, sigma, radius, half_size) * gaussian_weight(y, sigma) * step;
        y += step;
    }
    return clamp(sum, 0.0, 1.0);
}

fn shadow_coverage(p: vec2<f32>, sigma: f32, radii: vec4<f32>, half_size: vec2<f32>, spread: f32) -> f32 {
    if sigma <= 0.01 {
        let d = rounded_rect_sdf(p, half_size, max(radii + vec4<f32>(spread), vec4<f32>(0.0)));
        return 1.0 - smoothstep(-0.5, 0.5, d);
    }
    return blurred_coverage(p, sigma, radii, half_size, spread);
}

@fragment
fn fs_main(in: ShadowVertexOut) -> @location(0) vec4<f32> {
    let center = in.surface_rect.xy + in.surface_rect.zw * 0.5;
    let offset = in.shadow_params.xy;
    let sigma = in.shadow_params.z;
    let spread = in.shadow_params.w;
    let surface_half_size = max(vec2<f32>(0.0), in.surface_rect.zw * 0.5);
    let shadow_kind = in.shadow_meta.x;
    let p_surface = in.pos_px - center;
    let d_surface = rounded_rect_sdf(p_surface, surface_half_size, max(in.corner_radii, vec4<f32>(0.0)));
    let edge_aa = max(fwidth(d_surface) * 0.5, 0.0001);
    var coverage = 0.0;

    if shadow_kind >= 1.5 {
        let p = in.pos_px - center - offset;
        let half_size = max(vec2<f32>(0.0), surface_half_size + vec2<f32>(spread));
        coverage = shadow_coverage(p, sigma, in.corner_radii, half_size, spread);
    } else if shadow_kind >= 0.5 {
        let edge = 1.0 - smoothstep(-edge_aa, edge_aa, d_surface);
        let inner_spread = -spread;
        let p_shadow = p_surface - offset;
        let half_size = max(vec2<f32>(0.0), surface_half_size + vec2<f32>(inner_spread));
        coverage = edge * (1.0 - shadow_coverage(p_shadow, sigma, in.corner_radii, half_size, inner_spread));
    } else {
        let p = in.pos_px - center - offset;
        let half_size = max(vec2<f32>(0.0), surface_half_size + vec2<f32>(spread));
        coverage = shadow_coverage(p, sigma, in.corner_radii, half_size, spread);
    }

    let alpha = in.color.a * coverage;
    return vec4<f32>(in.color.rgb, alpha);
}
",
);
