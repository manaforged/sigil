#![allow(clippy::redundant_pub_crate)]
//! Per-instance data + collection helpers for [`crate::scene_paint`].
//!
//! Split out of `scene_paint.rs` to respect the 400-line hygiene cap
//! and to colocate instance-data tests with the helpers they cover.

use bytemuck::{Pod, Zeroable};
use sigil_hal::Rect;
use sigil_style::Rgba8;

use crate::scene::{Scene, SceneItem, SceneRect};
use crate::scene_command::DrawRect;

/// Per-instance GPU record for one rectangle.
#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub(crate) struct RectInstance {
    /// `[x, y, w, h]` in logical pixels.
    pub(crate) rect: [f32; 4],
    /// Linear RGBA in `[0, 1]`.
    pub(crate) color: [f32; 4],
    /// `[radius_px, opacity_01]`.
    pub(crate) params: [f32; 2],
    /// Pad to 16-byte alignment.
    pub(crate) _pad: [f32; 2],
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
        rect.border_radius.map_or(0.0_f32, |px| f32::from(px.0)),
        f32::from(rect.opacity) / 10_000.0,
    )
}

pub(crate) fn draw_rect_instance(draw: &DrawRect, opacity_scale: f32) -> RectInstance {
    rect_instance_parts(
        draw.rect,
        draw.color,
        draw.border_radius.map_or(0.0_f32, |px| f32::from(px.0)),
        (f32::from(draw.opacity) / 10_000.0) * opacity_scale,
    )
}

fn rect_instance_parts(rect: Rect, color: Rgba8, radius: f32, opacity: f32) -> RectInstance {
    let Rect {
        origin,
        width,
        height,
    } = rect;
    let color = [
        f32::from(color.r) / 255.0,
        f32::from(color.g) / 255.0,
        f32::from(color.b) / 255.0,
        f32::from(color.a) / 255.0,
    ];
    RectInstance {
        rect: [origin.x, origin.y, width, height],
        color,
        params: [radius.max(0.0), opacity.clamp(0.0, 1.0)],
        _pad: [0.0, 0.0],
    }
}

pub(crate) const fn instance_stride() -> u64 {
    core::mem::size_of::<RectInstance>() as u64
}

#[cfg(test)]
mod tests {
    use sigil_core::{Cx, NodeId, TextContent, prelude::*};
    use sigil_hal::Size;
    use sigil_layout::{LayoutBuilderExt, LayoutEngine, LayoutStyle, TextMeasure};
    use sigil_style::NodeStyleExt;
    use taffy::prelude::{AvailableSpace, Size as TaffySize};

    use super::{RectInstance, collect_instances, instance_stride};
    use crate::scene::emit_scene;

    struct StubMeasure;
    impl TextMeasure for StubMeasure {
        fn measure(&self, _: &TextContent, _: TaffySize<AvailableSpace>) -> TaffySize<f32> {
            TaffySize {
                width: 0.0,
                height: 0.0,
            }
        }
    }

    fn lay_out(cx: &mut Cx, root: NodeId) {
        LayoutEngine::with_text_measure(Box::new(StubMeasure))
            .compute(cx, root, Size::fixed(320.0, 240.0))
            .expect("compute succeeds");
    }

    /// Building a tree with two `bg-accent` containers + one `text(..)`
    /// child must yield exactly two instances (rects only). The text
    /// node is dropped because Phase 14 wires text paint, not 08a.
    #[test]
    fn collect_instances_drops_text_items() {
        let mut cx = Cx::new_root();
        let root = cx.add(
            container()
                .layout(LayoutStyle::flex_row().fixed_size(120.0, 48.0))
                .child(
                    container()
                        .layout(LayoutStyle::block().fixed_size(60.0, 48.0))
                        .class("bg-accent"),
                )
                .child(text("dropped"))
                .child(
                    container()
                        .layout(LayoutStyle::block().fixed_size(60.0, 48.0))
                        .class("bg-accent"),
                ),
        );
        lay_out(&mut cx, root);
        #[allow(deprecated)]
        let scene = emit_scene(&cx, root).to_flat();
        let instances = collect_instances(&scene);
        assert_eq!(
            instances.len(),
            2,
            "text node must not appear in instance buffer (Phase 14 wires text paint)"
        );
    }

    /// The instance buffer entry for a colored rect must carry the
    /// rect's logical-pixel position + size + 0..1-normalized color.
    #[test]
    fn rect_instance_carries_position_and_color() {
        let mut cx = Cx::new_root();
        let root = cx.add(
            container()
                .layout(LayoutStyle::block().fixed_size(120.0, 48.0))
                .class("bg-accent"),
        );
        lay_out(&mut cx, root);
        #[allow(deprecated)]
        let scene = emit_scene(&cx, root).to_flat();
        let instances = collect_instances(&scene);
        assert_eq!(instances.len(), 1);
        let inst: &RectInstance = instances.first().expect("one instance");
        assert!((inst.rect[2] - 120.0).abs() < 1e-6);
        assert!((inst.rect[3] - 48.0).abs() < 1e-6);
        for ch in inst.color {
            assert!(
                (0.0..=1.0).contains(&ch),
                "color channel {ch} must be normalized"
            );
        }
        // Default opacity is full = 10_000 → 1.0 after normalization.
        assert!((inst.params[1] - 1.0).abs() < 1e-6);
    }

    #[test]
    fn instance_stride_matches_struct_size() {
        assert_eq!(
            instance_stride(),
            core::mem::size_of::<RectInstance>() as u64
        );
    }
}
