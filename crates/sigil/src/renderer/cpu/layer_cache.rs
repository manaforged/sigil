use tiny_skia::{Pixmap, Transform as TsTransform};

use crate::geometry::{Point, Rect};
use crate::renderer::cache_limits::{CacheLimits, FrameCache};
use crate::renderer::plan::PlanNode;
use crate::renderer::scene_command::state::SceneState;
use crate::scene::command::{LayerScope, SceneCommand, Transform};

use super::layers::take;
use super::paint::CpuPainter;

#[derive(Clone, PartialEq)]
pub(super) struct LayerKey {
    pub(super) scope: LayerScope,
    pub(super) bounds: Rect,
    pub(super) region: Rect,
    pub(super) transform: Transform,
    pub(super) place: (f32, Point),
}

pub(super) type LayerCache = FrameCache<LayerKey, Pixmap>;

impl LayerCache {
    pub(super) fn put(&mut self, key: LayerKey, commands: &[SceneCommand], pixels: &Pixmap) {
        let area = |pixmap: &Pixmap| u64::from(pixmap.width()) * u64::from(pixmap.height());
        let need = (area(pixels), CacheLimits::DEFAULT.cpu_layer_pixels);
        if self.admit(commands, need, area) {
            self.insert(key, commands, pixels.clone());
        }
    }
}

impl CpuPainter<'_> {
    pub(super) fn filtered_region(
        &mut self,
        key: LayerKey,
        (children, commands): (&[PlanNode<'_>], &[SceneCommand]),
    ) -> Option<(Pixmap, TsTransform)> {
        let (x0, y0, width, height) = self.pixel_window(key.region);
        if let Some(found) = self.cached.get(&key, commands)
            && found.width() == width
            && found.height() == height
        {
            let mut layer = take(self.pool, width, height)?;
            layer.data_mut().copy_from_slice(found.data());
            return Some((layer, TsTransform::from_translate(x0, y0)));
        }
        let state = SceneState::new(Some(key.bounds), key.transform, 1.0);
        let (mut layer, at) = self.render_region(key.region, state, children)?;
        super::filter::filter(&mut layer, &key.scope.effect, self.scale);
        self.cached.put(key, commands, &layer);
        Some((layer, at))
    }
}
