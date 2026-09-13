#![allow(
    unused_crate_dependencies,
    reason = "target does not use every package dependency"
)]
#![allow(missing_docs)]
#![allow(clippy::unwrap_used)]

use sigil::prelude::*;
use sigil::renderer::{SceneCommand, emit_scene};

struct StubMeasure;

impl TextMeasure for StubMeasure {
    fn measure(&self, _content: &TextContent, _space: sigil::hal::Size) -> sigil::hal::LogicalSize {
        sigil::hal::LogicalSize::new(40.0, 16.0)
    }
}

#[test]
fn progress_has_progress_indicator_role() {
    sigil::reactive::Runtime::scope("sigil-progress-role", || {
        let value = signal(0.0_f32);
        let mut cx = Cx::new_root();
        let root = cx.add(progress(value, 100.0));
        let node = cx.get(root).unwrap();
        assert_eq!(node.a11y.role, Role::ProgressIndicator);
    });
}

#[test]
fn progress_aria_value_reflects_signal() {
    sigil::reactive::Runtime::scope("sigil-progress-aria-value", || {
        let value = signal(50.0_f32);
        let mut cx = Cx::new_root();
        let root = cx.add(progress(value, 100.0));
        let node = cx.get(root).unwrap();
        assert_eq!(node.a11y.value_min, Some(0.0));
        assert_eq!(node.a11y.value_max, Some(100.0));
        let now = node.a11y.value_now.expect("value_now present");
        assert_approx(now.get(), 50.0);
    });
}

#[test]
fn progress_aria_value_updates_reactively() {
    sigil::reactive::Runtime::scope("sigil-progress-aria-reactive", || {
        let value = signal(0.0_f32);
        let mut cx = Cx::new_root();
        let root = cx.add(progress(value, 100.0));

        value.set(75.0);
        let node = cx.get(root).unwrap();
        let now = node.a11y.value_now.expect("value_now present");
        assert_approx(now.get(), 75.0);
    });
}

#[test]
fn progress_track_emits_border_strong_color() {
    sigil::reactive::Runtime::scope("sigil-progress-track-color", || {
        let value = signal(0.0_f32);
        let mut cx = Cx::new_root();
        let root = cx.add(progress(value, 100.0));
        let mut layout = LayoutEngine::with_text_measure(Box::new(StubMeasure));
        layout
            .compute(&mut cx, root, Size::fixed(640.0, 480.0))
            .expect("layout computes");

        let theme = Theme::default();
        let colors = scene_rect_colors(&cx, root);
        assert!(
            colors.contains(&theme.gray.step(4)),
            "track should emit the muted gray-4 color"
        );
    });
}

#[test]
fn progress_fill_emits_accent_color() {
    sigil::reactive::Runtime::scope("sigil-progress-fill-color", || {
        let value = signal(60.0_f32);
        let mut cx = Cx::new_root();
        let root = cx.add(progress(value, 100.0));
        let mut layout = LayoutEngine::with_text_measure(Box::new(StubMeasure));
        layout
            .compute(&mut cx, root, Size::fixed(640.0, 480.0))
            .expect("layout computes");

        let theme = Theme::default();
        let colors = scene_rect_colors(&cx, root);
        assert!(
            colors.contains(&theme.accent),
            "fill should emit the solid accent color"
        );
    });
}

#[test]
fn progress_indeterminate_emits_accent_fill() {
    sigil::reactive::Runtime::scope("sigil-progress-indeterminate-color", || {
        let phase = signal(0.5_f32);
        let mut cx = Cx::new_root();
        let root = cx.add(progress(phase, 1.0).indeterminate());
        let mut layout = LayoutEngine::with_text_measure(Box::new(StubMeasure));
        layout
            .compute(&mut cx, root, Size::fixed(640.0, 480.0))
            .expect("layout computes");

        let theme = Theme::default();
        let colors = scene_rect_colors(&cx, root);
        assert!(
            colors.contains(&theme.accent),
            "indeterminate fill should emit the solid accent color"
        );
    });
}

fn scene_rect_colors(cx: &Cx, root: NodeId) -> Vec<sigil::style::Rgba8> {
    emit_scene(cx, root)
        .commands
        .iter()
        .filter_map(|command| match command {
            SceneCommand::Rect(rect) => Some(rect.color),
            _ => None,
        })
        .collect()
}

fn assert_approx(actual: f32, expected: f32) {
    assert!(
        (actual - expected).abs() < 0.001,
        "expected {expected}, got {actual}"
    );
}
