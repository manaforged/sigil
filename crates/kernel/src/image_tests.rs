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
