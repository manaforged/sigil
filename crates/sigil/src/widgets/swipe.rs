use crate::animation::{Animator, Decay, Property, Spring};
use crate::events::{DragPhase, PointerEvent};
use crate::geometry::{Point, Size};
use crate::kernel::NodeBuilder;
use crate::layout::Orientation;
use crate::reactive::{Signal, effect, signal};
use crate::widgets::{Side, Widget};

const AWAY_SHARE: f32 = 0.5;

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
                let moved = self.now() + self.along(event.delta());
                self.offset.snap(moved.clamp(0.0, self.travel()));
            }
            DragPhase::End => self.release(self.along(event.velocity())),
            DragPhase::Cancel => self.offset.target(0.0),
            _ => {}
        }
    }

    fn release(self, velocity: f32) {
        let travel = self.travel();
        let rest = Decay::new(self.now(), velocity).target();
        if travel > 0.0 && rest >= travel * AWAY_SHARE {
            self.offset.fling_to(velocity, &[travel]);
            self.leaving.set(true);
        } else {
            self.offset.fling_to(velocity, &[0.0]);
        }
    }

    fn now(self) -> f32 {
        self.offset.value().peek()
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
