use smallvec::SmallVec;

mod reason;
mod regions;

pub(crate) use reason::{FullReason, note_plan};
use regions::regions;

use crate::animation_runtime::linear_channel_to_srgb;
use crate::geometry::{Point, Rect};
use crate::renderer::scene_command::state::{SceneState, ink_rect};
use crate::renderer::scene_command::walk::{intersect_rect, transform_rect};
use crate::renderer::scene_command::{CommandScene, LayerEffect, LayerScope, SceneCommand};
use crate::scene::command::payloads::DrawRect;
use crate::scene::corner_radii::CornerRadii;
use crate::style::{Color, Rgba8};

const FULL_OPACITY: u16 = 10_000;
const AA_MARGIN: f32 = 1.0;
const CLEAR_BLEED: f32 = 2.0;
const MAX_REGION_SHARE: f32 = 0.6;
const MAX_REGIONS: usize = 3;
const MAX_CHANGED_SHARE: usize = 4;
const MIN_SHARE_SCENE: usize = 64;
const RESYNC_WINDOW: usize = 16;

#[derive(Clone, PartialEq)]
pub(crate) struct FrameKey {
    pub(crate) target: wgpu::Texture,
    pub(crate) viewport: (f32, f32),
    pub(crate) scale: f32,
    pub(crate) clear: Color,
}

pub(crate) struct Retained {
    key: FrameKey,
    scene: CommandScene,
}

#[derive(Debug, PartialEq)]
pub(crate) enum Plan {
    Unchanged,
    Regions(Regions),
    Full(FullReason),
}

pub(crate) type Regions = SmallVec<[Rect; MAX_REGIONS]>;

impl Retained {
    pub(crate) const fn new(key: FrameKey, scene: CommandScene) -> Self {
        Self { key, scene }
    }

    pub(crate) fn plan(&self, key: &FrameKey, scene: &CommandScene) -> Plan {
        if key.clear.a < 1.0 {
            return Plan::Full(FullReason::TranslucentClear);
        }
        if self.key.viewport != key.viewport {
            return Plan::Full(FullReason::ViewportResized);
        }
        if self.key != *key {
            return Plan::Full(FullReason::KeyChanged);
        }
        let area = Rect::new(Point::new(0.0, 0.0), key.viewport.0, key.viewport.1);
        scene_damage(&self.scene, scene, area, key.scale)
    }
}

pub(crate) fn scene_damage(
    previous: &CommandScene,
    next: &CommandScene,
    area: Rect,
    scale: f32,
) -> Plan {
    let (before, after) = (&previous.commands, &next.commands);
    if after.iter().any(
        |command| matches!(command, SceneCommand::PushLayer(scope) if !local_effect(scope.effect)),
    ) {
        return Plan::Full(FullReason::NonLocalLayerEffect);
    }
    let prefix = before.iter().zip(after).take_while(|(a, b)| a == b).count();
    if prefix == before.len() && prefix == after.len() {
        return Plan::Unchanged;
    }
    let room = before.len().min(after.len()) - prefix;
    let suffix = before
        .iter()
        .rev()
        .zip(after.iter().rev())
        .take(room)
        .take_while(|(a, b)| a == b)
        .count();
    let (old, new) = align(before, after, prefix, suffix);
    let changed_count = old.iter().chain(&new).filter(|mark| **mark).count();
    if after.len() >= MIN_SHARE_SCENE && changed_count * MAX_CHANGED_SHARE > after.len() * 2 {
        return Plan::Full(FullReason::ChangedShare);
    }
    let mut inks: SmallVec<[Rect; 16]> = SmallVec::new();
    for (commands, marks) in [(before, &old), (after, &new)] {
        let end = marks
            .iter()
            .rposition(|mark| *mark)
            .map_or(0, |last| last + 1);
        let changed = |index: usize| marks.get(index).copied().unwrap_or(false);
        if changed_ink(commands, end, &changed, (area, scale), &mut inks).is_none() {
            return Plan::Full(FullReason::UnboundedChange);
        }
    }
    regions(&inks, area, scale)
}

