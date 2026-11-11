#![allow(clippy::redundant_pub_crate)]

use crate::geometry::Rect;
use crate::style::Color;
use crate::style::srgb::unit;
use bytemuck::{Pod, Zeroable};
use half::f16;

use crate::renderer::scene_command::walk::IDENTITY;
use crate::renderer::scene_command::{
    DrawGradient, DrawPattern, DrawRect, GradientStop, PatternKind, Stroke, StrokeBand, Transform,
};
use crate::renderer::shadow_paint::Shadow;
use crate::scene::command::{Gradient, GradientKind};
use crate::scene::corner_radii::CornerRadii;
pub(crate) use crate::scene::shadow::{expanded_shadow_rect, srgb_channel_to_linear};

mod helpers;

use helpers::{inflate, rect_fields, shadow_color, stroke_fields};

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
            Self::Gamma => unit(channel),
        }
    }

    fn flag(self) -> u32 {
        u32::from(self == Self::Linear)
    }

    pub(crate) fn rgba(self, color: Color) -> [f32; 4] {
        [
            self.channel(color.r),
            self.channel(color.g),
            self.channel(color.b),
            unit(color.a),
        ]
    }
}

pub(crate) const FILL_KIND_GRADIENT: u32 = 0;
#[cfg(feature = "kittest")]
pub(crate) const FILL_KIND_SLASH: u32 = 1;
#[cfg(feature = "kittest")]
pub(crate) const FILL_KIND_CHECKER: u32 = 2;
pub(crate) const FILL_KIND_WASH: u32 = 3;
pub(crate) const FILL_KIND_RADIAL: u32 = 4;
pub(crate) const FILL_KIND_CONIC: u32 = 5;

pub(crate) const fn affine_rows(transform: Transform) -> [[f32; 4]; 2] {
    let [[a, b, tx], [c, d, ty]] = transform;
    [[a, b, tx, 0.0], [c, d, ty, 0.0]]
}

const fn with_tail(head: [f32; 4], tail: [f32; 2]) -> [f32; 4] {
    let [a, b, _, _] = head;
    let [x, y] = tail;
    [a, b, x, y]
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
    pub(crate) const fn with_phase(mut self, phase: [f32; 2]) -> Self {
        self.line = with_tail(self.line, phase);
        self
    }
}

macro_rules! dash_and_transform {
    ($($instance:ty),+) => {$(
        impl $instance {
            pub(crate) const fn with_dash_origin(mut self, origin: [f32; 2]) -> Self {
                self.params = with_tail(self.params, origin);
                self
            }

            pub(crate) const fn transformed(mut self, transform: Transform) -> Self {
                [self.xform_x, self.xform_y] = affine_rows(transform);
                self
            }
        }
    )+};
}

dash_and_transform!(RectInstance, FillInstance);

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
    let rect = draw.rect;
    let (stroke_params, stroke_meta) = stroke_fields(draw.stroke, rect, space);
    let outset = StrokeBand::reach(draw.stroke, rect);
    let [xform_x, xform_y] = affine_rows(IDENTITY);
    let opacity = (f32::from(draw.opacity) / 10_000.0) * opacity_scale;
    RectInstance {
        rect: rect_fields(inflate(rect, outset)),
        color: space.rgba(draw.color),
        corner_radii: draw.border_radius.normalized_for(rect.width, rect.height),
        params: [opacity.clamp(0.0, 1.0), outset, 0.0, 0.0],
        stroke_params,
        stroke_meta,
        xform_x,
        xform_y,
    }
}

pub(crate) fn draw_gradient_instance(
    draw: &DrawGradient,
    opacity_scale: f32,
    stop_offset: u32,
    stop_count: u32,
    space: BlendSpace,
) -> FillInstance {
    let opacity = (f32::from(draw.opacity) / 10_000.0) * opacity_scale;
    let base = fill_base(draw.rect, draw.border_radius, draw.stroke, opacity, space);
    let (fill_kind, line, pattern_a) = pack_gradient(&draw.gradient);
    let span = draw.gradient.repeat_span();
    let [first, last] = span.unwrap_or([0.0, 1.0]);
    FillInstance {
        line,
        stop_range: [
            stop_offset,
            stop_count.min(MAX_GRADIENT_STOPS_U32),
            fill_kind,
            space.flag(),
        ],
        pattern_a,
        pattern_b: [first, last, f32::from(u8::from(span.is_some())), 0.0],
        ..base
    }
}

