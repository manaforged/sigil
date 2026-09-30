#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]
#![allow(clippy::unwrap_used, clippy::panic, clippy::cast_precision_loss)]

#[path = "renderer_child_paint_repro/support.rs"]
#[allow(dead_code)]
mod support;

use sigil::geometry::Size;
use sigil::kernel::*;
use sigil::layout::{LayoutBuilderExt, LayoutEngine, LayoutStyle};
use sigil::scene::{SceneCommand, ShadowKind, emit_scene};
use sigil::style::{Elevation, Paint, Token};
use support::TEXT_MEASURE;

const SIZE: u32 = 64;

fn layout(cx: &mut Cx, root: NodeId) {
    let mut engine = LayoutEngine::with_text_measure(Box::new(TEXT_MEASURE));
    engine
        .compute(cx, root, Size::fixed(SIZE as f32, SIZE as f32))
        .expect("layout compute");
}

fn shadow_extent(elevation: Option<Elevation>) -> Option<f32> {
    let mut cx = Cx::new_root();
    let node = container().layout(LayoutStyle::block().fixed_size(40.0, 40.0));
    let root = cx.add(match elevation {
        Some(elevation) => node.shadow(elevation),
        None => node.bg(Token::Surface),
    });
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
            .bg(Token::Surface)
            .shadow(Elevation::Lg),
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
    let sm = shadow_extent(Some(Elevation::Sm)).expect("shadow-sm emits");
    let base = shadow_extent(Some(Elevation::Base)).expect("shadow emits");
    let xl = shadow_extent(Some(Elevation::Xl)).expect("shadow-xl emits");
    assert!(
        sm < base && base < xl,
        "higher elevation must cast a larger shadow: sm={sm}, base={base}, xl={xl}",
    );
}

#[test]
fn no_shadow_class_emits_no_shadow_command() {
    assert!(
        shadow_extent(None).is_none(),
        "a plain surface must not emit a shadow",
    );
}
