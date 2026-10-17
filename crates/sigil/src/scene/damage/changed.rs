use smallvec::SmallVec;

use crate::geometry::Rect;
use crate::scene::command::effects::transform_rect;
use crate::scene::geometry::intersect_rect;
use crate::scene::state::{SceneState, ink_rect};
use crate::scene::{LayerEffect, LayerScope, SceneCommand};

enum Bound {
    Nothing,
    Rect(Option<Rect>),
    Full,
}

pub(super) fn changed_ink(
    commands: &[SceneCommand],
    end: usize,
    changed: &dyn Fn(usize) -> bool,
    (area, scale): (Rect, f32),
    inks: &mut SmallVec<[Rect; 16]>,
) -> Option<()> {
    let mut state = SceneState::default();
    let mut layers: SmallVec<[LayerScope; 4]> = SmallVec::new();
    let mut opened: SmallVec<[bool; 16]> = SmallVec::new();
    for (index, command) in commands.iter().enumerate().take(end) {
        let marked = changed(index);
        match command {
            SceneCommand::PushClip(_)
            | SceneCommand::PushTransform(_)
            | SceneCommand::PushLayer(_)
            | SceneCommand::SaveLayer { .. } => opened.push(marked),
            SceneCommand::PopClip
            | SceneCommand::PopTransform
            | SceneCommand::PopLayer
            | SceneCommand::RestoreLayer => {
                let push_changed = opened.pop().unwrap_or(false);
                if marked && !push_changed {
                    return None;
                }
            }
            _ => {}
        }
        if marked {
            let ink = match structural_ink(commands, index, &state, &layers, (area, scale)) {
                Some(Bound::Full) => return None,
                Some(Bound::Rect(rect)) => rect,
                Some(Bound::Nothing) => None,
                None => command_ink(command, &state, &layers, scale),
            };
            inks.extend(ink);
        }
        match command {
            SceneCommand::PushLayer(scope) => layers.push(*scope),
            SceneCommand::PopLayer => {
                layers.pop();
            }
            _ => {
                state.apply(command);
            }
        }
    }
    Some(())
}

fn structural_ink(
    commands: &[SceneCommand],
    index: usize,
    state: &SceneState,
    layers: &[LayerScope],
    (area, scale): (Rect, f32),
) -> Option<Bound> {
    let command = commands.get(index)?;
    Some(match command {
        SceneCommand::PushClip(rect) => Bound::Rect(within(
            transform_rect(state.current_transform(), *rect),
            state,
            layers,
        )),
        SceneCommand::PushLayer(_) => Bound::Rect(command_ink(command, state, layers, scale)),
        SceneCommand::SaveLayer { .. } => {
            let end = SceneState::group_end(commands, index + 1);
            let children = commands.get(index + 1..end).unwrap_or_default();
            Bound::Rect(state.draw_bounds(children, area, scale))
        }
        SceneCommand::PushTransform(_) => Bound::Full,
        SceneCommand::PopClip
        | SceneCommand::PopTransform
        | SceneCommand::PopLayer
        | SceneCommand::RestoreLayer => Bound::Nothing,
        _ => return None,
    })
}

fn within(rect: Rect, state: &SceneState, layers: &[LayerScope]) -> Option<Rect> {
    let mut rect = rect;
    for layer in layers {
        if matches!(layer.effect, LayerEffect::Identity) {
            rect = intersect_rect(rect, layer.clip)?;
        }
    }
    state.clip_rect(rect)
}

pub(crate) fn command_ink(
    command: &SceneCommand,
    state: &SceneState,
    layers: &[LayerScope],
    scale: f32,
) -> Option<Rect> {
    let moved = layers
        .iter()
        .find(|layer| matches!(layer.effect, LayerEffect::Transform(_)));
    let rect = match moved {
        Some(layer) => ink_rect(&SceneCommand::PushLayer(*layer), scale)?,
        None => transform_rect(state.current_transform(), ink_rect(command, scale)?),
    };
    within(rect, state, layers)
}
