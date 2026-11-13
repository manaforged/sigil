use std::rc::Rc;

use smallvec::SmallVec;

use crate::geometry::{Point, Rect, Size};
use crate::kernel::{DeltaKind, Node, NodeBuilder};
use crate::layout::shift::{Shift, ShiftSlot};
use crate::layout::snap;
use crate::reactive::Signal;

#[doc(hidden)]
#[derive(Clone, Debug)]
pub struct ReactivePosition {
    pos: Signal<Point>,
    slot: Rc<ShiftSlot>,
}

impl ReactivePosition {
    #[must_use]
    pub fn get(&self) -> (f32, f32) {
        let point = self.pos.try_peek().unwrap_or_default();
        (point.x, point.y)
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
enum SizeListener {
    Signal(Signal<Size>),
    Sink(Rc<dyn Fn((f32, f32)) -> bool>),
}

#[derive(Clone, Default)]
pub(crate) struct SizeReactive {
    listeners: SmallVec<[SizeListener; 1]>,
}

impl SizeReactive {
    pub(crate) fn publish(&self, size: Size) -> bool {
        let mut changed = false;
        for listener in &self.listeners {
            changed |= match listener {
                SizeListener::Signal(signal) => publish_changed(*signal, size),
                SizeListener::Sink(sink) => sink((size.width, size.height)),
            };
        }
        changed
    }
}

fn publish_changed<T: Clone + PartialEq + 'static>(signal: Signal<T>, value: T) -> bool {
    let changed = signal
        .try_peek_with(|current| current != &value)
        .unwrap_or(false);
    if changed {
        signal.set(value);
    }
    changed
}

#[doc(hidden)]
#[derive(Clone, Debug, Default)]
pub struct BoundsReactive {
    listeners: SmallVec<[Signal<Rect>; 1]>,
}

impl BoundsReactive {
    pub(crate) fn publish(&self, rect: Rect) -> bool {
        let mut changed = false;
        for signal in &self.listeners {
            changed |= publish_changed(*signal, rect);
        }
        changed
    }
}

#[derive(Clone, Debug, Default)]
pub(crate) struct PositionReactive {
    listeners: SmallVec<[Signal<Point>; 1]>,
}

impl PositionReactive {
    pub(crate) fn publish(&self, rect: Rect) -> bool {
        let mut changed = false;
        for signal in &self.listeners {
            changed |= publish_changed(*signal, rect.origin);
        }
        changed
    }
}

pub(crate) fn offset(node: NodeBuilder, pos: Signal<Point>) -> NodeBuilder {
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

pub(crate) fn position(node: NodeBuilder, position: Signal<Point>) -> NodeBuilder {
    node.mutate(move |node| {
        ext_mut::<PositionReactive>(node).listeners.push(position);
    })
}

pub(crate) fn size(node: NodeBuilder, size: Signal<Size>) -> NodeBuilder {
    node.mutate(move |node| {
        ext_mut::<SizeReactive>(node)
            .listeners
            .push(SizeListener::Signal(size));
    })
}

pub(crate) fn size_sink(
    node: NodeBuilder,
    sink: impl Fn((f32, f32)) -> bool + 'static,
) -> NodeBuilder {
    let sink: Rc<dyn Fn((f32, f32)) -> bool> = Rc::new(sink);
    node.mutate(move |node| {
        ext_mut::<SizeReactive>(node)
            .listeners
            .push(SizeListener::Sink(sink));
    })
}

pub(crate) fn bounds(node: NodeBuilder, bounds: Signal<Rect>) -> NodeBuilder {
    node.mutate(move |node| {
        ext_mut::<BoundsReactive>(node).listeners.push(bounds);
    })
}

fn ext_mut<E: Default + 'static>(node: &mut Node) -> &mut E {
    if node.extension::<E>().is_none() {
        let _ = node.attach(E::default());
    }
    node.extension_mut::<E>().unwrap_or_else(|| unreachable!())
}
