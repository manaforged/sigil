use crate::scene::Wash;

const ENTRIES: usize = 2;

/// The texture format baked washes are stored in.
pub(crate) const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8UnormSrgb;

#[derive(Clone, Copy, PartialEq)]
struct Key {
    wash: Wash,
    size: (u32, u32),
}

/// Washes already shaded into textures, most recently used last.
#[derive(Default)]
pub(crate) struct WashCache {
    entries: Vec<(Key, wgpu::TextureView)>,
}

impl WashCache {
    /// Returns the baked texture for `wash` at `size` pixels, if one is kept.
    pub(crate) fn find(&mut self, wash: Wash, size: (u32, u32)) -> Option<wgpu::TextureView> {
        let key = Key { wash, size };
        let at = self.entries.iter().position(|(kept, _)| *kept == key)?;
        let entry = self.entries.remove(at);
        let view = entry.1.clone();
        self.entries.push(entry);
        Some(view)
    }

    /// Keeps a baked texture, dropping the least recently used one past the limit.
    pub(crate) fn insert(&mut self, wash: Wash, size: (u32, u32), view: wgpu::TextureView) {
        if self.entries.len() >= ENTRIES {
            self.entries.remove(0);
        }
        self.entries.push((Key { wash, size }, view));
    }

    /// Whether `view` is one of the kept textures.
    pub(crate) fn contains(&self, view: &wgpu::TextureView) -> bool {
        self.entries.iter().any(|(_, kept)| kept == view)
    }
}
