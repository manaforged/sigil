use crate::geometry::Size;
use crate::kernel::{Cx, NodeId, TextContent, container};
use crate::layout::{LayoutEngine, TextMeasure};
use crate::reactive::Runtime;

use super::SceneCache;

struct StubMeasure;

impl TextMeasure for StubMeasure {
    fn measure(&self, _: &TextContent, _: Size) -> crate::geometry::LogicalSize {
        crate::geometry::LogicalSize::new(0.0, 0.0)
    }
}

fn laid_out() -> (Cx, NodeId) {
    let mut cx = Cx::new_root();
    let root = cx.add(container());
    LayoutEngine::with_text_measure(Box::new(StubMeasure))
        .compute(&mut cx, root, Size::fixed(100.0, 80.0))
        .expect("layout");
    (cx, root)
}

#[test]
fn replays_only_when_context_root_viewport_and_scale_match() {
    Runtime::scope("scene-cache-replay", || {
        let (cx, root) = laid_out();
        let mut cache = SceneCache::default();

        let _ = cache.scene(&cx, root, (100.0, 80.0), false);
        let _ = cache.scene(&cx, root, (100.0, 80.0), true);
        assert_eq!(cache.emissions, 1, "matching expose redraw replays");

        let _ = cache.scene(&cx, root, (120.0, 80.0), true);
        assert_eq!(cache.emissions, 2, "viewport change rebuilds");
    });
}

#[test]
fn an_opaque_window_paints_the_theme_surface_behind_the_tree() {
    Runtime::scope("scene-cache-backdrop", || {
        let (cx, root) = laid_out();
        let surface = crate::style::Theme::default().color(crate::style::Token::Surface);

        let opaque = super::window_scene(&cx, root, (100.0, 80.0), true);
        let backdrop = match opaque.commands.first() {
            Some(crate::scene::SceneCommand::Rect(rect)) => Some((rect.color, rect.rect)),
            _ => None,
        };
        let (color, rect) = backdrop.expect("an opaque window starts with the backdrop");
        assert_eq!(color, surface);
        assert!(
            rect.origin.x < 0.0
                && rect.origin.y < 0.0
                && rect.origin.x + rect.width > 100.0
                && rect.origin.y + rect.height > 80.0,
            "the backdrop's edge lies outside the viewport: {rect:?}"
        );

        let clear = super::window_scene(&cx, root, (100.0, 80.0), false);
        assert_eq!(clear.commands.len() + 1, opaque.commands.len());
    });
}
