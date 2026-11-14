#![allow(clippy::redundant_pub_crate)]

use crate::renderer::frame_upload::UploadedRange;
use crate::renderer::scene_paint::PaintCtx;

pub(crate) mod setup;

use setup::{QuadDesc, QuadPipeline, buffer_group, buffer_layout, clipped_pass};

macro_rules! sdf_helpers {
    () => {
        "fn aa_pad(row_x: vec2<f32>, row_y: vec2<f32>) -> vec2<f32> {
    let column_x = length(vec2<f32>(row_x.x, row_y.x));
    let column_y = length(vec2<f32>(row_x.y, row_y.y));
    return vec2<f32>(1.0 / max(column_x, 1e-6), 1.0 / max(column_y, 1e-6));
}

fn corner_radius(p: vec2<f32>, radii: vec4<f32>) -> f32 {
    let bottom = select(radii.w, radii.z, p.x > 0.0);
    let top = select(radii.x, radii.y, p.x > 0.0);
    return select(bottom, top, p.y < 0.0);
}

fn rounded_rect_sdf(p: vec2<f32>, half: vec2<f32>, radii: vec4<f32>) -> f32 {
    let r = corner_radius(p, radii);
    let q = abs(p) - half + vec2<f32>(r, r);
    return length(max(q, vec2<f32>(0.0, 0.0))) + min(max(q.x, q.y), 0.0) - r;
}

fn corner_distance(a: vec2<f32>, code: f32) -> f32 {
    var e = code;
    var b = a;
    var side = 1.0;
    if code < 0.0 {
        if max(a.x, a.y) >= 1.0 {
            return min(length(a - vec2<f32>(1.0, 0.0)), length(a - vec2<f32>(0.0, 1.0)));
        }
        e = -code;
        b = vec2<f32>(1.0) - a;
        side = -1.0;
    }
    let norm = pow(pow(b.x, e) + pow(b.y, e), 1.0 / e);
    let pull = pow(max(b / max(norm, 1e-6), vec2<f32>(1e-6)), vec2<f32>(e - 1.0));
    return side * (norm - 1.0) / max(length(pull), 1e-6);
}

fn shaped_rect_sdf(p: vec2<f32>, half: vec2<f32>, radii: vec4<f32>, code: f32) -> f32 {
    if code == 0.0 {
        return rounded_rect_sdf(p, half, radii);
    }
    let r = corner_radius(p, radii);
    let q = abs(p) - half + vec2<f32>(r, r);
    if r <= 0.0 || min(q.x, q.y) <= 0.0 {
        return length(max(q, vec2<f32>(0.0, 0.0))) + min(max(q.x, q.y), 0.0) - r;
    }
    return r * corner_distance(q / r, code);
}

fn corner_reach(t_in: f32, code: f32) -> f32 {
    let t = clamp(t_in, 0.0, 1.0);
    if code < 0.0 {
        let m = -code;
        return 1.0 - pow(max(1.0 - pow(1.0 - t, m), 0.0), 1.0 / m);
    }
    return pow(max(1.0 - pow(t, code), 0.0), 1.0 / code);
}

fn edge_coverage(d: f32, aa: f32) -> f32 {
    return clamp(0.5 - d / (2.0 * aa), 0.0, 1.0);
}

fn dash_mask(local: vec2<f32>, dash_len: f32, dash_gap: f32) -> f32 {
    if dash_len <= 0.0 {
        return 1.0;
    }
    let period = dash_len + dash_gap;
    let arc = abs(local.x) + abs(local.y);
    return select(0.0, 1.0, arc - floor(arc / period) * period < dash_len);
}

fn stroke_mask(d: f32, local: vec2<f32>, band: vec4<f32>, aa: f32) -> f32 {
    if band.x + band.y <= 0.0 {
        return 0.0;
    }
    let stroke_d = max(-band.x - d, d - band.y);
    let alpha = edge_coverage(stroke_d, aa);
    return alpha * dash_mask(local, band.z, band.w);
}

"
    };
}

pub(in crate::renderer) use sdf_helpers;

