use std::cell::RefCell;
use std::rc::Rc;

use smallvec::SmallVec;

use super::{LayoutForest, LayoutText, LocalId};
use crate::kernel::TextContent;
use crate::layout::forest::{AvailableSpace, SpaceConstraint};

const ENTRIES_PER_NODE: usize = 8;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct IntrinsicKey {
    main: u32,
    cross: u32,
    along_main: bool,
    available: (u64, u64),
    fonts: u64,
}

impl IntrinsicKey {
    pub(crate) const fn new(
        cb: (f32, f32),
        along_main: bool,
        available: AvailableSpace,
        fonts: u64,
    ) -> Self {
        Self {
            main: cb.0.to_bits(),
            cross: cb.1.to_bits(),
            along_main,
            available: (constraint_bits(available.w), constraint_bits(available.h)),
            fonts,
        }
    }
}

const fn constraint_bits(constraint: SpaceConstraint) -> u64 {
    match constraint {
        SpaceConstraint::Definite(px) => px.to_bits() as u64,
        SpaceConstraint::MinContent => 1 << 32,
        SpaceConstraint::MaxContent => 2 << 32,
    }
}

#[derive(Debug, Default)]
pub(crate) struct IntrinsicMemo(RefCell<SmallVec<[(IntrinsicKey, f32); ENTRIES_PER_NODE]>>);

impl LayoutForest {
    pub(crate) fn intrinsic_get(&self, id: LocalId, key: IntrinsicKey) -> Option<f32> {
        self.live_memo(id)?
            .0
            .borrow()
            .iter()
            .find(|(stored, _)| *stored == key)
            .map(|(_, extent)| *extent)
    }

    pub(crate) fn intrinsic_put(&self, id: LocalId, key: IntrinsicKey, extent: f32) {
        let Some(memo) = self.live_memo(id) else {
            return;
        };
        let mut entries = memo.0.borrow_mut();
        if let Some(slot) = entries.iter_mut().find(|(stored, _)| *stored == key) {
            slot.1 = extent;
            return;
        }
        if entries.len() == ENTRIES_PER_NODE {
            entries.remove(0);
        }
        entries.push((key, extent));
    }

    pub(crate) fn invalidate(&self, id: LocalId) {
        let mut current = Some(id);
        while let Some(at) = current {
            let Some(node) = self.node(at) else {
                return;
            };
            self.forget(at.index as usize);
            current = node.parent;
        }
    }

    pub(crate) fn clear_intrinsic(&self) {
        for memo in &self.intrinsic {
            memo.0.borrow_mut().clear();
        }
    }

    pub(super) fn forget(&self, index: usize) {
        if let Some(memo) = self.intrinsic.get(index) {
            memo.0.borrow_mut().clear();
        }
    }

    pub(super) fn adopt(&mut self, parent: LocalId, children: &[LocalId]) {
        for child in children {
            if let Ok(node) = self.live_mut(*child) {
                node.parent = Some(parent);
            }
        }
    }

    fn live_memo(&self, id: LocalId) -> Option<&IntrinsicMemo> {
        self.node(id)
            .and_then(|_| self.intrinsic.get(id.index as usize))
    }
}

impl LayoutText {
    pub(crate) fn same_source(&self, other: &Self) -> bool {
        self.wrap == other.wrap
            && self.ellipsis == other.ellipsis
            && self.style == other.style
            && match (&self.content, &other.content) {
                (TextContent::Static(a), TextContent::Static(b)) => a == b,
                (TextContent::Reactive(a), TextContent::Reactive(b)) => a.key == b.key,
                (TextContent::Compute(a), TextContent::Compute(b)) => Rc::ptr_eq(a, b),
                _ => false,
            }
    }
}
