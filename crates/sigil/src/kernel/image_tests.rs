#![allow(clippy::unwrap_used)]

use super::*;

#[test]
fn rejects_mismatched_buffer_length() {
    assert!(ImageSource::from_rgba8(2, 2, vec![0; 15]).is_none());
    assert!(ImageSource::from_rgba8(2, 2, vec![0; 16]).is_some());
    assert!(ImageSource::from_rgba8(0, 4, Vec::new()).is_none());
}

#[test]
fn content_id_is_stable_and_distinguishes_pixels() {
    let a = ImageSource::from_rgba8(1, 1, vec![10, 20, 30, 255]).unwrap();
    let same = ImageSource::from_rgba8(1, 1, vec![10, 20, 30, 255]).unwrap();
    let different = ImageSource::from_rgba8(1, 1, vec![11, 20, 30, 255]).unwrap();
    assert_eq!(a.id(), same.id(), "identical pixels must hash equal");
    assert_ne!(a.id(), different.id(), "a changed pixel must change the id");
    assert_eq!(a, same);
    assert_ne!(a, different);
}

#[test]
fn cached_image_builds_each_key_once() {
    use std::cell::Cell;
    let builds = Cell::new(0);
    let make = || {
        builds.set(builds.get() + 1);
        ImageSource::from_rgba8(1, 1, vec![1, 2, 3, 255])
    };
    let key = 0x5eed_u64;
    let first = cached_image(key, make);
    let again = cached_image(key, || {
        builds.set(builds.get() + 1);
        None
    });
    assert_eq!(builds.get(), 1, "a cached key does not build again");
    assert_eq!(first, again);
    let _ = cached_image(key + 1, make);
    assert_eq!(builds.get(), 2, "a new key builds");
}
