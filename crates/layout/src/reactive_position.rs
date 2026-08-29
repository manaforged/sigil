use sigil_kernel::NodeBuilder;
use sigil_reactive::Signal;

#[derive(Clone, Copy)]
pub struct ReactivePosition {
    pos: Signal<(f32, f32)>,
}

impl ReactivePosition {
    #[must_use]
    pub fn get(&self) -> (f32, f32) {
        self.pos.peek()
    }
}

#[derive(Clone, Copy)]
pub struct SizeReactive {
    size: Signal<(f32, f32)>,
}

impl SizeReactive {
    pub fn publish(&self, size: (f32, f32)) {
        if self.size.peek() != size {
            self.size.set(size);
        }
    }
}

#[derive(Clone, Copy)]
pub struct BoundsReactive {
    bounds: Signal<(f32, f32, f32, f32)>,
}

impl BoundsReactive {
    pub fn publish(&self, bounds: (f32, f32, f32, f32)) {
        if self.bounds.peek() != bounds {
            self.bounds.set(bounds);
        }
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
