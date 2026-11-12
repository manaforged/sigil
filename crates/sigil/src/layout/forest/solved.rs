use std::cell::Cell;

use smallvec::SmallVec;

use super::intrinsic::constraint_bits;
use super::{AvailableSpace, ComputedBox, LayoutForest, LocalId};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct SolvedKey {
    w: u32,
    h: u32,
    assigned_w: Option<u32>,
    assigned_h: Option<u32>,
    available: (u64, u64),
    fonts: u64,
    cell: Option<u32>,
}

impl SolvedKey {
    pub(crate) fn new(
        (w, h): (f32, f32),
        (assigned_w, assigned_h): (Option<f32>, Option<f32>),
        available: AvailableSpace,
        fonts: u64,
    ) -> Self {
        Self {
            w: w.to_bits(),
            h: h.to_bits(),
            assigned_w: assigned_w.map(f32::to_bits),
            assigned_h: assigned_h.map(f32::to_bits),
            available: (constraint_bits(available.w), constraint_bits(available.h)),
            fonts,
            cell: crate::layout::snap::cell().map(f32::to_bits),
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub(super) struct SolvedSubtree {
    key: SolvedKey,
    origin: (f32, f32),
    root_box: ComputedBox,
    frame: u32,
}

const fn shift_box(cb: ComputedBox, dx: f32, dy: f32) -> ComputedBox {
    ComputedBox {
        x: cb.x + dx,
        y: cb.y + dy,
        w: cb.w,
        h: cb.h,
    }
}

const TRAIT_MASK: u8 = 0b11;
const FILLS_WIDTH: u8 = 0;
const HEIGHT_FREE: u8 = 2;

impl LayoutForest {
    pub(crate) fn fills_get(&self, id: LocalId) -> Option<bool> {
        self.trait_get(id, FILLS_WIDTH)
    }

    pub(crate) fn fills_put(&self, id: LocalId, fills: bool) {
        self.trait_put(id, FILLS_WIDTH, fills);
    }

    pub(crate) fn height_free_get(&self, id: LocalId) -> Option<bool> {
        self.trait_get(id, HEIGHT_FREE)
    }

    pub(crate) fn height_free_put(&self, id: LocalId, free: bool) {
        self.trait_put(id, HEIGHT_FREE, free);
    }

    fn trait_get(&self, id: LocalId, shift: u8) -> Option<bool> {
        let code = self
            .node(id)
            .and_then(|_| self.fills.get(id.index as usize))?
            .get();
        match (code >> shift) & TRAIT_MASK {
            1 => Some(false),
            2 => Some(true),
            _ => None,
        }
    }

    fn trait_put(&self, id: LocalId, shift: u8, value: bool) {
        if let Some(slot) = self
            .node(id)
            .and_then(|_| self.fills.get(id.index as usize))
        {
            let code = if value { 2 } else { 1 };
            slot.set((slot.get() & !(TRAIT_MASK << shift)) | (code << shift));
        }
    }

    pub(crate) fn store_solved(&self, id: LocalId, key: SolvedKey, origin: (f32, f32)) {
        let Some(root_box) = self.boxed(id) else {
            return;
        };
        if let Some(slot) = self.live_solved(id) {
            slot.set(Some(SolvedSubtree {
                key,
                origin,
                root_box,
                frame: self.frame,
            }));
        }
    }

    pub(crate) fn replay_subtree(&self, id: LocalId, key: SolvedKey, origin: (f32, f32)) -> bool {
        let Some(solved) = self.live_solved(id).and_then(Cell::get) else {
            return false;
        };
        if solved.key != key || solved.frame == 0 {
            return false;
        }
        let Some(descendants) = self.collect_clean_descendants(id, solved.frame) else {
            return false;
        };
        self.apply_replayed_subtree(id, solved, origin, &descendants);
        true
    }

    pub(crate) fn forget_solved_subtree(&self, root: LocalId) {
        let mut stack = SmallVec::<[LocalId; 32]>::new();
        stack.push(root);
        while let Some(at) = stack.pop() {
            let Some(node) = self.node(at) else {
                continue;
            };
            if let Some(slot) = self.solved.get(at.index as usize) {
                slot.set(None);
            }
            stack.extend_from_slice(&node.children);
        }
    }

    pub(crate) fn invalidate_solved_ancestors(&self, id: LocalId) {
        let mut current = self.node(id).and_then(|node| node.parent);
        while let Some(at) = current {
            let Some(node) = self.node(at) else {
                return;
            };
            if let Some(slot) = self.solved.get(at.index as usize) {
                slot.set(None);
            }
            current = node.parent;
        }
    }

    pub(crate) fn shift_solved_from(&self, id: LocalId, min_frame: u32, (dx, dy): (f32, f32)) {
        let Some(slot) = self.live_solved(id) else {
            return;
        };
        let Some(sub) = slot.get() else {
            return;
        };
        if sub.frame >= min_frame {
            slot.set(Some(SolvedSubtree {
                origin: (sub.origin.0 + dx, sub.origin.1 + dy),
                root_box: shift_box(sub.root_box, dx, dy),
                frame: self.frame,
                ..sub
            }));
        } else {
            slot.set(None);
        }
    }

    pub(super) fn clear_solved(&self) {
        for slot in &self.solved {
            slot.set(None);
        }
    }

    fn live_solved(&self, id: LocalId) -> Option<&Cell<Option<SolvedSubtree>>> {
        self.node(id)
            .and_then(|_| self.solved.get(id.index as usize))
    }

    fn collect_clean_descendants(
        &self,
        root: LocalId,
        frame: u32,
    ) -> Option<SmallVec<[LocalId; 32]>> {
        let root_node = self.node(root)?;
        let mut out = SmallVec::<[LocalId; 32]>::new();
        let mut stack = SmallVec::<[LocalId; 32]>::from_slice(&root_node.children);
        while let Some(at) = stack.pop() {
            let node = self.node(at)?;
            let box_frame = self.boxes.get(at.index as usize)?.frame();
            if box_frame > frame {
                return None;
            }
            out.push(at);
            stack.extend_from_slice(&node.children);
        }
        Some(out)
    }

    fn apply_replayed_subtree(
        &self,
        root: LocalId,
        solved: SolvedSubtree,
        origin: (f32, f32),
        descendants: &[LocalId],
    ) {
        let (dx, dy) = (origin.0 - solved.origin.0, origin.1 - solved.origin.1);
        let now = self.frame;
        let root_box = shift_box(solved.root_box, dx, dy);
        if let Some(cell) = self.boxes.get(root.index as usize) {
            cell.write(root_box, now);
        }
        if let Some(slot) = self.solved.get(root.index as usize) {
            slot.set(Some(SolvedSubtree {
                origin,
                root_box,
                frame: now,
                ..solved
            }));
        }
        for child in descendants {
            self.shift_replayed_node(*child, solved.frame, now, (dx, dy));
        }
    }

    fn shift_replayed_node(&self, id: LocalId, prev_frame: u32, now: u32, (dx, dy): (f32, f32)) {
        let idx = id.index as usize;
        if let Some(cell) = self.boxes.get(idx)
            && cell.frame() == prev_frame
        {
            if dx != 0.0 || dy != 0.0 {
                cell.write(shift_box(cell.get(), dx, dy), now);
            } else {
                cell.stamp(now);
            }
        }
        if let Some(slot) = self.solved.get(idx)
            && let Some(sub) = slot.get()
            && sub.frame == prev_frame
        {
            slot.set(Some(SolvedSubtree {
                origin: (sub.origin.0 + dx, sub.origin.1 + dy),
                root_box: shift_box(sub.root_box, dx, dy),
                frame: now,
                ..sub
            }));
        }
    }
}
