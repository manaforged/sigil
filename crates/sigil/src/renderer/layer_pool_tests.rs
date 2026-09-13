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
}

#[test]
fn max_entries_constant() {
    assert_eq!(LayerPool::MAX_ENTRIES, 4);
}
