#![allow(clippy::redundant_pub_crate)]

use crate::geometry::Rect;
use crate::style::Rgba8;
use bytemuck::{Pod, Zeroable};
use half::f16;

use crate::renderer::scene_command::walk::IDENTITY;
use crate::renderer::scene_command::{
    DrawGradient, DrawPattern, DrawRect, GradientStop, PatternKind, Transform,
};
use crate::renderer::shadow_paint::Shadow;
use crate::scene::corner_radii::CornerRadii;
pub(crate) use crate::scene::shadow::{expanded_shadow_rect, srgb_channel_to_linear};

mod helpers;

use helpers::{
    RectPaint, inflate, rect_fields, rect_instance_parts, shadow_color, stroke_fields,
    stroke_outset,
};

pub(crate) const LAYER_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba16Float;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum BlendSpace {
    Linear,
    Gamma,
}

impl BlendSpace {
    pub(crate) fn from_format(format: wgpu::TextureFormat) -> Self {
        if format.is_srgb() || format == LAYER_FORMAT {
            Self::Linear
        } else {
            Self::Gamma
        }
    }

    fn channel(self, channel: u8) -> f32 {
        match self {
            Self::Linear => srgb_channel_to_linear(channel),
            Self::Gamma => f32::from(channel) / 255.0,
        }
    }

    pub(crate) fn rgba(self, color: Rgba8) -> [f32; 4] {
        [
            self.channel(color.r),
            self.channel(color.g),
            self.channel(color.b),
            f32::from(color.a) / 255.0,
        ]
    }

    pub(crate) fn rgba16f(self, color: Rgba8) -> [f16; 4] {
        let [r, g, b, a] = self.rgba(color);
        [
            f16::from_f32(r),
            f16::from_f32(g),
            f16::from_f32(b),
            f16::from_f32(a),
        ]
    }
}

pub(crate) const FILL_KIND_GRADIENT: u32 = 0;
pub(crate) const FILL_KIND_SLASH: u32 = 1;
pub(crate) const FILL_KIND_CHECKER: u32 = 2;

pub(crate) const fn affine_rows(transform: Transform) -> [[f32; 4]; 2] {
    let [[a, b, tx], [c, d, ty]] = transform;
    [[a, b, tx, 0.0], [c, d, ty, 0.0]]
}

pub(crate) const MAX_GRADIENT_STOPS: usize = 8;
pub(crate) const MAX_GRADIENT_STOPS_U32: u32 = 8;

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub(crate) struct RectInstance {
    pub(crate) rect: [f32; 4],
    pub(crate) color: [f32; 4],
    pub(crate) corner_radii: [f32; 4],
    pub(crate) params: [f32; 4],
    pub(crate) stroke_params: [f32; 4],
    pub(crate) stroke_meta: [f32; 4],
    pub(crate) xform_x: [f32; 4],
    pub(crate) xform_y: [f32; 4],
}

impl RectInstance {
    pub(crate) const fn with_dash_origin(mut self, origin: [f32; 2]) -> Self {
        let [opacity, outset, _, _] = self.params;
        let [x, y] = origin;
        self.params = [opacity, outset, x, y];
        self
    }

    pub(crate) const fn transformed(mut self, transform: Transform) -> Self {
        let [xform_x, xform_y] = affine_rows(transform);
        self.xform_x = xform_x;
        self.xform_y = xform_y;
        self
    }
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub(crate) struct FillInstance {
    pub(crate) rect: [f32; 4],
    pub(crate) line: [f32; 4],
    pub(crate) corner_radii: [f32; 4],
    pub(crate) params: [f32; 4],
    pub(crate) stop_range: [u32; 4],
    pub(crate) stroke_params: [f32; 4],
    pub(crate) stroke_meta: [f32; 4],
    pub(crate) pattern_a: [f32; 4],
    pub(crate) pattern_b: [f32; 4],
    pub(crate) xform_x: [f32; 4],
    pub(crate) xform_y: [f32; 4],
}

impl FillInstance {
    pub(crate) const fn with_dash_origin(mut self, origin: [f32; 2]) -> Self {
        let [opacity, outset, _, _] = self.params;
        let [x, y] = origin;
        self.params = [opacity, outset, x, y];
        self
    }

    pub(crate) const fn with_phase(mut self, phase: [f32; 2]) -> Self {
        let [a, b, _, _] = self.line;
        let [x, y] = phase;
        self.line = [a, b, x, y];
        self
    }

