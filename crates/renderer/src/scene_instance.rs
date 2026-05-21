#![allow(clippy::redundant_pub_crate)]
//! Per-instance data + collection helpers for [`crate::scene_paint`].
//!
//! Split out of `scene_paint.rs` to respect the 400-line hygiene cap
//! and to colocate instance-data tests with the helpers they cover.

use bytemuck::{Pod, Zeroable};
use hal::Rect;
use half::f16;
use style::Rgba8;

use crate::corner_radii::CornerRadii;
use crate::scene::{Scene, SceneItem, SceneRect};
use crate::scene_command::{DrawGradient, DrawPattern, DrawRect, GradientStop, PatternKind};
use crate::shadow_paint::{Shadow, ShadowKind};

mod helpers;

use helpers::{
    inflate, rect_fields, rect_instance_parts, shadow_color, stroke_fields, stroke_outset,
};

/// `fill_kind` discriminator stored in `FillInstance::stop_range.z`.
pub(crate) const FILL_KIND_GRADIENT: u32 = 0;
pub(crate) const FILL_KIND_SLASH: u32 = 1;
pub(crate) const FILL_KIND_CHECKER: u32 = 2;

pub(crate) const MAX_GRADIENT_STOPS: usize = 8;
pub(crate) const MAX_GRADIENT_STOPS_U32: u32 = 8;

/// Per-instance GPU record for one rectangle.
#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub(crate) struct RectInstance {
    /// `[x, y, w, h]` in logical pixels.
    pub(crate) rect: [f32; 4],
    /// Linear RGBA in `[0, 1]`.
    pub(crate) color: [f32; 4],
    /// `[top_left, top_right, bottom_right, bottom_left]` radii.
    pub(crate) corner_radii: [f32; 4],
    /// `[opacity_01, outset, 0, 0]`.
    pub(crate) params: [f32; 4],
    /// `[r, g, b, a]` stroke colour.
    pub(crate) stroke_params: [f32; 4],
    /// `[width, style_f32, align_f32, dash_gap]`.
    pub(crate) stroke_meta: [f32; 4],
}

/// Per-instance GPU record for gradient + pattern filled rectangles.
///
/// `stop_range.z` carries the `fill_kind` discriminator:
/// `0 = gradient`, `1 = slash`, `2 = checker`.
/// `pattern_a` and `pattern_b` are zero for gradient fills.
#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub(crate) struct FillInstance {
    /// `[x, y, w, h]` in logical pixels (inflated by stroke outset).
    pub(crate) rect: [f32; 4],
    /// `[start_x, start_y, end_x, end_y]` gradient axis in logical pixels.
    pub(crate) line: [f32; 4],
    /// `[top_left, top_right, bottom_right, bottom_left]` radii.
    pub(crate) corner_radii: [f32; 4],
    /// `[opacity_01, outset, 0, 0]`.
    pub(crate) params: [f32; 4],
    /// `[stop_offset, stop_count, fill_kind, 0]`.
    pub(crate) stop_range: [u32; 4],
    /// `[r, g, b, a]` stroke colour.
    pub(crate) stroke_params: [f32; 4],
    /// `[width, style_f32, align_f32, dash_gap]`.
    pub(crate) stroke_meta: [f32; 4],
    /// Pattern parameter slot A (meaning is fill_kind-dependent).
    pub(crate) pattern_a: [f32; 4],
    /// Pattern parameter slot B (meaning is fill_kind-dependent).
    pub(crate) pattern_b: [f32; 4],
}

/// Per-instance GPU record for one drop shadow.
#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub(crate) struct ShadowInstance {
    /// Expanded draw quad `[x, y, w, h]` in logical pixels.
    pub(crate) rect: [f32; 4],
    /// Original casting surface `[x, y, w, h]` in logical pixels.
    pub(crate) surface_rect: [f32; 4],
    /// `[top_left, top_right, bottom_right, bottom_left]` radii.
    pub(crate) corner_radii: [f32; 4],
    /// Straight RGBA in `[0, 1]`, with alpha scaled by layer opacity.
    pub(crate) color: [f32; 4],
    /// `[offset_x, offset_y, blur_sigma, spread]`.
    pub(crate) shadow_params: [f32; 4],
    /// `[kind_f32, 0, 0, 0]` where `0 = outer` and `1 = inner`.
    pub(crate) shadow_meta: [f32; 4],
}

/// Packed gradient stop storage. Colour channels are IEEE-754 f16 bits.
#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub(crate) struct GpuStop {
    pub(crate) offset_bits: u32,
    pub(crate) rgba01: u32,
    pub(crate) rgba23: u32,
    pub(crate) _pad: u32,
}

/// Per-frame uniform carrying the viewport size in logical pixels.
#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub(crate) struct SceneUniforms {
    pub(crate) viewport: [f32; 2],
    pub(crate) _pad: [f32; 2],
}

#[allow(dead_code)]
pub(crate) fn collect_instances(scene: &Scene) -> Vec<RectInstance> {
    let mut out = Vec::with_capacity(scene.items().len());
    for item in scene.items() {
        if let SceneItem::Rect(rect) = item {
            out.push(rect_instance(rect));
        }
    }
    out
}

