use crate::kernel::NodeBuilder;
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
        let changed = self.size.try_peek().is_some_and(|current| current != size);
        if changed {
            self.size.set(size);
        }
        changed
    }
}

#[derive(Clone, Copy)]
pub struct BoundsReactive {
    bounds: Signal<(f32, f32, f32, f32)>,
}

impl BoundsReactive {
    pub fn publish(&self, bounds: (f32, f32, f32, f32)) -> bool {
        let changed = self
            .bounds
            .try_peek()
            .is_some_and(|current| current != bounds);
        if changed {
            self.bounds.set(bounds);
        }
        changed
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
