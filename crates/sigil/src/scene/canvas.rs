use std::rc::Rc;

use crate::geometry::Rect;
use crate::kernel::Node;
use crate::reactive::{Runtime, Signal, effect_named, signal};
use crate::scene::SceneBuilder;
use crate::style::Theme;
use crate::style::shader_paint::ShaderPaints;

pub(crate) type Recorder = Rc<dyn Fn(&mut SceneBuilder, Rect, &Theme)>;

#[derive(Clone)]
pub(crate) struct CanvasPaint {
    pub(crate) record: Recorder,
    pub(crate) version: Option<Signal<u64>>,
}

impl CanvasPaint {
    pub(crate) fn themed(record: Recorder) -> Self {
        Self {
            record,
            version: None,
        }
    }

    pub(crate) fn record(&self, scene: &mut SceneBuilder, rect: Rect, theme: &Theme) {
        scene.clip(rect, |scene| {
            let start = scene.recorded();
            (self.record)(scene, rect, theme);
            scene.mark_leaf_from(start);
            scene.theme_from(start, theme);
        });
    }

    pub(crate) fn version(&self) -> u64 {
        self.version.and_then(Signal::try_peek).unwrap_or_default()
    }
}

pub(crate) fn watch(read: impl Fn() + 'static) -> Signal<u64> {
    let version = signal(0_u64);
    effect_named("sigil-scene::redraw", move || {
        read();
        version.update(|n| *n = n.wrapping_add(1));
        Runtime::mark_frame_dirty();
    });
    version
}

pub(crate) fn paint_version(node: &Node) -> u64 {
    let canvas = node
        .extension::<CanvasPaint>()
        .map_or(0, CanvasPaint::version);
    let shaders = node
        .extension::<ShaderPaints>()
        .map_or(0, ShaderPaints::version);
    canvas.wrapping_add(shaders)
}
