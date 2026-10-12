use crate::events::KeyEvent;
use crate::geometry::Point;
use crate::hal::{HitPosition, TextAffinity};
use crate::layout::with_text_geometry;

use super::super::TextEdit;
use super::TextEditRuntime;
use super::keymap::LineStep;

pub(super) fn move_horizontal(
    event: &mut KeyEvent,
    runtime: &TextEditRuntime,
    right: bool,
    extend: bool,
) {
    event.stop_propagation();
    if runtime.disabled() {
        return;
    }
    let mut state = runtime.edit_mut();
    let position = runtime.caret_at(&state);
    if (extend || state.selection().is_none())
        && let Some(target) = wrap_transition(state.as_str(), runtime, position, right)
    {
        runtime.jump(&mut state, position.affinity, target, extend);
        return;
    }
    let before = state.caret();
    if extend && state.selection().is_none() {
        runtime.anchor_affinity.set(position.affinity);
    }
    state.move_by(right, extend);
    let affinity = if state.caret() == before {
        position.affinity
    } else if right {
        TextAffinity::Upstream
    } else {
        TextAffinity::Downstream
    };
    runtime.commit(&state);
    runtime.caret_affinity.set(affinity);
}

fn wrap_transition(
    buffer: &str,
    runtime: &TextEditRuntime,
    position: HitPosition,
    right: bool,
) -> Option<HitPosition> {
    let style = runtime.text_style.peek();
    let width = runtime.geometry_width();
    let opposite = HitPosition {
        byte_index: position.byte_index,
        affinity: match position.affinity {
            TextAffinity::Downstream => TextAffinity::Upstream,
            TextAffinity::Upstream => TextAffinity::Downstream,
        },
    };
    let wrapped = with_text_geometry(|engine| {
        let current = engine.caret(buffer, &style, width, position);
        let other = engine.caret(buffer, &style, width, opposite);
        (current.top - other.top).abs() > f32::EPSILON
    })?;
    let toward = if right {
        TextAffinity::Upstream
    } else {
        TextAffinity::Downstream
    };
    (wrapped && position.affinity == toward).then_some(opposite)
}

pub(super) fn move_vertical(
    event: &mut KeyEvent,
    runtime: &TextEditRuntime,
    step: LineStep,
    extend: bool,
) {
    if runtime.disabled() {
        event.stop_propagation();
        return;
    }
    let mut state = runtime.edit_mut();
    let buffer = state.as_str().to_owned();
    let style = runtime.text_style.peek();
    let width = runtime.geometry_width();
    let position = runtime.caret_at(&state);
    let x = runtime.preferred_x.get().unwrap_or_else(|| {
        with_text_geometry(|engine| engine.caret(&buffer, &style, width, position).x).unwrap_or(0.0)
    });
    let (target, moved) = with_text_geometry(|engine| {
        let current = engine.caret(&buffer, &style, width, position);
        let height = current.height.max(1.0);
        #[allow(
            clippy::cast_precision_loss,
            reason = "vertical movement counts are small relative to layout coordinates"
        )]
        let rows = step.rows(runtime.viewport_size.peek().1, height) as f32;
        let target = engine.hit(
            &buffer,
            &style,
            width,
            Point::new(x, current.top + height.mul_add(0.5, rows * height)),
        );
        let moved = target.byte_index != position.byte_index
            || (engine.caret(&buffer, &style, width, target).top - current.top).abs()
                > f32::EPSILON;
        (target, moved)
    })
    .unwrap_or((position, false));
    if !extend && !moved {
        return;
    }
    event.stop_propagation();
    runtime.jump(&mut state, position.affinity, target, extend);
    runtime.preferred_x.set(Some(x));
}

pub(super) fn move_visual_edge(
    event: &mut KeyEvent,
    runtime: &TextEditRuntime,
    end: bool,
    extend: bool,
) {
    event.stop_propagation();
    if runtime.disabled() {
        return;
    }
    let mut state = runtime.edit_mut();
    let buffer = state.as_str().to_owned();
    let style = runtime.text_style.peek();
    let width = runtime.geometry_width();
    let position = runtime.caret_at(&state);
    let target = with_text_geometry(|engine| {
        let current = engine.caret(&buffer, &style, width, position);
        let x = if end { width.unwrap_or(f32::MAX) } else { 0.0 };
        engine.hit(
            &buffer,
            &style,
            width,
            Point::new(x, current.height.max(1.0).mul_add(0.5, current.top)),
        )
    })
    .unwrap_or(position);
    runtime.jump(&mut state, position.affinity, target, extend);
}

impl TextEditRuntime {
    fn caret_at(&self, state: &TextEdit) -> HitPosition {
        HitPosition {
            byte_index: state.caret(),
            affinity: self.caret_affinity.peek(),
        }
    }

    fn jump(&self, state: &mut TextEdit, from: TextAffinity, target: HitPosition, extend: bool) {
        if extend && state.selection().is_none() {
            self.anchor_affinity.set(from);
        }
        state.jump_to(target.byte_index, extend);
        self.commit(state);
        self.caret_affinity.set(target.affinity);
    }

    pub(in crate::widgets::text_edit) fn geometry_width(&self) -> Option<f32> {
        let width = self.content_size.get().0;
        (width > 0.0).then_some(width)
    }

    pub(super) fn update_scroll(&self, edit: &TextEdit, column: f32) {
        let style = self.text_style.peek();
        let Some(caret_x) = with_text_geometry(|engine| {
            engine
                .caret(
                    edit.as_str(),
                    &style,
                    None,
                    HitPosition {
                        byte_index: edit.caret(),
                        affinity: TextAffinity::Downstream,
                    },
                )
                .x
        }) else {
            return;
        };
        let current = self.scroll_x.peek();
        let offset = if caret_x < current {
            caret_x
        } else if caret_x > current + column {
            caret_x - column
        } else {
            current
        }
        .max(0.0);
        if (offset - current).abs() > f32::EPSILON {
            self.scroll_x.set(offset);
        }
    }
}