const fn pack_gradient(gradient: &Gradient) -> (u32, [f32; 4], [f32; 4]) {
    match gradient.kind {
        GradientKind::Linear { start, end } => (
            FILL_KIND_GRADIENT,
            [start.x, start.y, end.x, end.y],
            [0.0; 4],
        ),
        GradientKind::Radial { center, radius } => (
            FILL_KIND_RADIAL,
            [center.x, center.y, center.x, center.y],
            [radius[0], radius[1], 0.0, 0.0],
        ),
        GradientKind::Conic { center, angle } => (
            FILL_KIND_CONIC,
            [center.x, center.y, center.x, center.y],
            [0.0, 0.0, angle, 0.0],
        ),
    }
}

pub(crate) fn draw_pattern_instance(
    draw: &DrawPattern,
    opacity_scale: f32,
    space: BlendSpace,
    [stop_offset, stop_count]: [u32; 2],
) -> FillInstance {
    let opacity = (f32::from(draw.opacity) / 10_000.0) * opacity_scale;
    let base = fill_base(draw.rect, draw.border_radius, draw.stroke, opacity, space);
    let (fill_kind, line, pattern_a, mut pattern_b) = match draw.pattern.normalized() {
        #[cfg(feature = "kittest")]
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
        #[cfg(feature = "kittest")]
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
        PatternKind::Wash(wash) => pack_wash(wash),
    };
    if fill_kind == FILL_KIND_WASH {
        pattern_b[3] = crate::scene::command::wash_centre_bits(draw.rect.width, draw.rect.height);
    }
    FillInstance {
        line,
        stop_range: [stop_offset, stop_count, fill_kind, space.flag()],
        pattern_a,
        pattern_b,
        ..base
    }
}

fn fill_base(
    rect: Rect,
    radii: CornerRadii,
    stroke: Option<Stroke>,
    opacity: f32,
    space: BlendSpace,
) -> FillInstance {
    let (stroke_params, stroke_meta) = stroke_fields(stroke, rect, space);
    let outset = StrokeBand::reach(stroke, rect);
    let [xform_x, xform_y] = affine_rows(IDENTITY);
    FillInstance {
        rect: rect_fields(inflate(rect, outset)),
        line: [0.0; 4],
        corner_radii: radii.normalized_for(rect.width, rect.height),
        params: [opacity.clamp(0.0, 1.0), outset, 0.0, 0.0],
        stop_range: [0, 0, 0, space.flag()],
        stroke_params,
        stroke_meta,
        pattern_a: [0.0; 4],
        pattern_b: [0.0; 4],
        xform_x,
        xform_y,
    }
}

type PatternPack = (u32, [f32; 4], [f32; 4], [f32; 4]);

#[allow(
    clippy::cast_precision_loss,
    reason = "a wash seed is masked to 24 bits, which f32 holds exactly"
)]
fn pack_wash(wash: crate::scene::Wash) -> PatternPack {
    (
        FILL_KIND_WASH,
        [wash.scale, wash.seed as f32, 0.0, 0.0],
        [wash.density, wash.pools, wash.edges, wash.granulation],
        [
            wash.grain,
            wash.specks,
            f32::from(u8::from(wash.band) | (u8::from(wash.dark()) << 1)),
            0.0,
        ],
    )
}

/// The paper and pigment stops a wash reads, with their positions in the palette.
pub(crate) fn wash_stops(wash: crate::scene::Wash) -> Vec<GradientStop> {
    wash.palette()
        .zip(0_u16..)
        .map(|(color, index)| GradientStop::new(f32::from(index), color))
        .collect()
}

#[cfg(feature = "kittest")]
fn premultiplied(space: BlendSpace, color: Color) -> [f32; 4] {
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
    let transparent = [GradientStop::new(
        0.0,
        Color {
            r: 0,
            g: 0,
            b: 0,
            a: 0,
        },
    )];
    let stops = if stops.is_empty() {
        &transparent[..]
    } else {
        stops
    };
    stops
        .iter()
        .take(MAX_GRADIENT_STOPS)
        .map(|stop| gpu_stop(*stop, space))
        .collect()
}

fn gpu_stop(stop: GradientStop, space: BlendSpace) -> GpuStop {
    let [r, g, b, a] = space.rgba(stop.color).map(f16::from_f32);
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