macro_rules! srgb_wgsl {
    () => {
        "
fn srgb_encode(c: vec3<f32>) -> vec3<f32> {
    let v = clamp(c, vec3<f32>(0.0), vec3<f32>(1.0));
    let high = 1.055 * pow(v, vec3<f32>(1.0 / 2.4)) - vec3<f32>(0.055);
    return select(high, v * 12.92, v <= vec3<f32>(0.0031308));
}

fn srgb_decode(c: vec3<f32>) -> vec3<f32> {
    let v = clamp(c, vec3<f32>(0.0), vec3<f32>(1.0));
    let high = pow((v + vec3<f32>(0.055)) / 1.055, vec3<f32>(2.4));
    return select(high, v / 12.92, v <= vec3<f32>(0.04045));
}

fn linear_to_oklab(c: vec3<f32>) -> vec3<f32> {
    let lms = mat3x3<f32>(
        vec3<f32>(0.41222146, 0.21190350, 0.08830246),
        vec3<f32>(0.53633255, 0.68069950, 0.28171885),
        vec3<f32>(0.05144599, 0.10739696, 0.62997870),
    ) * c;
    return mat3x3<f32>(
        vec3<f32>(0.21045426, 1.97799850, 0.02590404),
        vec3<f32>(0.79361780, -2.42859220, 0.78277177),
        vec3<f32>(-0.00407205, 0.45059370, -0.80867577),
    ) * (sign(lms) * pow(abs(lms), vec3<f32>(1.0 / 3.0)));
}

fn oklab_to_linear(lab: vec3<f32>) -> vec3<f32> {
    let lms = mat3x3<f32>(
        vec3<f32>(1.0, 1.0, 1.0),
        vec3<f32>(0.39633778, -0.10556135, -0.08948418),
        vec3<f32>(0.21580376, -0.06385417, -1.29148550),
    ) * lab;
    return mat3x3<f32>(
        vec3<f32>(4.07674170, -1.26843800, -0.00419609),
        vec3<f32>(-3.30771160, 2.60975740, -0.70341860),
        vec3<f32>(0.23096994, -0.34131938, 1.70761470),
    ) * (lms * lms * lms);
}

"
    };
}

pub(in crate::renderer) use srgb_wgsl;

const INSTANCE_LAYOUT: wgpu::VertexBufferLayout<'_> = wgpu::VertexBufferLayout {
    array_stride: core::mem::size_of::<crate::renderer::scene_instance::FillInstance>() as u64,
    step_mode: wgpu::VertexStepMode::Instance,
    attributes: &wgpu::vertex_attr_array![
        1 => Float32x4,
        2 => Float32x4,
        3 => Float32x4,
        4 => Float32x4,
        5 => Uint32x4,
        6 => Float32x4,
        7 => Float32x4,
        8 => Float32x4,
        9 => Float32x4,
        10 => Float32x4,
        11 => Float32x4,
    ],
};

pub(crate) struct FillPipeline {
    quad: QuadPipeline,
    uniform_layout: wgpu::BindGroupLayout,
    stops_layout: wgpu::BindGroupLayout,
}

impl FillPipeline {
    pub(crate) fn new(device: &wgpu::Device, format: wgpu::TextureFormat) -> Self {
        let uniform_layout = buffer_layout(
            device,
            "sigil-renderer fill paint uniform layout",
            wgpu::ShaderStages::VERTEX,
            wgpu::BufferBindingType::Uniform,
        );
        let stops_layout = buffer_layout(
            device,
            "sigil-renderer fill paint stops layout",
            wgpu::ShaderStages::FRAGMENT,
            wgpu::BufferBindingType::Storage { read_only: true },
        );
        let quad = QuadPipeline::new(
            device,
            format,
            QuadDesc {
                label: "sigil-renderer fill paint",
                wgsl: concat!(
                    sdf_helpers!(),
                    srgb_wgsl!(),
                    include_str!("fill_paint.wgsl")
                ),
                groups: &[Some(&uniform_layout), Some(&stops_layout)],
                instance: INSTANCE_LAYOUT,
                fs_entry: "fs_main",
                blend: wgpu::BlendState::PREMULTIPLIED_ALPHA_BLENDING,
            },
        );
        Self {
            quad,
            uniform_layout,
            stops_layout,
        }
    }

    pub(crate) const fn format(&self) -> wgpu::TextureFormat {
        self.quad.format
    }

    pub(crate) fn paint_uploaded(
        &self,
        mut ctx: PaintCtx<'_>,
        instances: UploadedRange,
        stops: &UploadedRange,
        uniform: wgpu::BufferBinding<'_>,
    ) {
        let uniform_bg = buffer_group(
            ctx.device,
            "sigil-renderer fill paint uniform bind group",
            &self.uniform_layout,
            uniform,
        );
        let stops_bg = buffer_group(
            ctx.device,
            "sigil-renderer fill paint stops bind group",
            &self.stops_layout,
            wgpu::BufferBinding {
                buffer: &stops.buffer,
                offset: stops.bytes.start,
                size: wgpu::BufferSize::new(stops.bytes.end - stops.bytes.start),
            },
        );
        let Some(pass) = clipped_pass(&mut ctx, "sigil-renderer fill paint pass") else {
            return;
        };
        self.quad.draw(
            pass,
            &[&uniform_bg, &stops_bg],
            &[],
            instances.buffer.slice(instances.bytes),
            instances.count,
        );
    }
}

#[cfg(test)]
#[path = "fill_paint_tests.rs"]
mod tests;
