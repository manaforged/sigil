use std::cell::RefCell;

use smallvec::SmallVec;

use crate::animation::runtime::registry::{glide_config, glides};
use crate::animation::runtime::style::record_on;
use crate::animation::runtime::{AnimationConfig, SignalAnimator};
use crate::kernel::{Cx, DeltaKind, Node, NodeId, NodeTag};
use crate::reactive::{Signal, batch, owner_by_id, signal};
use crate::style::{StyleAtom, StyleBinding};

pub(crate) struct Glide {
    offset: [Signal<f32>; 2],
    animators: RefCell<[SignalAnimator; 2]>,
}

impl Glide {
    fn new(tag: Option<NodeTag>, config: AnimationConfig) -> Self {
        let offset = [signal(0.0_f32), signal(0.0_f32)];
        record_on(tag, &offset, DeltaKind::Paint);
        Self {
            offset,
            animators: RefCell::new(offset.map(|value| SignalAnimator::new(value, config))),
        }
    }

    pub(crate) fn offset(&self) -> Option<[f32; 2]> {
        let offset = self
            .offset
            .map(|value| value.try_peek().unwrap_or_default());
        (offset != [0.0; 2]).then_some(offset)
    }

    fn shift(&self, by: [f32; 2], config: AnimationConfig) {
        batch(|| {
            let mut animators = self.animators.borrow_mut();
            for ((animator, value), by) in animators.iter_mut().zip(self.offset).zip(by) {
                if by.abs() <= f32::EPSILON {
                    continue;
                }
                animator.set_config(config);
                animator.snap(value.try_peek().unwrap_or_default() + by);
                animator.target(0.0);
            }
        });
    }
}

pub(crate) fn opted_in(node: &Node) -> bool {
    node.extension::<StyleBinding>()
        .filter(|binding| binding.has_transition())
        .is_some_and(|binding| glides(binding.cascade()))
}

pub(crate) fn glide(cx: &Cx, id: NodeId, by: [f32; 2]) {
    let Some(node) = cx.get(id) else {
        return;
    };
    let Some(binding) = node.extension::<StyleBinding>() else {
        return;
    };
    let atoms: SmallVec<[StyleAtom; 32]> = binding.cascade().collect();
    let Some(config) = glide_config(&atoms) else {
        return;
    };
    let cell = binding.glide();
    if cell.get().is_none() {
        owner_by_id(node.owner, || {
            let _ = cell.set(Box::new(Glide::new(node.tag(), config)));
        });
    }
    if let Some(glide) = cell.get() {
        glide.shift(by, config);
    }
}
