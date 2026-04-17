#![allow(missing_docs)]
#![allow(clippy::float_cmp, clippy::unwrap_used)]

use ahash as _;
use bytemuck as _;
use pollster as _;
use raw_window_handle as _;
use sigil_text as _;
use skrifa as _;
use tracing as _;
use wgpu as _;
#[cfg(windows)]
use windows_core as _;

use sigil_core::{TextContent, prelude::*};
use sigil_hal::Size;
use sigil_layout::{LayoutBuilderExt, LayoutEngine, LayoutResult, LayoutStyle, TextMeasure};
use sigil_renderer::{SceneItem, emit_scene, unsupported_style_warning_count_for};
use sigil_style::{NodeStyleExt, Theme};
use taffy::prelude::{AvailableSpace, Size as TaffySize};

struct StubMeasure;

impl TextMeasure for StubMeasure {
    fn measure(&self, _source: &TextContent, _space: TaffySize<AvailableSpace>) -> TaffySize<f32> {
        TaffySize {
            width: 0.0,
            height: 0.0,
        }
    }
}

#[test]
fn theme_accent_rect_emits_in_scene() {
    let mut cx = Cx::new_root();
    let root = cx.add(
        container()
            .layout(LayoutStyle::block().fixed_size(120.0, 48.0))
            .class("bg-accent"),
    );

    layout(&mut cx, root);

    let scene = emit_scene(&cx, root);
    let rect = scene
        .items()
        .iter()
        .find_map(|item| match item {
            SceneItem::Rect(rect) => Some(rect),
            SceneItem::Text(_) => None,
        })
        .unwrap();

    assert_eq!(rect.node, root);
    assert_eq!(rect.rect.width, 120.0);
    assert_eq!(rect.rect.height, 48.0);
    assert_eq!(rect.color, Theme::default().accent);
}

#[test]
fn unsupported_shadow_warns_once_across_frames() {
    let mut cx = Cx::new_root();
    let root = cx.add(
        container()
            .layout(LayoutStyle::block().fixed_size(32.0, 32.0))
            .class("shadow-xl"),
    );

    layout(&mut cx, root);

    for _frame in 0..60 {
        let _scene = emit_scene(&cx, root);
    }

    assert_eq!(unsupported_style_warning_count_for("shadow-xl", root), 1);
}

fn layout(cx: &mut Cx, root: NodeId) {
    let mut engine = LayoutEngine::with_text_measure(Box::new(StubMeasure));
    engine
        .compute(cx, root, Size::fixed(320.0, 240.0))
        .expect("layout compute succeeds");
    assert!(
        cx.get(root)
            .and_then(|node| node.get_extension::<LayoutResult>())
            .is_some()
    );
}
