use crate::events::KeyEvent;
use crate::geometry::Point;
use crate::hal::{HitPosition, TextAffinity};
use crate::layout::with_text_geometry;

use super::super::TextEdit;
use super::keymap::LineStep;
use super::{TextEditRuntime, commit};

pub(in crate::widgets::text_edit) fn geometry_width(runtime: &TextEditRuntime) -> Option<f32> {
    let width = runtime.content_size.get().0;
    (width > 0.0).then_some(width)
}

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
    let position = HitPosition {
        byte_index: state.caret(),
        affinity: runtime.caret_affinity.peek(),
    };
    if (extend || state.selection().is_none())
        && let Some(target) = wrap_transition(state.as_str(), runtime, position, right)
    {
        if extend {
            if state.selection().is_none() {
                runtime.anchor_affinity.set(position.affinity);
            }
            state.extend_to(target.byte_index);
        } else {
            state.place_caret(target.byte_index);
        }
        commit(runtime, &state);
        runtime.caret_affinity.set(target.affinity);
        return;
    }
    let before = state.caret();
    if extend {
        if state.selection().is_none() {
            runtime.anchor_affinity.set(position.affinity);
        }
        if right {
            state.shift_right();
        } else {
            state.shift_left();
        }
    } else if right {
        state.move_right();
    } else {
        state.move_left();
    }
    let affinity = if state.caret() == before {
        position.affinity
    } else if right {
        TextAffinity::Upstream
    } else {
        TextAffinity::Downstream
    };
    commit(runtime, &state);
    runtime.caret_affinity.set(affinity);
}

fn wrap_transition(
    buffer: &str,
    runtime: &TextEditRuntime,
    position: HitPosition,
    right: bool,
) -> Option<HitPosition> {
    let style = runtime.text_style.peek();
    let width = geometry_width(runtime);
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
    let moving_toward_other = (right && position.affinity == TextAffinity::Upstream)
        || (!right && position.affinity == TextAffinity::Downstream);
    wrapped.then_some(opposite).filter(|_| moving_toward_other)
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
    let width = geometry_width(runtime);
    let position = HitPosition {
        byte_index: state.caret(),
        affinity: runtime.caret_affinity.peek(),
    };
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
    if extend {
        if state.selection().is_none() {
            runtime.anchor_affinity.set(position.affinity);
        }
        state.extend_to(target.byte_index);
    } else {
        state.place_caret(target.byte_index);
    }
    commit(runtime, &state);
    runtime.caret_affinity.set(target.affinity);
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
    let width = geometry_width(runtime);
    let position = HitPosition {
        byte_index: state.caret(),
        affinity: runtime.caret_affinity.peek(),
    };
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
    if extend {
        if state.selection().is_none() {
            runtime.anchor_affinity.set(position.affinity);
        }
        state.extend_to(target.byte_index);
    } else {
        state.place_caret(target.byte_index);
    }
    commit(runtime, &state);
    runtime.caret_affinity.set(target.affinity);
}

pub(super) fn update_scroll(runtime: &TextEditRuntime, edit: &TextEdit, column: f32) {
    let style = runtime.text_style.peek();
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
    let current = runtime.scroll_x.peek();
    let offset = if caret_x < current {
        caret_x
    } else if caret_x > current + column {
        caret_x - column
    } else {
        current
    }
    .max(0.0);
    if (offset - current).abs() > f32::EPSILON {
        runtime.scroll_x.set(offset);
    }
}
