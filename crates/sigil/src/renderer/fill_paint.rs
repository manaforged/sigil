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
                wgsl: concat!(sdf_helpers!(), include_str!("fill_paint.wgsl")),
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
