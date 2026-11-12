use sigil::events::Event;
use sigil::events::{Key, Modifiers};
use sigil::geometry::Point;

use super::{Harness, Node};

const DRAG_STEPS: u16 = 4;

mod sealed {
    #[allow(unnameable_types, reason = "sealed")]
    pub trait Sealed {}
}

/// A place a pointer step acts on: a point, a node, or the accessible name of a control.
pub trait Target: sealed::Sealed {
    /// The point in logical pixels where a pointer step on this target acts.
    fn point(&self, ui: &mut Harness) -> Point;
}

impl sealed::Sealed for Point {}

impl sealed::Sealed for Node {}

impl sealed::Sealed for str {}

impl Target for Point {
    fn point(&self, _: &mut Harness) -> Point {
        *self
    }
}

impl Target for Node {
    #[track_caller]
    fn point(&self, _: &mut Harness) -> Point {
        self.center().unwrap_or_else(|| {
            super::fail(format!(
                "node {:?} {:?} has no bounds",
                self.role(),
                self.name()
            ))
        })
    }
}

impl Target for str {
    #[track_caller]
    fn point(&self, ui: &mut Harness) -> Point {
        let what = format!("control named {self:?}");
        let node = ui.wait_for(&what, |ui| ui.labelled(self));
        node.point(ui)
    }
}

impl Harness {
    #[track_caller]
    pub fn click(&mut self, target: &(impl Target + ?Sized)) -> &mut Self {
        let at = target.point(self);
        self.send(Event::click(Point::new(at.x, at.y)))
    }

    #[track_caller]
    pub fn hover(&mut self, target: &(impl Target + ?Sized)) -> &mut Self {
        let at = target.point(self);
        self.send([Event::pointer_move(Point::new(at.x, at.y))])
    }

    #[track_caller]
    pub fn scroll(&mut self, target: &(impl Target + ?Sized), lines: f32) -> &mut Self {
        let at = target.point(self);
        self.send([
            Event::pointer_move(Point::new(at.x, at.y)),
            Event::wheel(Point::new(at.x, at.y), lines),
        ])
    }

    #[track_caller]
    pub fn drag(
        &mut self,
        from: &(impl Target + ?Sized),
        to: &(impl Target + ?Sized),
    ) -> &mut Self {
        let (from, to) = (from.point(self), to.point(self));
        let mut events = vec![
            Event::pointer_move(Point::new(from.x, from.y)),
            Event::pointer_down(Point::new(from.x, from.y)),
        ];
        let steps = f32::from(DRAG_STEPS);
        events.extend((1..=DRAG_STEPS).map(|step| {
            let t = f32::from(step) / steps;
            Event::pointer_drag(Point::new(
                (to.x - from.x).mul_add(t, from.x),
                (to.y - from.y).mul_add(t, from.y),
            ))
        }));
        events.push(Event::pointer_up(Point::new(to.x, to.y)));
        self.send(events)
    }

    #[track_caller]
    pub fn press(&mut self, key: impl Into<Key>) -> &mut Self {
        self.press_with(key, Modifiers::empty())
    }

    #[track_caller]
    pub fn press_with(&mut self, key: impl Into<Key>, modifiers: Modifiers) -> &mut Self {
        self.send(Event::press(key, modifiers))
    }

    #[track_caller]
    pub fn type_text(&mut self, text: &str) -> &mut Self {
        self.send(Event::type_text(text))
    }

    #[track_caller]
    pub fn type_into(&mut self, label: &str, text: &str) -> &mut Self {
        self.click(label).type_text(text)
    }

    #[track_caller]
    pub fn focus_next(&mut self) -> &mut Self {
        self.send([Event::tab(false)])
    }

    #[track_caller]
    pub fn focus_previous(&mut self) -> &mut Self {
        self.send([Event::tab(true)])
    }
}
