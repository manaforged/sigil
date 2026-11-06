use std::cell::Cell;
use std::rc::Rc;

use crate::app::Session;
use crate::app::pty::{Modes, Mouse, Row};
use crate::events::{MouseButton, PointerEvent, ScrollDelta, ScrollEvent};
use crate::kernel::{NodeBuilder, container, text};
use crate::layout::{mono_cell, rows};
use crate::reactive::{effect_named, signal};
use crate::style::theme::ambient::ActiveTheme;
use crate::widgets::Widget;
use crate::widgets::virtual_list::virtual_list;

use super::{key_payload, shell};

impl Session {
    /// Builds a view of this terminal that fills its space, sizes the terminal to it, and takes keys, paste, the mouse and the wheel.
    #[must_use]
    pub fn view(self: &Rc<Self>) -> NodeBuilder {
        let list = virtual_list(self.rows, |_i, row: &Row| {
            container().child(text(row.text.clone()).runs(row.runs.clone()))
        })
        .row_height(rows(1.0))
        .follow_end(true)
        .keyboard(false);
        let size = signal((0.0_f32, 0.0_f32));
        self.follow(size);
        let (keys, pasted) = (Rc::clone(self), Rc::clone(self));
        let node = shell(move |event| keys.key(event))
            .child(list)
            .selectable_text(true)
            .size_reactive(size)
            .on_paste(move |text| pasted.paste(text).is_ok());
        pointer(wheel(node, self), self)
    }

    fn follow(self: &Rc<Self>, size: crate::reactive::Signal<(f32, f32)>) {
        let sized = Rc::clone(self);
        effect_named("sigil::terminal::size", move || {
            let (width, height) = size.get();
            let (cell_width, cell_height) = mono_cell();
            if width > 0.0 && height > 0.0 {
                drop(sized.resize(cells(width, cell_width), cells(height, cell_height)));
            }
        });
        let tinted = Rc::clone(self);
        let theme = ActiveTheme::capture();
        effect_named("sigil::terminal::colors", move || {
            let (ink, background) = theme.with(|theme| (theme.on_surface, theme.surface));
            tinted.set_colors(ink, background);
        });
    }

    fn key(&self, event: &crate::events::KeyEvent) -> bool {
        let application = self.modes().is_some_and(|modes| modes.cursor_keys);
        let Some(payload) = key_payload(event, application) else {
            return false;
        };
        drop(self.scroll(i32::MIN));
        self.write(payload.as_bytes()).is_ok()
    }
}

fn cells(span: f32, cell: f32) -> u16 {
    (span / cell.max(1.0)).floor().clamp(1.0, 999.0) as u16
}

fn cell_at(x: f32, y: f32) -> (u16, u16) {
    let (width, height) = mono_cell();
    let cell = |at: f32, size: f32| (at.max(0.0) / size.max(1.0)).floor().min(998.0) as u16;
    (cell(x, width), cell(y, height))
}

fn wheel(node: NodeBuilder, session: &Rc<Session>) -> NodeBuilder {
    let session = Rc::clone(session);
    let left = Cell::new(0.0_f32);
    node.on_scroll(move |event| {
        let count = whole_rows(event, &left);
        if count != 0 {
            scroll(&session, event, count);
            event.stop_propagation();
        }
    })
}

fn whole_rows(event: &ScrollEvent, left: &Cell<f32>) -> i32 {
    let delta = match event.delta {
        ScrollDelta::Lines(point) => point.y,
        ScrollDelta::Pixels(point) => point.y / mono_cell().1.max(1.0),
    };
    let total = left.get() + delta;
    let whole = total.trunc();
    left.set(total - whole);
    whole as i32
}

fn scroll(session: &Session, event: &ScrollEvent, count: i32) {
    let Some(modes) = session.modes() else {
        return;
    };
    let up = count > 0;
    let at = cell_at(event.position.x, event.position.y);
    let step = Mouse::Wheel { up };
    if session.report(step, at, event.modifiers, modes) {
        for _ in 1..count.unsigned_abs() {
            session.report(step, at, event.modifiers, modes);
        }
    } else if modes.alternate {
        let key = match (up, modes.cursor_keys) {
            (true, true) => "\u{1b}OA",
            (true, false) => "\u{1b}[A",
            (false, true) => "\u{1b}OB",
            (false, false) => "\u{1b}[B",
        };
        drop(session.write(key.repeat(count.unsigned_abs() as usize).as_bytes()));
    } else {
        drop(session.scroll(count));
    }
}

fn pointer(node: NodeBuilder, session: &Rc<Session>) -> NodeBuilder {
    let (down, up, moved) = (Rc::clone(session), Rc::clone(session), Rc::clone(session));
    let held = Rc::new(Cell::new(None::<MouseButton>));
    let last = Rc::new(Cell::new((u16::MAX, u16::MAX)));
    let (pressed, released, dragged) = (Rc::clone(&held), Rc::clone(&held), held);
    node.pointer_down(move |event| {
        let Some(button) = event.button else {
            return;
        };
        if send(&down, Mouse::Down(button), event) {
            pressed.set(Some(button));
            crate::events::selection::clear();
        }
    })
    .pointer_up(move |event| {
        if let Some(button) = event.button {
            send(&up, Mouse::Up(button), event);
        }
        released.set(None);
    })
    .pointer_move(move |event| {
        let at = cell_at(event.position.x, event.position.y);
        if last.replace(at) != at {
            send(&moved, dragged.get().map_or(Mouse::Move, Mouse::Drag), event);
        }
    })
}

fn send(session: &Session, mouse: Mouse, event: &PointerEvent) -> bool {
    let at = cell_at(event.position.x, event.position.y);
    session
        .modes()
        .is_some_and(|modes: Modes| session.report(mouse, at, event.modifiers, modes))
}
