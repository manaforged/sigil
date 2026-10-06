#![allow(clippy::unwrap_used)]

use std::cell::Cell;

use super::*;

fn pixel(seed: u64) -> Option<ImageSource> {
    let bytes = seed.to_le_bytes();
    ImageSource::from_rgba8(1, 1, vec![bytes[0], bytes[1], bytes[2], 255])
}

#[test]
fn an_image_in_use_survives_a_full_cache() {
    let builds = Cell::new(0);
    let hot = 0xfeed_0000_u64;
    let make_hot = || {
        builds.set(builds.get() + 1);
        pixel(hot)
    };
    let _ = cached_image(hot, make_hot);
    for key in 1..=(CACHE_CAP as u64 * 2) {
        let _ = cached_image(key, || pixel(key));
        let _ = cached_image(hot, make_hot);
    }
    assert_eq!(
        builds.get(),
        1,
        "the image used on every step was evicted and decoded again"
    );
}

#[test]
fn the_least_recently_used_image_leaves_first() {
    let cold = 0xc01d_0000_u64;
    let _ = cached_image(cold, || pixel(cold));
    for key in 1..=(CACHE_CAP as u64) {
        let _ = cached_image(key, || pixel(key));
    }
    let rebuilt = Cell::new(false);
    let _ = cached_image(cold, || {
        rebuilt.set(true);
        pixel(cold)
    });
    assert!(rebuilt.get(), "the oldest image should have been evicted");
}
