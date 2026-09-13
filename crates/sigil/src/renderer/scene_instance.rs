#![allow(clippy::redundant_pub_crate)]

use crate::hal::Rect;
use crate::style::Rgba8;
use bytemuck::{Pod, Zeroable};
use half::f16;

use crate::renderer::corner_radii::CornerRadii;
use crate::renderer::scene_command::{
    DrawGradient, DrawPattern, DrawRect, GradientStop, PatternKind,
};
use crate::renderer::shadow_paint::{Shadow, ShadowKind};

mod helpers;

use helpers::{
    RectPaint, inflate, rect_fields, rect_instance_parts, shadow_color, stroke_fields,
    stroke_outset,
};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum BlendSpace {
    Linear,
    Gamma,
}

impl BlendSpace {
    pub(crate) fn from_format(format: wgpu::TextureFormat) -> Self {
        if format.is_srgb() {
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
        let alpha = f32::from(color.a) / 255.0;
        [
            f16::from_f32(self.channel(color.r) * alpha),
            f16::from_f32(self.channel(color.g) * alpha),
            f16::from_f32(self.channel(color.b) * alpha),
            f16::from_f32(alpha),
        ]
    }
}

pub(crate) const FILL_KIND_GRADIENT: u32 = 0;
pub(crate) const FILL_KIND_SLASH: u32 = 1;
pub(crate) const FILL_KIND_CHECKER: u32 = 2;

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
    let (stroke_params, stroke_meta) = stroke_fields(draw.stroke, space);
    let outset = stroke_outset(draw.stroke);
    let inflated = inflate(draw.rect, outset);
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
    }
}

pub(crate) fn draw_pattern_instance(
    draw: &DrawPattern,
    opacity_scale: f32,
    space: BlendSpace,
) -> FillInstance {
    let (stroke_params, stroke_meta) = stroke_fields(draw.stroke, space);
    let outset = stroke_outset(draw.stroke);
    let inflated = inflate(draw.rect, outset);
    let Rect {
        origin,
        width,
        height,
    } = inflated;
    let opacity = ((f32::from(draw.opacity) / 10_000.0) * opacity_scale).clamp(0.0, 1.0);
    let (fill_kind, pattern_a, pattern_b) = pack_pattern(draw.pattern, space);
    FillInstance {
        rect: [origin.x, origin.y, width, height],
        line: [0.0; 4],
        corner_radii: draw
            .border_radius
            .normalized_for(draw.rect.width, draw.rect.height),
        params: [opacity, outset, 0.0, 0.0],
        stop_range: [0, 0, fill_kind, 0],
        stroke_params,
        stroke_meta,
        pattern_a,
        pattern_b,
    }
}

fn pack_pattern(kind: PatternKind, space: BlendSpace) -> (u32, [f32; 4], [f32; 4]) {
    match kind {
        PatternKind::Slash {
            color,
            period,
            line_width,
        } => {
            let [r, g, b, a] = space.rgba(color);
            (
                FILL_KIND_SLASH,
                [r, g, b, a],
                [period, line_width, 0.0, 0.0],
            )
        }
        PatternKind::Checker {
            color_a,
            color_b,
            cell,
        } => {
            let [ra, ga, ba, aa] = space.rgba(color_a);
            let [rb, gb, bb, _ab] = space.rgba(color_b);
            (FILL_KIND_CHECKER, [ra, ga, ba, aa], [rb, gb, bb, cell])
        }
    }
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

pub(crate) fn expanded_shadow_rect(surface: Rect, shadow: Shadow) -> Rect {
    if shadow.kind == ShadowKind::Inner {
        return surface;
    }
    let [offset_x, offset_y] = shadow.offset;
    let outset = shadow.blur.mul_add(3.0, shadow.spread.abs());
    Rect::new(
        crate::hal::Point::new(
            surface.origin.x - outset + offset_x,
            surface.origin.y - outset + offset_y,
        ),
        surface.width + outset * 2.0,
        surface.height + outset * 2.0,
    )
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

pub(crate) fn srgb_channel_to_linear(channel: u8) -> f32 {
    let value = f32::from(channel) / 255.0;
    if value <= 0.040_45 {
        value / 12.92
    } else {
        ((value + 0.055) / 1.055).powf(2.4)
    }
}

#[cfg(test)]
#[path = "scene_instance_tests.rs"]
mod tests;
