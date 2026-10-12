#![allow(clippy::indexing_slicing)]

use super::*;

impl LayoutNode {
    pub(crate) fn children(&self) -> &[LocalId] {
        &self.children
    }
}

impl LayoutForest {
    pub(crate) const fn slab(&self) -> usize {
        self.nodes.len()
    }
}

struct NullText;

impl TextMeasure for NullText {
    fn measure(
        &self,
        _: &crate::kernel::TextContent,
        _: crate::geometry::Size,
    ) -> crate::geometry::LogicalSize {
        crate::geometry::LogicalSize::default()
    }
}

fn forest(count: usize) -> (LayoutForest, Vec<LocalId>) {
    let mut f = LayoutForest::new();
    let ids = (0..count)
        .map(|_| f.insert(None, StyleData::default()))
        .collect();
    (f, ids)
}

const fn cb(x: f32, y: f32, w: f32, h: f32) -> ComputedBox {
    ComputedBox { x, y, w, h }
}

const UNBOUNDED: AvailableSpace = AvailableSpace {
    w: SpaceConstraint::MaxContent,
    h: SpaceConstraint::MaxContent,
};

#[test]
fn insert_and_attach_build_an_indexable_tree() {
    let (empty, _) = forest(0);
    assert_eq!((empty.count(), empty.slab()), (0, 0));
    let (mut tree, ids) = forest(4);
    let [parent, a, b, c] = ids[..] else {
        panic!("four ids")
    };
    assert_eq!((parent.slot(), parent.era()), (0, 0));
    assert_eq!(tree.node(parent).unwrap().children(), &[]);
    for child in [a, b, c] {
        tree.attach(parent, child).unwrap();
    }
    assert_eq!(tree.node(parent).unwrap().children(), &[a, b, c]);
    let node = tree.refer(parent).unwrap();
    assert_eq!(node.id(), parent);
    assert_eq!(node.children(), &[a, b, c]);
    assert!(node.kernel().is_none());
    let style = StyleData {
        flex_grow: 2.5,
        display: crate::layout::style_data::Display::Flex,
        ..StyleData::default()
    };
    let styled = tree.insert(None, style.clone());
    assert_eq!(tree.refer(styled).unwrap().style(), &style);
}

#[test]
fn freed_slot_is_reused_with_a_new_era_and_default_style() {
    let mut f = LayoutForest::new();
    let first = f.insert(
        None,
        StyleData {
            flex_grow: 7.0,
            ..StyleData::default()
        },
    );
    let _other = f.insert(None, StyleData::default());
    f.write(first, cb(42.0, 0.0, 0.0, 0.0)).unwrap();
    f.free(first).unwrap();
    assert!(f.node(first).is_none());
    assert!(matches!(
        f.write(first, ComputedBox::default()).unwrap_err(),
        LayoutError::LayoutInvalid(_)
    ));
    let reused = f.insert(None, StyleData::default());
    assert_eq!((reused.slot(), reused.era()), (first.slot(), 1));
    assert_eq!(f.slab(), 2);
    assert!(f.node(reused).is_some());
    assert!(f.node(first).is_none());
    let n = f.refer(reused).unwrap();
    assert!((n.style().flex_grow - 0.0).abs() < f32::EPSILON);
    assert_eq!(n.style().display, crate::layout::style_data::Display::Block);
    let scope = MeasureScope::new(&f, &NullText, UNBOUNDED);
    scope.write(first, cb(-1.0, 0.0, 0.0, 0.0));
    assert_eq!(f.boxed(reused), Some(ComputedBox::default()));
    assert!(f.boxed(first).is_none());
}

#[test]
fn boxes_are_written_per_id_directly_and_through_a_scope() {
    let (mut f, ids) = forest(3);
    let boxes = [cb(1.0, 2.0, 3.0, 4.0), cb(5.0, 6.0, 7.0, 8.0)];
    for (id, value) in ids.iter().zip(boxes) {
        f.write(*id, value).unwrap();
    }
    assert_eq!(f.boxed(ids[0]), Some(boxes[0]));
    assert_eq!(f.boxed(ids[1]), Some(boxes[1]));
    let updated = cb(9.0, 0.0, 0.0, 0.0);
    f.write(ids[0], updated).unwrap();
    assert_eq!(f.boxed(ids[0]), Some(updated));
    assert_eq!(f.boxed(ids[1]), Some(boxes[1]));
    let avail = AvailableSpace::definite(100.0, 50.0);
    let scope = MeasureScope::new(&f, &NullText, avail);
    assert_eq!(scope.available(), avail);
    assert_eq!(scope.forest().count(), 3);
    let _ = scope.text();
    let scattered = [1.0, 2.0, 3.0].map(|v| cb(v, v, v, v));
    for (id, value) in ids.iter().zip(scattered) {
        scope.write(*id, value);
    }
    for (id, value) in ids.iter().zip(scattered) {
        assert_eq!(f.boxed(*id), Some(value));
    }
}

#[test]
fn definite_space_keeps_both_axes() {
    let a = AvailableSpace::definite(10.0, 20.0);
    match (a.w, a.h) {
        (SpaceConstraint::Definite(w), SpaceConstraint::Definite(h)) => {
            assert!((w - 10.0).abs() < f32::EPSILON);
            assert!((h - 20.0).abs() < f32::EPSILON);
        }
        _ => panic!("expected definite axes"),
    }
}

#[test]
fn text_memo_keeps_recent_entries_across_overflow() {
    let key = |index: u32| TextMemoKey {
        index,
        tag: 0,
        known_w: 0,
        known_h: 0,
        wrap: 0,
        content: 0,
        fonts: 0,
    };
    let size = crate::layout::style_data::Size {
        width: 1.0,
        height: 2.0,
    };
    let live = u32::try_from(TEXT_MEMO_LIVE).expect("memo cap fits u32");
    let mut memo = TextMemo::default();
    for index in 0..live {
        memo.store(key(index), size);
    }
    memo.store(key(u32::MAX), size);
    assert_eq!(memo.live.len(), 1);
    assert_eq!(memo.get(key(7)), Some(size));
    assert_eq!(memo.live.len(), 2);
    assert_eq!(memo.stale.len(), TEXT_MEMO_LIVE - 1);
    for index in 0..2 * live {
        memo.store(key(1_000_000 + index), size);
    }
    assert_eq!(memo.get(key(7)), None);
}
