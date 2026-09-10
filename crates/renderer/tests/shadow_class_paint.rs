#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]
#![allow(missing_docs)]
#![allow(clippy::unwrap_used, clippy::panic, clippy::cast_precision_loss)]

#[path = "child_paint_repro/support.rs"]
#[allow(dead_code)]
mod support;

use sigil_hal::Size;
use sigil_kernel::{Cx, prelude::*};
use sigil_layout::{LayoutBuilderExt, LayoutEngine, LayoutStyle};
use sigil_renderer::{SceneCommand, ShadowKind, emit_scene};
use sigil_style::NodeStyleExt;
use support::StubMeasure;

const SIZE: u32 = 64;

fn layout(cx: &mut Cx, root: NodeId) {
    let mut engine = LayoutEngine::with_text_measure(Box::new(StubMeasure));
    engine
        .compute(cx, root, Size::fixed(SIZE as f32, SIZE as f32))
        .expect("layout compute");
}

fn shadow_extent(class: &'static str) -> Option<f32> {
    let mut cx = Cx::new_root();
    let root = cx.add(
        container()
            .layout(LayoutStyle::block().fixed_size(40.0, 40.0))
            .class(class),
    );
    layout(&mut cx, root);
    emit_scene(&cx, root)
        .commands
        .iter()
        .find_map(|command| match command {
            SceneCommand::DrawShadow { shadow, .. } => Some(shadow.blur + shadow.offset[1]),
            _ => None,
        })
}

#[test]
fn shadow_class_emits_translucent_outer_drop_shadow() {
    let mut cx = Cx::new_root();
    let root = cx.add(
        container()
            .layout(LayoutStyle::block().fixed_size(40.0, 40.0))
            .class("bg-surface shadow-lg"),
    );
    layout(&mut cx, root);

    let shadow = emit_scene(&cx, root)
        .commands
        .iter()
        .find_map(|command| match command {
            SceneCommand::DrawShadow { shadow, .. } => Some(*shadow),
            _ => None,
        })
        .expect("shadow-lg must emit a DrawShadow command");
    assert_eq!(shadow.kind, ShadowKind::Outer);
    assert!(shadow.blur > 0.0, "elevation must carry a blur radius");
    assert!(
        shadow.offset[1] > 0.0,
        "a drop shadow sits below its surface"
    );
    assert!(
        shadow.color[3] > 0.0 && shadow.color[3] < 1.0,
        "shadow ink is a translucent black, got alpha {}",
        shadow.color[3],
    );
}

#[test]
fn elevation_scale_is_monotonic() {
    let sm = shadow_extent("shadow-sm").expect("shadow-sm emits");
    let base = shadow_extent("shadow").expect("shadow emits");
    let xl = shadow_extent("shadow-xl").expect("shadow-xl emits");
    assert!(
        sm < base && base < xl,
        "higher elevation must cast a larger shadow: sm={sm}, base={base}, xl={xl}",
    );
}

#[test]
fn no_shadow_class_emits_no_shadow_command() {
    assert!(
        shadow_extent("bg-surface").is_none(),
        "a plain surface must not emit a shadow",
    );
}
