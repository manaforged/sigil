use super::*;

impl LayoutNode {
    pub(crate) fn children(&self) -> &[LocalId] {
        &self.children
    }
}

impl LayoutForest {
    pub(crate) fn empty(&self) -> bool {
        self.count() == 0
    }

    pub(crate) const fn slab(&self) -> usize {
        self.nodes.len()
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum LayoutPass {
    Measure,
    Arrange,
}

impl AvailableSpace {
    #[must_use]
    pub(crate) const fn unbounded() -> Self {
        Self {
            w: SpaceConstraint::MaxContent,
            h: SpaceConstraint::MaxContent,
        }
    }
}

#[test]
fn empty_forest_has_no_nodes() {
    let f = LayoutForest::new();
    assert!(f.empty());
    assert_eq!(f.count(), 0);
    assert_eq!(f.slab(), 0);
}

#[test]
fn insert_returns_indexable_id() {
    let mut f = LayoutForest::new();
    let id = f.insert(None, StyleData::default());
    assert_eq!(id.slot(), 0);
    assert_eq!(id.era(), 0);
    assert_eq!(f.count(), 1);
    assert!(f.node(id).is_some());
    assert_eq!(f.node(id).unwrap().children(), &[]);
}

#[test]
fn attach_records_children_in_insert_order() {
    let mut f = LayoutForest::new();
    let parent = f.insert(None, StyleData::default());
    let a = f.insert(None, StyleData::default());
    let b = f.insert(None, StyleData::default());
    let c = f.insert(None, StyleData::default());
    f.attach(parent, a).unwrap();
    f.attach(parent, b).unwrap();
    f.attach(parent, c).unwrap();
    let kids = f.node(parent).unwrap().children();
    assert_eq!(kids, &[a, b, c]);
}

#[test]
fn freed_slot_bumps_generation_and_invalidates_stale_id() {
    let mut f = LayoutForest::new();
    let first = f.insert(None, StyleData::default());
    assert_eq!(first.era(), 0);
    f.free(first).unwrap();
    assert!(f.node(first).is_none());
    let reused = f.insert(None, StyleData::default());
    assert_eq!(reused.slot(), first.slot());
    assert_eq!(reused.era(), 1);
    assert!(f.node(reused).is_some());
    assert!(f.node(first).is_none());
}

#[test]
fn free_then_reinsert_does_not_grow_slab() {
    let mut f = LayoutForest::new();
    let a = f.insert(None, StyleData::default());
    let b = f.insert(None, StyleData::default());
    assert_eq!(f.slab(), 2);
    f.free(a).unwrap();
    let _c = f.insert(None, StyleData::default());
    assert_eq!(f.slab(), 2);
    let _ = b;
}

#[test]
fn computed_box_slab_is_independently_indexable() {
    let mut f = LayoutForest::new();
    let a = f.insert(None, StyleData::default());
    let b = f.insert(None, StyleData::default());
    let box_a = ComputedBox {
        x: 1.0,
        y: 2.0,
        w: 3.0,
        h: 4.0,
    };
    let box_b = ComputedBox {
        x: 5.0,
        y: 6.0,
        w: 7.0,
        h: 8.0,
    };
    f.write(a, box_a).unwrap();
    f.write(b, box_b).unwrap();
    assert_eq!(f.boxed(a), Some(box_a));
    assert_eq!(f.boxed(b), Some(box_b));
    let updated = ComputedBox {
        x: 9.0,
        ..ComputedBox::default()
    };
    f.write(a, updated).unwrap();
    assert_eq!(f.boxed(a), Some(updated));
    assert_eq!(f.boxed(b), Some(box_b));
}

#[test]
fn write_to_stale_id_errors() {
    let mut f = LayoutForest::new();
    let id = f.insert(None, StyleData::default());
    f.free(id).unwrap();
    let err = f.write(id, ComputedBox::default()).unwrap_err();
    assert!(matches!(err, LayoutError::LayoutInvalid(_)));
}

#[test]
fn node_ref_exposes_children_and_kernel_link() {
    let mut f = LayoutForest::new();
    let parent = f.insert(None, StyleData::default());
    let child = f.insert(None, StyleData::default());
    f.attach(parent, child).unwrap();
    let n = f.refer(parent).unwrap();
    assert_eq!(n.id(), parent);
    assert_eq!(n.children(), &[child]);
    assert!(n.kernel().is_none());
}

#[test]
fn measure_scope_borrows_forest_and_constraints() {
    struct NullText;
    impl TextMeasure for NullText {
        fn measure(
            &self,
            _: &crate::kernel::TextContent,
            _: crate::hal::Size,
        ) -> crate::hal::LogicalSize {
            crate::hal::LogicalSize::default()
        }
    }
    let f = LayoutForest::new();
    let t = NullText;
    let avail = AvailableSpace::definite(100.0, 50.0);
    let scope = MeasureScope::new(&f, &t, avail);
    assert_eq!(scope.available(), avail);
    assert!(scope.forest().empty());
    let _ = scope.text();
}

#[test]
fn available_space_helpers_round_trip() {
    let a = AvailableSpace::definite(10.0, 20.0);
    match (a.w, a.h) {
        (SpaceConstraint::Definite(w), SpaceConstraint::Definite(h)) => {
            assert!((w - 10.0).abs() < f32::EPSILON);
            assert!((h - 20.0).abs() < f32::EPSILON);
        }
        _ => panic!("expected definite axes"),
    }
    let u = AvailableSpace::unbounded();
    assert!(matches!(u.w, SpaceConstraint::MaxContent));
    assert!(matches!(u.h, SpaceConstraint::MaxContent));
}

#[test]
fn layout_pass_variants_are_distinct() {
    assert_ne!(LayoutPass::Measure, LayoutPass::Arrange);
}

#[test]
fn node_ref_style_returns_style_passed_at_insert() {
    let mut f = LayoutForest::new();
    let style = StyleData {
        flex_grow: 2.5,
        display: crate::layout::style_data::Display::Flex,
        ..StyleData::default()
    };
    let id = f.insert(None, style.clone());
    let n = f.refer(id).unwrap();
    assert_eq!(n.style(), &style);
    assert!((n.style().flex_grow - 2.5).abs() < f32::EPSILON);
    assert_eq!(n.style().display, crate::layout::style_data::Display::Flex);
}

#[test]
fn freed_slot_resets_style_to_default() {
    let mut f = LayoutForest::new();
    let style = StyleData {
        flex_grow: 7.0,
        ..StyleData::default()
    };
    let id = f.insert(None, style);
    f.free(id).unwrap();
    let reused = f.insert(None, StyleData::default());
    let n = f.refer(reused).unwrap();
    assert!((n.style().flex_grow - 0.0).abs() < f32::EPSILON);
}

struct NullText;
impl TextMeasure for NullText {
    fn measure(
        &self,
        _: &crate::kernel::TextContent,
        _: crate::hal::Size,
    ) -> crate::hal::LogicalSize {
        crate::hal::LogicalSize::default()
    }
}

#[test]
fn measure_scope_can_scatter_write_without_mut() {
    let mut f = LayoutForest::new();
    let id = f.insert(None, StyleData::default());
    let t = NullText;
    let scope = MeasureScope::new(&f, &t, AvailableSpace::unbounded());
    let value = ComputedBox {
        x: 1.0,
        y: 2.0,
        w: 3.0,
        h: 4.0,
    };
    scope.write(id, value);
    assert_eq!(f.boxed(id), Some(value));
}

#[test]
fn scatter_writes_to_distinct_ids_are_independent() {
    let mut forest = LayoutForest::new();
    let a = forest.insert(None, StyleData::default());
    let b = forest.insert(None, StyleData::default());
    let c = forest.insert(None, StyleData::default());
    let text = NullText;
    let scope = MeasureScope::new(&forest, &text, AvailableSpace::unbounded());
    let ba = ComputedBox {
        x: 1.0,
        y: 1.0,
        w: 1.0,
        h: 1.0,
    };
    let bb = ComputedBox {
        x: 2.0,
        y: 2.0,
        w: 2.0,
        h: 2.0,
    };
    let bc = ComputedBox {
        x: 3.0,
        y: 3.0,
        w: 3.0,
        h: 3.0,
    };
    scope.write(a, ba);
    scope.write(b, bb);
    scope.write(c, bc);
    assert_eq!(forest.boxed(a), Some(ba));
    assert_eq!(forest.boxed(b), Some(bb));
    assert_eq!(forest.boxed(c), Some(bc));
}

#[test]
fn write_to_stale_id_is_silent_no_op() {
    let mut f = LayoutForest::new();
    let id = f.insert(None, StyleData::default());
    let seeded = ComputedBox {
        x: 42.0,
        ..ComputedBox::default()
    };
    f.write(id, seeded).unwrap();
    f.free(id).unwrap();
    let reused = f.insert(None, StyleData::default());
    assert_eq!(reused.slot(), id.slot());
    assert_ne!(reused.era(), id.era());
    let t = NullText;
    let scope = MeasureScope::new(&f, &t, AvailableSpace::unbounded());
    scope.write(
        id,
        ComputedBox {
            x: -1.0,
            ..ComputedBox::default()
        },
    );
    assert_eq!(f.boxed(reused), Some(ComputedBox::default()));
    assert!(f.boxed(id).is_none());
}

#[test]
fn node_ref_style_on_stale_id_falls_back_to_default() {
    let mut f = LayoutForest::new();
    let id = f.insert(None, StyleData::default());
    f.free(id).unwrap();
    let live = f.insert(None, StyleData::default());
    let n = f.refer(live).unwrap();
    assert_eq!(n.style().display, crate::layout::style_data::Display::Block);
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