    pub(crate) const fn transformed(mut self, transform: Transform) -> Self {
        let [xform_x, xform_y] = affine_rows(transform);
        self.xform_x = xform_x;
        self.xform_y = xform_y;
        self
    }
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub(crate) struct ShadowInstance {
    pub(crate) rect: [f32; 4],
    pub(crate) surface_rect: [f32; 4],
    pub(crate) corner_radii: [f32; 4],
    pub(crate) color: [f32; 4],
    pub(crate) shadow_params: [f32; 4],
    pub(crate) shadow_meta: [f32; 4],
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub(crate) struct GpuStop {
    pub(crate) offset_bits: u32,
    pub(crate) rgba01: u32,
    pub(crate) rgba23: u32,
    pub(crate) _pad: u32,
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub(crate) struct SceneUniforms {
    pub(crate) viewport: [f32; 2],
    pub(crate) _pad: [f32; 2],
}

pub(crate) fn draw_rect_instance(
    draw: &DrawRect,
    opacity_scale: f32,
    space: BlendSpace,
) -> RectInstance {
    rect_instance_parts(
        draw.rect,
        draw.border_radius,
        RectPaint {
            color: space.rgba(draw.color),
            opacity: (f32::from(draw.opacity) / 10_000.0) * opacity_scale,
            stroke: draw.stroke,
            space,
        },
    )
}

pub(crate) fn draw_gradient_instance(
    draw: &DrawGradient,
    opacity_scale: f32,
    stop_offset: u32,
    stop_count: u32,
    space: BlendSpace,
) -> FillInstance {
    let (stroke_params, stroke_meta) = stroke_fields(draw.stroke, draw.rect, space);
    let outset = stroke_outset(draw.stroke, draw.rect);
    let inflated = inflate(draw.rect, outset);
    let [xform_x, xform_y] = affine_rows(IDENTITY);
    let Rect {
        origin,
        width,
        height,
    } = inflated;
    FillInstance {
        rect: [origin.x, origin.y, width, height],
        line: [
            draw.gradient.start.x,
            draw.gradient.start.y,
            draw.gradient.end.x,
            draw.gradient.end.y,
        ],
        corner_radii: draw
            .border_radius
            .normalized_for(draw.rect.width, draw.rect.height),
        params: [
            ((f32::from(draw.opacity) / 10_000.0) * opacity_scale).clamp(0.0, 1.0),
            outset,
            0.0,
            0.0,
        ],
        stop_range: [
            stop_offset,
            stop_count.min(MAX_GRADIENT_STOPS_U32),
            FILL_KIND_GRADIENT,
            0,
        ],
        stroke_params,
        stroke_meta,
        pattern_a: [0.0; 4],
        pattern_b: [0.0; 4],
        xform_x,
        xform_y,
    }
}

pub(crate) fn draw_pattern_instance(
    draw: &DrawPattern,
    opacity_scale: f32,
    space: BlendSpace,
) -> FillInstance {
    let (stroke_params, stroke_meta) = stroke_fields(draw.stroke, draw.rect, space);
    let outset = stroke_outset(draw.stroke, draw.rect);
    let inflated = inflate(draw.rect, outset);
    let [xform_x, xform_y] = affine_rows(IDENTITY);
    let Rect {
        origin,
        width,
        height,
    } = inflated;
    let opacity = ((f32::from(draw.opacity) / 10_000.0) * opacity_scale).clamp(0.0, 1.0);
    let (fill_kind, line, pattern_a, pattern_b) = pack_pattern(draw.pattern, space);
    FillInstance {
        rect: [origin.x, origin.y, width, height],
        line,
        corner_radii: draw
            .border_radius
            .normalized_for(draw.rect.width, draw.rect.height),
        params: [opacity, outset, 0.0, 0.0],
        stop_range: [0, 0, fill_kind, 0],
        stroke_params,
        stroke_meta,
        pattern_a,
        pattern_b,
        xform_x,
        xform_y,
    }
}

type PatternPack = (u32, [f32; 4], [f32; 4], [f32; 4]);

fn pack_pattern(kind: PatternKind, space: BlendSpace) -> PatternPack {
    match kind.normalized() {
        PatternKind::Slash {
            color,
            period,
            line_width,
        } => (
            FILL_KIND_SLASH,
            [period, line_width, 0.0, 0.0],
            premultiplied(space, color),
            [0.0; 4],
        ),
        PatternKind::Checker {
            color_a,
            color_b,
            cell,
        } => (
            FILL_KIND_CHECKER,
            [cell, 0.0, 0.0, 0.0],
            premultiplied(space, color_a),
            premultiplied(space, color_b),
        ),
    }
}

fn premultiplied(space: BlendSpace, color: Rgba8) -> [f32; 4] {
    let [r, g, b, a] = space.rgba(color);
    [r * a, g * a, b * a, a]
}

pub(crate) fn draw_shadow_instance(
    surface: Rect,
    radii: CornerRadii,
    shadow: Shadow,
    opacity_scale: f32,
) -> ShadowInstance {
    let expanded = expanded_shadow_rect(surface, shadow);
    let [offset_x, offset_y] = shadow.offset;
    ShadowInstance {
        rect: rect_fields(expanded),
        surface_rect: rect_fields(surface),
        corner_radii: radii.normalized_for(surface.width, surface.height),
        color: shadow_color(shadow.color, opacity_scale),
        shadow_params: [offset_x, offset_y, shadow.blur.max(0.0), shadow.spread],
        shadow_meta: [shadow.kind.as_f32(), 0.0, 0.0, 0.0],
    }
}

pub(crate) fn gradient_stop_records(stops: &[GradientStop], space: BlendSpace) -> Vec<GpuStop> {
    let mut records = Vec::with_capacity(stops.len().clamp(1, MAX_GRADIENT_STOPS));
    for stop in stops.iter().take(MAX_GRADIENT_STOPS) {
        records.push(gpu_stop(*stop, space));
    }
    if records.is_empty() {
        records.push(gpu_stop(
            GradientStop::new(
                0.0,
                Rgba8 {
                    r: 0,
                    g: 0,
                    b: 0,
                    a: 0,
                },
            ),
            space,
        ));
    }
    records
}

fn gpu_stop(stop: GradientStop, space: BlendSpace) -> GpuStop {
    let [r, g, b, a] = space.rgba16f(stop.color);
    GpuStop {
        offset_bits: stop.offset.clamp(0.0, 1.0).to_bits(),
        rgba01: pack_f16_pair(r, g),
        rgba23: pack_f16_pair(b, a),
        _pad: 0,
    }
}

fn pack_f16_pair(first: f16, second: f16) -> u32 {
    u32::from(first.to_bits()) | (u32::from(second.to_bits()) << 16)
}

#[cfg(test)]
#[path = "scene_instance_tests.rs"]
mod tests;