#[allow(dead_code)]
pub(crate) fn rect_instance(rect: &SceneRect) -> RectInstance {
    rect_instance_parts(
        rect.rect,
        rect.color,
        rect.border_radius,
        f32::from(rect.opacity) / 10_000.0,
        None,
    )
}

pub(crate) fn draw_rect_instance(draw: &DrawRect, opacity_scale: f32) -> RectInstance {
    rect_instance_parts(
        draw.rect,
        draw.color,
        draw.border_radius,
        (f32::from(draw.opacity) / 10_000.0) * opacity_scale,
        draw.stroke,
    )
}

pub(crate) fn draw_gradient_instance(
    draw: &DrawGradient,
    opacity_scale: f32,
    stop_offset: u32,
    stop_count: u32,
) -> FillInstance {
    let (stroke_params, stroke_meta) = stroke_fields(draw.stroke);
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
        corner_radii: draw.border_radius.nonnegative_array(),
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

/// Build a `FillInstance` for a pattern fill.
pub(crate) fn draw_pattern_instance(draw: &DrawPattern, opacity_scale: f32) -> FillInstance {
    let (stroke_params, stroke_meta) = stroke_fields(draw.stroke);
    let outset = stroke_outset(draw.stroke);
    let inflated = inflate(draw.rect, outset);
    let Rect {
        origin,
        width,
        height,
    } = inflated;
    let opacity = ((f32::from(draw.opacity) / 10_000.0) * opacity_scale).clamp(0.0, 1.0);
    let (fill_kind, pattern_a, pattern_b) = pack_pattern(draw.pattern);
    FillInstance {
        rect: [origin.x, origin.y, width, height],
        line: [0.0; 4],
        corner_radii: draw.border_radius.nonnegative_array(),
        params: [opacity, outset, 0.0, 0.0],
        stop_range: [0, 0, fill_kind, 0],
        stroke_params,
        stroke_meta,
        pattern_a,
        pattern_b,
    }
}

/// Encode pattern parameters into the two `[f32; 4]` GPU slots.
///
/// Slash:   `pattern_a = [r, g, b, a]` (linear 0..1),
///          `pattern_b = [period, line_width, 0, 0]`.
/// Checker: `pattern_a = [r_a, g_a, b_a, a_a]` (linear 0..1),
///          `pattern_b = [r_b, g_b, b_b, cell]`.
fn pack_pattern(kind: PatternKind) -> (u32, [f32; 4], [f32; 4]) {
    match kind {
        PatternKind::Slash {
            color,
            period,
            line_width,
        } => {
            let [r, g, b, a] = rgba8_to_linear(color);
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
            let [ra, ga, ba, aa] = rgba8_to_linear(color_a);
            let [rb, gb, bb, _ab] = rgba8_to_linear(color_b);
            (FILL_KIND_CHECKER, [ra, ga, ba, aa], [rb, gb, bb, cell])
        }
    }
}

/// Convert sRGB `Rgba8` to linear-light `[f32; 4]` (straight alpha).
fn rgba8_to_linear(color: Rgba8) -> [f32; 4] {
    [
        srgb_channel_to_linear(color.r),
        srgb_channel_to_linear(color.g),
        srgb_channel_to_linear(color.b),
        f32::from(color.a) / 255.0,
    ]
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
        corner_radii: radii.nonnegative_array(),
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
        hal::Point::new(
            surface.origin.x - outset + offset_x,
            surface.origin.y - outset + offset_y,
        ),
        surface.width + outset * 2.0,
        surface.height + outset * 2.0,
    )
}

pub(crate) fn gradient_stop_records(stops: &[GradientStop]) -> Vec<GpuStop> {
    let mut records = Vec::with_capacity(stops.len().clamp(1, MAX_GRADIENT_STOPS));
    for stop in stops.iter().take(MAX_GRADIENT_STOPS) {
        records.push(gpu_stop(*stop));
    }
    if records.is_empty() {
        records.push(gpu_stop(GradientStop::new(
            0.0,
            Rgba8 {
                r: 0,
                g: 0,
                b: 0,
                a: 0,
            },
        )));
    }
    records
}

pub(crate) fn srgb_to_linear_rgba16f(color: Rgba8) -> [f16; 4] {
    let alpha = f32::from(color.a) / 255.0;
    [
        f16::from_f32(srgb_channel_to_linear(color.r) * alpha),
        f16::from_f32(srgb_channel_to_linear(color.g) * alpha),
        f16::from_f32(srgb_channel_to_linear(color.b) * alpha),
        f16::from_f32(alpha),
    ]
}

fn gpu_stop(stop: GradientStop) -> GpuStop {
    let [r, g, b, a] = srgb_to_linear_rgba16f(stop.color);
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

pub(crate) const fn instance_stride() -> u64 {
    core::mem::size_of::<RectInstance>() as u64
}

pub(crate) const fn fill_instance_stride() -> u64 {
    core::mem::size_of::<FillInstance>() as u64
}

pub(crate) const fn shadow_instance_stride() -> u64 {
    core::mem::size_of::<ShadowInstance>() as u64
}

pub(crate) const fn gpu_stop_stride() -> u64 {
    core::mem::size_of::<GpuStop>() as u64
}

#[cfg(test)]
mod tests {
    include!("scene_instance_tests.rs");
}
