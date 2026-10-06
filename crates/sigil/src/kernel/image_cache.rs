use std::cell::RefCell;
use std::collections::HashMap;

use super::image::ImageSource;

const CACHE_CAP: usize = 256;
const CACHE_BYTES: usize = 64 << 20;

#[derive(Default)]
struct Cache {
    entries: HashMap<u64, (ImageSource, u64)>,
    held: usize,
    clock: u64,
}

impl Cache {
    fn get(&mut self, key: u64) -> Option<ImageSource> {
        self.clock += 1;
        let clock = self.clock;
        self.entries.get_mut(&key).map(|(source, used)| {
            *used = clock;
            source.clone()
        })
    }

    fn evict_for(&mut self, size: usize) {
        while self.entries.len() >= CACHE_CAP || self.held + size > CACHE_BYTES {
            let Some(oldest) = self
                .entries
                .iter()
                .min_by_key(|(_, (_, used))| *used)
                .map(|(key, _)| *key)
            else {
                return;
            };
            if let Some((gone, _)) = self.entries.remove(&oldest) {
                self.held -= gone.rgba().len();
            }
        }
    }

    fn insert(&mut self, key: u64, source: ImageSource) {
        let size = source.rgba().len();
        self.evict_for(size);
        self.clock += 1;
        self.held += size;
        if let Some((old, _)) = self.entries.insert(key, (source, self.clock)) {
            self.held -= old.rgba().len();
        }
    }
}

thread_local! {
    static CACHE: RefCell<Cache> = RefCell::new(Cache::default());
}

pub(crate) fn cached_image(
    key: u64,
    make: impl FnOnce() -> Option<ImageSource>,
) -> Option<ImageSource> {
    if let Some(hit) = CACHE.with(|cache| cache.borrow_mut().get(key)) {
        return Some(hit);
    }
    let source = make()?;
    CACHE.with(|cache| cache.borrow_mut().insert(key, source.clone()));
    Some(source)
}

#[cfg(test)]
#[path = "image_cache_tests.rs"]
mod tests;
