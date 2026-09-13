use crate::hal::Size;
use crate::kernel::{Cx, TextContent, container};
use crate::layout::{LayoutEngine, TextMeasure};
use crate::reactive::Runtime;

use super::SceneCache;

struct StubMeasure;

impl TextMeasure for StubMeasure {
    fn measure(&self, _: &TextContent, _: Size) -> crate::hal::LogicalSize {
        crate::hal::LogicalSize::new(0.0, 0.0)
    }
}

#[test]
fn replays_only_when_context_root_viewport_and_scale_match() {
    Runtime::scope("scene-cache-replay", || {
        let mut cx = Cx::new_root();
        let root = cx.add(container());
        let mut layout = LayoutEngine::with_text_measure(Box::new(StubMeasure));
        layout
            .compute(&mut cx, root, Size::fixed(100.0, 80.0))
            .expect("layout");
        let mut cache = SceneCache::default();

        let _ = cache.scene(&cx, root, (100.0, 80.0), false);
        let _ = cache.scene(&cx, root, (100.0, 80.0), true);
        assert_eq!(cache.emissions, 1, "matching expose redraw replays");

        let _ = cache.scene(&cx, root, (120.0, 80.0), true);
        assert_eq!(cache.emissions, 2, "viewport change rebuilds");
    });
}
