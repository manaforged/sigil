use sigil::events::{LogicalKey, ModifiersState};
use sigil::geometry::Point;

use super::{Harness, Node};
use crate::input;

const DRAG_STEPS: u16 = 4;

pub trait Target {
    fn point(&self, ui: &mut Harness) -> Point;
}

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
        self.send(input::click(at.x, at.y))
    }

    #[track_caller]
    pub fn hover(&mut self, target: &(impl Target + ?Sized)) -> &mut Self {
        let at = target.point(self);
        self.send([input::pointer_move(at.x, at.y)])
    }

    #[track_caller]
    pub fn scroll(&mut self, target: &(impl Target + ?Sized), lines: f32) -> &mut Self {
        let at = target.point(self);
        self.send([
            input::pointer_move(at.x, at.y),
            input::scroll_lines(at.x, at.y, lines),
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
            input::pointer_move(from.x, from.y),
            input::pointer_down(from.x, from.y),
        ];
        let steps = f32::from(DRAG_STEPS);
        events.extend((1..=DRAG_STEPS).map(|step| {
            let t = f32::from(step) / steps;
            input::pointer_drag(
                (to.x - from.x).mul_add(t, from.x),
                (to.y - from.y).mul_add(t, from.y),
            )
        }));
        events.push(input::pointer_up(to.x, to.y));
        self.send(events)
    }

    #[track_caller]
    pub fn press(&mut self, key: impl Into<LogicalKey>) -> &mut Self {
        self.press_with(key, ModifiersState::empty())
    }

    #[track_caller]
    pub fn press_with(
        &mut self,
        key: impl Into<LogicalKey>,
        modifiers: ModifiersState,
    ) -> &mut Self {
        self.send(input::press(key, modifiers))
    }

    #[track_caller]
    pub fn type_text(&mut self, text: &str) -> &mut Self {
        self.send(input::type_text(text))
    }

    #[track_caller]
    pub fn type_into(&mut self, label: &str, text: &str) -> &mut Self {
        self.click(label).type_text(text)
    }

    #[track_caller]
    pub fn focus_next(&mut self) -> &mut Self {
        self.send([input::tab(false)])
    }

    #[track_caller]
    pub fn focus_previous(&mut self) -> &mut Self {
        self.send([input::tab(true)])
    }
}
