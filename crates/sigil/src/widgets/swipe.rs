use crate::animation::{Animator, Property, Spring};
use crate::events::{DragPhase, PointerEvent};
use crate::geometry::{Point, Size};
use crate::kernel::NodeBuilder;
use crate::layout::Orientation;
use crate::reactive::{Signal, effect, signal};
use crate::widgets::{Side, Widget};

#[derive(Clone, Copy, Debug)]
pub(crate) struct Swipe {
    offset: Animator,
    leaving: Signal<bool>,
    size: Signal<Size>,
    axis: Orientation,
    outward: f32,
}

const fn heading(side: Side) -> (Orientation, f32) {
    match side {
        Side::Left => (Orientation::Horizontal, -1.0),
        Side::Right => (Orientation::Horizontal, 1.0),
        Side::Bottom => (Orientation::Vertical, 1.0),
        Side::Top => (Orientation::Vertical, -1.0),
    }
}

impl Swipe {
    pub(crate) fn new(toward: Side) -> Self {
        let (axis, outward) = heading(toward);
        Self {
            offset: Animator::spring(0.0, Spring::SNAPPY),
            leaving: signal(false),
            size: signal(Size::ZERO),
            axis,
            outward,
        }
    }

    pub(crate) fn reset(self) {
        self.leaving.set(false);
        self.offset.snap(0.0);
    }

    pub(crate) fn attach(self, node: NodeBuilder, away: impl Fn() + 'static) -> NodeBuilder {
        let (offset, leaving, outward) = (self.offset, self.leaving, self.outward);
        effect(move || {
            if leaving.get() && !offset.active().get() {
                leaving.set(false);
                away();
            }
        });
        let property: fn(f32) -> Property = match self.axis {
            Orientation::Horizontal => Property::TranslateX,
            Orientation::Vertical => Property::TranslateY,
        };
        node.track_size(self.size)
            .drive(property, move || outward * offset.get())
            .pointer_drag(move |phase, event| self.follow(phase, event))
    }

    fn follow(self, phase: DragPhase, event: &PointerEvent) {
        match phase {
            DragPhase::Move => {
                let moved = self.offset.peek() + self.along(event.delta());
                self.offset.snap(moved.clamp(0.0, self.travel()));
            }
            DragPhase::End => self.release(self.along(event.velocity())),
            DragPhase::Cancel => self.offset.target(0.0),
            _ => {}
        }
    }

    fn release(self, velocity: f32) {
        let landed = self.offset.fling_to(velocity, &[self.travel(), 0.0]);
        if landed.is_some_and(|point| point > 0.0) {
            self.leaving.set(true);
        }
    }

    fn along(self, point: Point) -> f32 {
        self.outward
            * match self.axis {
                Orientation::Horizontal => point.x,
                Orientation::Vertical => point.y,
            }
    }

    fn travel(self) -> f32 {
        let size = self.size.peek();
        match self.axis {
            Orientation::Horizontal => size.width,
            Orientation::Vertical => size.height,
        }
    }
}