fn align(
    before: &[SceneCommand],
    after: &[SceneCommand],
    prefix: usize,
    suffix: usize,
) -> (Vec<bool>, Vec<bool>) {
    let mut old = vec![false; before.len()];
    let mut new = vec![false; after.len()];
    let (end_old, end_new) = (before.len() - suffix, after.len() - suffix);
    let (mut i, mut j) = (prefix, prefix);
    while i < end_old && j < end_new {
        if before.get(i) == after.get(j) {
            i += 1;
            j += 1;
            continue;
        }
        let skip = (1..=RESYNC_WINDOW).find_map(|step| {
            if i + step < end_old && before.get(i + step) == after.get(j) {
                Some((step, 0))
            } else if j + step < end_new && before.get(i) == after.get(j + step) {
                Some((0, step))
            } else {
                None
            }
        });
        let (skip_old, skip_new) = skip.unwrap_or((1, 1));
        old.iter_mut()
            .skip(i)
            .take(skip_old)
            .for_each(|mark| *mark = true);
        new.iter_mut()
            .skip(j)
            .take(skip_new)
            .for_each(|mark| *mark = true);
        i += skip_old;
        j += skip_new;
    }
    old.iter_mut()
        .take(end_old)
        .skip(i)
        .for_each(|mark| *mark = true);
    new.iter_mut()
        .take(end_new)
        .skip(j)
        .for_each(|mark| *mark = true);
    (old, new)
}

fn local_effect(effect: LayerEffect) -> bool {
    match effect {
        LayerEffect::Identity => true,
        LayerEffect::Transform([[_, skew_x, _], [skew_y, _, _]]) => skew_x == 0.0 && skew_y == 0.0,
        LayerEffect::LayerBlur { .. } => false,
    }
}

enum Bound {
    Nothing,
    Rect(Option<Rect>),
    Full,
}

fn changed_ink(
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

fn command_ink(
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

pub(crate) fn clipped(scene: &CommandScene, regions: &[Rect], clear: Color) -> CommandScene {
    let fill = Rgba8 {
        r: linear_channel_to_srgb(clear.r),
        g: linear_channel_to_srgb(clear.g),
        b: linear_channel_to_srgb(clear.b),
        a: u8::MAX,
    };
    let mut commands = Vec::with_capacity(scene.commands.len() + 3 * regions.len());
    for region in regions {
        commands.push(SceneCommand::PushClip(*region));
        commands.push(SceneCommand::Rect(DrawRect {
            node: None,
            rect: region.expand(CLEAR_BLEED),
            color: fill,
            border_radius: CornerRadii::ZERO,
            opacity: FULL_OPACITY,
            stroke: None,
            tone: None,
            leaf: true,
            border: None,
            outline: false,
        }));
        cull_into(&scene.commands, *region, &mut commands);
        commands.push(SceneCommand::PopClip);
    }
    CommandScene { commands }
}

fn cull_into(source: &[SceneCommand], region: Rect, out: &mut Vec<SceneCommand>) {
    let mut state = SceneState::default();
    let mut layers: SmallVec<[LayerScope; 4]> = SmallVec::new();
    let mut index = 0;
    while let Some(command) = source.get(index) {
        let keep = match command {
            SceneCommand::PushLayer(scope) => {
                let hit = command_ink(command, &state, &layers, 1.0)
                    .is_some_and(|ink| intersect_rect(ink, region).is_some());
                if !hit {
                    index = SceneState::layer_end(source, index + 1) + 1;
                    continue;
                }
                layers.push(*scope);
                true
            }
            SceneCommand::PopLayer => {
                layers.pop();
                true
            }
            _ if state.apply(command) => true,
            _ => command_ink(command, &state, &layers, 1.0)
                .is_some_and(|ink| intersect_rect(ink, region).is_some()),
        };
        if keep {
            out.push(command.clone());
        }
        index += 1;
    }
}
