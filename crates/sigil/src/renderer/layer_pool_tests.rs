#![allow(clippy::unwrap_used)]

use super::*;

#[test]
fn round_up_multiples_of_16() {
    assert_eq!(round_up(0, 16), 0);
    assert_eq!(round_up(1, 16), 16);
    assert_eq!(round_up(15, 16), 16);
    assert_eq!(round_up(16, 16), 16);
    assert_eq!(round_up(17, 16), 32);
    assert_eq!(round_up(100, 16), 112);
    assert_eq!(round_up(128, 16), 128);
}

#[test]
fn pool_starts_empty() {
    let pool = LayerPool::new();
    assert_eq!(pool.len(), 0);
    assert!(pool.is_empty());
    assert_eq!(LayerPool::MAX_ENTRIES, 4);
}

#[test]
fn a_full_screen_layer_fits_the_cache_budget() {
    let screen = (3200, 2000);
    let layer = LayerPool::bytes(screen, crate::renderer::scene_instance::LAYER_FORMAT);
    assert!(
        layer <= LayerPool::budget_for_target(screen),
        "a full-screen layer must be retained between frames, not reallocated"
    );
}
