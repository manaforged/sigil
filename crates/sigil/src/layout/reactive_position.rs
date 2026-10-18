use crate::kernel::{DeltaKind, NodeBuilder};
use crate::reactive::Signal;

#[derive(Clone, Copy)]
pub struct ReactivePosition {
    pos: Signal<(f32, f32)>,
}

impl ReactivePosition {
    #[must_use]
    pub fn get(&self) -> (f32, f32) {
        self.pos.try_peek().unwrap_or_default()
    }
}

#[derive(Clone, Copy)]
pub(crate) struct SizeReactive {
    size: Signal<(f32, f32)>,
}

impl SizeReactive {
    pub(crate) fn publish(self, size: (f32, f32)) -> bool {
        publish_changed(self.size, size)
    }
}

fn publish_changed<T: Clone + PartialEq + 'static>(signal: Signal<T>, value: T) -> bool {
    let changed = signal.try_peek().is_some_and(|current| current != value);
    if changed {
        signal.set(value);
    }
    changed
}

#[derive(Clone, Copy)]
pub struct BoundsReactive {
    bounds: Signal<(f32, f32, f32, f32)>,
}

impl BoundsReactive {
    pub fn publish(&self, bounds: (f32, f32, f32, f32)) -> bool {
        publish_changed(self.bounds, bounds)
    }
}

pub trait LayoutReactiveExt {
    #[must_use]
    fn position_reactive(self, pos: Signal<(f32, f32)>) -> Self;

    #[must_use]
    fn size_reactive(self, size: Signal<(f32, f32)>) -> Self;

    #[must_use]
    fn bounds_reactive(self, bounds: Signal<(f32, f32, f32, f32)>) -> Self;
}

impl LayoutReactiveExt for NodeBuilder {
    fn position_reactive(self, pos: Signal<(f32, f32)>) -> Self {
        self.mutate(move |node| {
            let _ = node.attach(ReactivePosition { pos });
            let tag = node.tag();
            crate::reactive::effect_named("sigil-layout::position_delta", move || {
                let _ = pos.get();
                if let Some(tag) = tag {
                    tag.record(DeltaKind::Layout);
                }
            });
        })
    }

    fn size_reactive(self, size: Signal<(f32, f32)>) -> Self {
        self.mutate(move |node| {
            let _ = node.attach(SizeReactive { size });
        })
    }

    fn bounds_reactive(self, bounds: Signal<(f32, f32, f32, f32)>) -> Self {
        self.mutate(move |node| {
            let _ = node.attach(BoundsReactive { bounds });
        })
    }
}
