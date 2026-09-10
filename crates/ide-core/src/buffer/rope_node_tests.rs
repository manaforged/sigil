use super::*;

#[test]
fn empty_has_zero_counts() {
    let n = Node::empty();
    assert_eq!(n.counts(), Counts::default());
    assert!(n.is_leaf());
}

#[test]
fn leaves_split_at_char_boundary() {
    let big: String = "🦀".repeat(LEAF_TARGET);
    let leaves = leaves_from_str(&big);
    assert!(!leaves.is_empty());
    for l in &leaves {
        assert!(std::str::from_utf8(l.text.as_bytes()).is_ok());
    }
    let total: usize = leaves.iter().map(|l| l.counts.bytes).sum();
    assert_eq!(total, big.len());
}

#[test]
fn build_from_leaves_balances() {
    let s: String = "a".repeat(LEAF_TARGET * 50);
    let leaves = leaves_from_str(&s);
    let root = build_from_leaves(leaves);
    assert_eq!(root.len_bytes(), s.len());
    assert!(root.depth() < 5);
}
