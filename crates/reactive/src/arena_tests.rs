use super::*;

#[test]
fn insert_get_remove_roundtrip() {
    let mut arena = Arena::<u32>::new();
    let a = arena.insert(1);
    let b = arena.insert(2);
    assert_eq!(arena.get(a), Some(&1));
    assert_eq!(arena.get(b), Some(&2));
    assert_eq!(arena.remove(a), Some(1));
    assert_eq!(arena.get(a), None);
    assert!(!arena.contains(a));
}

#[test]
fn stale_key_after_reuse_returns_none() {
    let mut arena = Arena::<&'static str>::new();
    let a = arena.insert("first");
    arena.remove(a);
    let b = arena.insert("second");
    // Same slot index, different generation.
    assert_eq!(arena.get(a), None);
    assert_eq!(arena.get(b), Some(&"second"));
}
