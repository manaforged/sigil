use std::rc::Rc;

use crate::kernel::{DeltaKind, NodeBuilder};
use crate::layout::shift::{Shift, ShiftSlot};
use crate::layout::snap;
use crate::reactive::Signal;

#[doc(hidden)]
#[derive(Clone, Debug)]
pub struct ReactivePosition {
    pos: Signal<(f32, f32)>,
    slot: Rc<ShiftSlot>,
}

impl ReactivePosition {
    #[must_use]
    pub fn get(&self) -> (f32, f32) {
        self.pos.try_peek().unwrap_or_default()
    }

    pub(crate) fn place(&self) {
        let (x, y) = self.get();
        self.slot.move_to((snap::offset(x), snap::offset(y)));
    }

    pub(crate) fn nest(&self, under: Shift) -> Shift {
        self.slot.nest(under)
    }
}

#[derive(Clone)]
pub(crate) enum SizeReactive {
    Signal(Signal<(f32, f32)>),
    Sink(Rc<dyn Fn((f32, f32)) -> bool>),
}

impl SizeReactive {
    pub(crate) fn publish(&self, size: (f32, f32)) -> bool {
        match self {
            Self::Signal(signal) => publish_changed(*signal, size),
            Self::Sink(sink) => sink(size),
        }
    }
}

fn publish_changed<T: Clone + PartialEq + 'static>(signal: Signal<T>, value: T) -> bool {
    let changed = signal.try_peek().is_some_and(|current| current != value);
    if changed {
        signal.set(value);
    }
    changed
}

#[doc(hidden)]
#[derive(Clone, Copy, Debug)]
pub struct BoundsReactive {
    bounds: Signal<(f32, f32, f32, f32)>,
}

impl BoundsReactive {
    pub(crate) fn publish(self, rect: crate::geometry::Rect) -> bool {
        publish_changed(
            self.bounds,
            (rect.origin.x, rect.origin.y, rect.width, rect.height),
        )
    }
}

pub(crate) fn position(node: NodeBuilder, pos: Signal<(f32, f32)>) -> NodeBuilder {
    node.mutate(move |node| {
        let _ = node.attach(ReactivePosition {
            pos,
            slot: Rc::new(ShiftSlot::new()),
        });
        let tag = node.tag();
        crate::reactive::effect_named("sigil-layout::position_delta", move || {
            let _ = pos.get();
            if let Some(tag) = tag {
                tag.record(DeltaKind::Layout);
            }
        });
    })
}

pub(crate) fn size(node: NodeBuilder, size: Signal<(f32, f32)>) -> NodeBuilder {
    node.mutate(move |node| {
        let _ = node.attach(SizeReactive::Signal(size));
    })
}

pub(crate) fn size_sink(
    node: NodeBuilder,
    sink: impl Fn((f32, f32)) -> bool + 'static,
) -> NodeBuilder {
    node.mutate(move |node| {
        let _ = node.attach(SizeReactive::Sink(Rc::new(sink)));
    })
}

pub(crate) fn bounds(node: NodeBuilder, bounds: Signal<(f32, f32, f32, f32)>) -> NodeBuilder {
    node.mutate(move |node| {
        let _ = node.attach(BoundsReactive { bounds });
    })
}
