use std::rc::Rc;

use crate::geometry::Rect;
use crate::reactive::Signal;
use crate::scene::SceneBuilder;
use crate::style::Theme;

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
        });
    }

    pub(crate) fn version(&self) -> u64 {
        self.version.and_then(Signal::try_peek).unwrap_or_default()
    }
}
