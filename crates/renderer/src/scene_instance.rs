#![allow(clippy::redundant_pub_crate)]
//! Per-instance data + collection helpers for [`crate::scene_paint`].
//!
//! Split out of `scene_paint.rs` to respect the 400-line hygiene cap
//! and to colocate instance-data tests with the helpers they cover.

use bytemuck::{Pod, Zeroable};
use half::f16;
use sigil_hal::Rect;
use sigil_style::Rgba8;

use crate::corner_radii::CornerRadii;
use crate::scene::{Scene, SceneItem, SceneRect};
use crate::scene_command::{DrawGradient, DrawPattern, DrawRect, GradientStop, PatternKind};
use crate::shadow_paint::{Shadow, ShadowKind};

mod helpers;

use helpers::{
    RectPaint, inflate, rect_fields, rect_instance_parts, shadow_color, stroke_fields,
    stroke_outset,
};

/// Color space in which translucent fills are alpha-composited.
///
/// CSS/web blends `rgba()` over its backdrop in gamma (sRGB-encoded) space;
/// physically-correct renderers blend in linear-light. The two diverge only for
/// *translucent* pixels — opaque fills round-trip identically — but for a
/// near-white border at ~10% alpha the linear blend lands ~2.5x brighter,
/// turning a faint hairline into a visible rule.
///
/// The space is implied by the render target's format: an sRGB target makes the
/// GPU decode→blend→encode in linear ([`Self::Linear`]); a plain `Unorm` target
/// stores and blends the bytes as authored, matching CSS ([`Self::Gamma`]). The
/// theme tokens are authored for the CSS model, so picking the format is the
/// single lever that selects which compositing math the whole scene uses.
///
/// Coverage: the rect-fill, glyph, gradient, pattern, sprite, and decoration
/// paths all honor this (each converts its colour where the target format is
/// known). The one exception is **shadow** colour, which bakes the sRGB→linear
/// curve at construction (`Shadow::glow`), before any format exists — so a
/// `Gamma` target tints shadows in linear. Shadow colours are near-black drop
/// shadows where the curve barely moves, so this is left as a known caveat
/// rather than reshaping the public `Shadow` API.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum BlendSpace {
    /// Physically-correct linear-light compositing (sRGB render target).
    Linear,
    /// CSS-faithful gamma-space compositing (`Unorm` render target).
    Gamma,
}

impl BlendSpace {
    /// The compositing space implied by a render target `format`. sRGB formats
    /// blend in linear (hardware decodes/encodes around the blend); everything
    /// else stores the authored bytes and blends them directly, as CSS does.
    pub(crate) fn from_format(format: wgpu::TextureFormat) -> Self {
        if format.is_srgb() {
            Self::Linear
        } else {
            Self::Gamma
        }
    }

    /// Map one sRGB-encoded `0..=255` channel to the float the fragment shader
    /// should emit for this blend space. `Linear` decodes the sRGB transfer
    /// curve (the GPU re-encodes on store); `Gamma` passes the encoded value
    /// straight through so the hardware blend operates on gamma values like CSS.
    fn channel(self, channel: u8) -> f32 {
        match self {
            Self::Linear => srgb_channel_to_linear(channel),
            Self::Gamma => f32::from(channel) / 255.0,
        }
    }

    /// Convert a straight-alpha `Rgba8` to the `[r, g, b, a]` the shader emits.
    pub(crate) fn rgba(self, color: Rgba8) -> [f32; 4] {
        [
            self.channel(color.r),
            self.channel(color.g),
            self.channel(color.b),
            f32::from(color.a) / 255.0,
        ]
    }

    /// Convert an `Rgba8` to the *premultiplied* `f16` `[r, g, b, a]` gradient
    /// stops are packed as — the same space choice as [`Self::rgba`], for the
    /// fill pipeline's interpolation.
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
        rect.border_radius,
        RectPaint {
            color: BlendSpace::Linear.rgba(rect.color),
            opacity: f32::from(rect.opacity) / 10_000.0,
            stroke: None,
            space: BlendSpace::Linear,
        },
    )
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

/// Build a `FillInstance` for a pattern fill.
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

/// Encode pattern parameters into the two `[f32; 4]` GPU slots.
///
/// Slash:   `pattern_a = [r, g, b, a]` (linear 0..1),
///          `pattern_b = [period, line_width, 0, 0]`.
/// Checker: `pattern_a = [r_a, g_a, b_a, a_a]` (linear 0..1),
///          `pattern_b = [r_b, g_b, b_b, cell]`.
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
        sigil_hal::Point::new(
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
mod tests {
    include!("scene_instance_tests.rs");
}
