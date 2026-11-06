use tiny_skia::{Pixmap, Transform as TsTransform};

use crate::geometry::{Point, Rect};
use crate::renderer::cache_limits::CacheLimits;
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

struct Entry {
    key: LayerKey,
    commands: Vec<SceneCommand>,
    pixels: Pixmap,
    used: u64,
}

#[derive(Default)]
pub(super) struct LayerCache {
    entries: Vec<Entry>,
    frame: u64,
}

impl LayerCache {
    pub(super) fn begin_frame(&mut self) {
        self.frame += 1;
        let (frame, stale) = (self.frame, CacheLimits::DEFAULT.cpu_layer_stale_frames);
        self.entries.retain(|entry| entry.used + stale >= frame);
    }

    pub(super) fn get(&mut self, key: &LayerKey, commands: &[SceneCommand]) -> Option<&Pixmap> {
        let frame = self.frame;
        let entry = self
            .entries
            .iter_mut()
            .find(|entry| entry.key == *key && entry.commands == commands)?;
        entry.used = frame;
        Some(&entry.pixels)
    }

    pub(super) fn put(&mut self, key: LayerKey, commands: &[SceneCommand], pixels: &Pixmap) {
        let limits = CacheLimits::DEFAULT;
        if commands.len() < limits.layer_min_commands {
            return;
        }
        let area = |pixmap: &Pixmap| u64::from(pixmap.width()) * u64::from(pixmap.height());
        let mut held: u64 = self.entries.iter().map(|entry| area(&entry.pixels)).sum();
        while held + area(pixels) > limits.cpu_layer_pixels {
            let Some(oldest) = self.oldest() else {
                return;
            };
            held -= area(&self.entries.swap_remove(oldest).pixels);
        }
        self.entries.push(Entry {
            key,
            commands: commands.to_vec(),
            pixels: pixels.clone(),
            used: self.frame,
        });
    }

    fn oldest(&self) -> Option<usize> {
        self.entries
            .iter()
            .enumerate()
            .filter(|(_, entry)| entry.used < self.frame)
            .min_by_key(|(_, entry)| entry.used)
            .map(|(index, _)| index)
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
