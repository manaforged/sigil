use smallvec::SmallVec;

use crate::geometry::Rect;
use crate::scene::command::effects::transform_rect;
use crate::scene::geometry::intersect_rect;
use crate::scene::state::{SceneState, ink_rect};
use crate::scene::{LayerEffect, LayerScope, SceneCommand};

enum ChangeBound {
    Nothing,
    Rect(Option<Rect>),
    #[cfg(any(
        test,
        feature = "kittest",
        all(feature = "scene", any(feature = "desktop", feature = "terminal"))
    ))]
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
        match scope_edge(command) {
            Some(true) => opened.push(marked),
            Some(false) => {
                let push_changed = opened.pop().unwrap_or(false);
                if marked && !push_changed {
                    return None;
                }
            }
            None => {}
        }
        if marked {
            let ink = match structural_ink(commands, index, &state, &layers, (area, scale)) {
                #[cfg(any(
                    test,
                    feature = "kittest",
                    all(feature = "scene", any(feature = "desktop", feature = "terminal"))
                ))]
                Some(ChangeBound::Full) => return None,
                Some(ChangeBound::Rect(rect)) => rect,
                Some(ChangeBound::Nothing) => None,
                None => command_ink(command, &state, &layers, scale),
            };
            inks.extend(ink);
            inks.extend(unmoved(command, &state, &layers, scale));
        }
        match command {
            SceneCommand::PushLayer(scope) => layers.push(scope.clone()),
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

const fn scope_edge(command: &SceneCommand) -> Option<bool> {
    match command {
        SceneCommand::PushClip(_) | SceneCommand::PushLayer(_) => Some(true),
        SceneCommand::PopClip | SceneCommand::PopLayer => Some(false),
        #[cfg(any(
            test,
            feature = "kittest",
            all(feature = "scene", any(feature = "desktop", feature = "terminal"))
        ))]
        SceneCommand::PushTransform(_) => Some(true),
        #[cfg(any(
            test,
            feature = "kittest",
            all(feature = "scene", any(feature = "desktop", feature = "terminal"))
        ))]
        SceneCommand::PopTransform => Some(false),
        #[cfg(any(test, feature = "kittest", feature = "desktop", feature = "terminal"))]
        SceneCommand::SaveLayer { .. } => Some(true),
        #[cfg(any(test, feature = "kittest", feature = "terminal"))]
        SceneCommand::RestoreLayer => Some(false),
        _ => None,
    }
}

fn structural_ink(
    commands: &[SceneCommand],
    index: usize,
    state: &SceneState,
    layers: &[LayerScope],
    (area, scale): (Rect, f32),
) -> Option<ChangeBound> {
    let command = commands.get(index)?;
    Some(match command {
        SceneCommand::PushClip(clip) => ChangeBound::Rect(within(
            transform_rect(state.current_transform(), clip.bounds()),
            state,
            layers,
        )),
        SceneCommand::PushLayer(_) => ChangeBound::Rect(command_ink(command, state, layers, scale)),
        #[cfg(any(test, feature = "kittest", feature = "desktop", feature = "terminal"))]
        SceneCommand::SaveLayer { .. } => {
            let end = SceneState::group_end(commands, index + 1);
            let children = commands.get(index + 1..end).unwrap_or_default();
            ChangeBound::Rect(state.draw_bounds(children, area, scale))
        }
        #[cfg(any(
            test,
            feature = "kittest",
            all(feature = "scene", any(feature = "desktop", feature = "terminal"))
        ))]
        SceneCommand::PushTransform(_) => ChangeBound::Full,
        SceneCommand::PopClip | SceneCommand::PopLayer => ChangeBound::Nothing,
        #[cfg(any(
            test,
            feature = "kittest",
            all(feature = "scene", any(feature = "desktop", feature = "terminal"))
        ))]
        SceneCommand::PopTransform => ChangeBound::Nothing,
        #[cfg(any(test, feature = "kittest", feature = "terminal"))]
        SceneCommand::RestoreLayer => ChangeBound::Nothing,
        _ => return None,
    })
}

fn unmoved(
    command: &SceneCommand,
    state: &SceneState,
    layers: &[LayerScope],
    scale: f32,
) -> Option<Rect> {
    let SceneCommand::PushLayer(scope) = command else {
        return None;
    };
    if !matches!(
        scope.effect,
        LayerEffect::Transform(_) | LayerEffect::Projected { .. }
    ) {
        return None;
    }
    let still = LayerScope {
        effect: LayerEffect::Identity,
        ..*scope
    };
    command_ink(&SceneCommand::PushLayer(still), state, layers, scale)
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
    let moved = layers.iter().find(|layer| {
        matches!(
            layer.effect,
            LayerEffect::Transform(_) | LayerEffect::Projected { .. }
        )
    });
    let rect = match moved {
        Some(layer) => ink_rect(&SceneCommand::PushLayer(layer.clone()), scale)?,
        None => transform_rect(state.current_transform(), ink_rect(command, scale)?),
    };
    within(rect, state, layers)
}
