//! Last-frame command scene replay for non-mutating OS redraws.

use sigil_hal::Color;
use sigil_kernel::{Cx, NodeId};
use sigil_reactive::Runtime;
use sigil_renderer::{CommandScene, WgpuRenderer, WgpuSurface, emit_scene};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct SceneKey {
    context: u64,
    root: NodeId,
    viewport: [u32; 2],
    scale: u32,
}

impl SceneKey {
    const fn new(cx: &Cx, root: NodeId, viewport: (f32, f32)) -> Self {
        Self {
            context: cx.instance_id(),
            root,
            viewport: [viewport.0.to_bits(), viewport.1.to_bits()],
            scale: cx.scale().factor().to_bits(),
        }
    }
}

/// Retains the previous command scene so an expose/unocclude redraw can replay
/// it without re-resolving every style and reshaping every paragraph.
#[derive(Default)]
pub(super) struct SceneCache {
    key: Option<SceneKey>,
    scene: CommandScene,
    #[cfg(test)]
    emissions: usize,
}

impl SceneCache {
    pub(super) fn scene(
        &mut self,
        cx: &Cx,
        root: NodeId,
        viewport: (f32, f32),
        allow_replay: bool,
    ) -> &CommandScene {
        let key = SceneKey::new(cx, root, viewport);
        if !allow_replay || self.key != Some(key) {
            self.scene = emit_scene(cx, root);
            self.key = Some(key);
            #[cfg(test)]
            {
                self.emissions += 1;
            }
        }
        &self.scene
    }

    pub(super) fn paint(&mut self, request: PaintRequest<'_>) {
        let PaintRequest {
            cx,
            root,
            renderer,
            surface,
            viewport,
            allow_replay,
        } = request;
        let Some(surface) = surface.as_mut() else {
            return;
        };
        let scene = self.scene(cx, root, viewport, allow_replay);
        let clear = if renderer.is_transparent() {
            Color::rgba(0.0, 0.0, 0.0, 0.0)
        } else {
            Color::rgba(0.03, 0.04, 0.05, 1.0)
        };
        if let Err(err) = renderer.render_command_scene(surface, scene, clear, viewport) {
            tracing::warn!("sigil-app: render_command_scene failed: {err}");
            Runtime::mark_frame_dirty();
        }
    }
}

pub(super) struct PaintRequest<'a> {
    pub(super) cx: &'a Cx,
    pub(super) root: NodeId,
    pub(super) renderer: &'a mut WgpuRenderer,
    pub(super) surface: &'a mut Option<WgpuSurface>,
    pub(super) viewport: (f32, f32),
    pub(super) allow_replay: bool,
}

#[cfg(test)]
mod tests {
    use sigil_hal::Size;
    use sigil_kernel::{Cx, TextContent, container};
    use sigil_layout::{LayoutEngine, TextMeasure};
    use sigil_reactive::Runtime;

    use super::SceneCache;

    struct StubMeasure;

    impl TextMeasure for StubMeasure {
        fn measure(&self, _: &TextContent, _: Size) -> sigil_hal::LogicalSize {
            sigil_hal::LogicalSize::new(0.0, 0.0)
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
}
