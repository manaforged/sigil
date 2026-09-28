use sigil::hal::{LogicalKey, ModifiersState, Point};

use super::{Harness, Node};
use crate::input;

const DRAG_STEPS: u16 = 4;

pub trait Target {
    fn point(&self) -> Point;
}

impl Target for Point {
    fn point(&self) -> Point {
        *self
    }
}

impl Target for Node {
    #[track_caller]
    fn point(&self) -> Point {
        self.center().unwrap_or_else(|| {
            super::fail(format!(
                "node {:?} {:?} has no bounds",
                self.role(),
                self.name()
            ))
        })
    }
}

impl Harness {
    #[track_caller]
    pub fn click(&mut self, target: &impl Target) -> &mut Self {
        let at = target.point();
        self.send(input::click(at.x, at.y))
    }

    #[track_caller]
    pub fn hover(&mut self, target: &impl Target) -> &mut Self {
        let at = target.point();
        self.send([input::pointer_move(at.x, at.y)])
    }

    #[track_caller]
    pub fn scroll(&mut self, target: &impl Target, lines: f32) -> &mut Self {
        let at = target.point();
        self.send([
            input::pointer_move(at.x, at.y),
            input::scroll_lines(at.x, at.y, lines),
        ])
    }

    #[track_caller]
    pub fn drag(&mut self, from: &impl Target, to: &impl Target) -> &mut Self {
        let (from, to) = (from.point(), to.point());
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
    pub fn focus_next(&mut self) -> &mut Self {
        self.send([input::tab(false)])
    }

    #[track_caller]
    pub fn focus_previous(&mut self) -> &mut Self {
        self.send([input::tab(true)])
    }
}
