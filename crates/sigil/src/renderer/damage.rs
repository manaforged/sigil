use smallvec::SmallVec;

pub(crate) use crate::scene::damage::FullReason;

use crate::animation::runtime::linear_channel_to_srgb;
use crate::geometry::{Point, Rect};
use crate::renderer::scene_command::state::SceneState;
use crate::renderer::scene_command::walk::intersect_rect;
use crate::renderer::scene_command::{CommandScene, LayerScope, SceneCommand};
use crate::scene::command::payloads::DrawRect;
use crate::scene::corner_radii::CornerRadii;
use crate::scene::damage::Budget;
use crate::scene::damage::scene_damage;
use crate::scene::damage::{Damage, command_ink};
use crate::style::{Color, Rgba};

const FULL_OPACITY: u16 = 10_000;
const CLEAR_BLEED: f32 = 2.0;
const MAX_REGIONS: usize = 3;
pub(crate) const GPU_BUDGET: Budget = Budget {
    max_regions: MAX_REGIONS,
    margin: 1.0,
    max_share: 0.6,
};

#[derive(Clone, PartialEq)]
pub(crate) struct FrameKey {
    pub(crate) target: wgpu::Texture,
    pub(crate) viewport: (f32, f32),
    pub(crate) scale: f32,
    pub(crate) clear: Rgba,
}

pub(crate) struct Retained {
    key: FrameKey,
    scene: CommandScene,
}

pub(crate) type Plan = Damage<Regions>;

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
        scene_damage((&self.scene, scene), (area, key.scale), GPU_BUDGET)
    }
}

/// Keeps the commands of `scene` that can paint inside `region`, with no clip around them.
#[cfg(feature = "terminal")]
pub(crate) fn culled(scene: &CommandScene, region: Rect) -> CommandScene {
    let mut commands = Vec::with_capacity(scene.commands.len());
    cull_into(&scene.commands, region, &mut commands);
    CommandScene { commands }
}

pub(crate) fn clipped(scene: &CommandScene, regions: &[Rect], clear: Rgba) -> CommandScene {
    let fill = Color {
        r: linear_channel_to_srgb(clear.r),
        g: linear_channel_to_srgb(clear.g),
        b: linear_channel_to_srgb(clear.b),
        a: u8::MAX,
    };
    let mut commands = Vec::with_capacity(scene.commands.len() + 3 * regions.len());
    for region in regions {
        commands.push(SceneCommand::PushClip((*region).into()));
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
                layers.push(scope.clone());
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

pub(crate) fn note_plan(plan: &Plan) {
    match plan {
        Plan::Unchanged => crate::profile::count(crate::profile::Counter::GpuPlanUnchanged, 1),
        Plan::Regions(_) => crate::profile::count(crate::profile::Counter::GpuPlanRegions, 1),
        Plan::Full(reason) => {
            crate::profile::count(crate::profile::Counter::GpuPlanFull, 1);
            let key = reason.name().as_ptr() as usize as u64;
            crate::profile::attribute(crate::profile::Hotspot::GpuFullRepaint, 1, key, || {
                reason.name().to_owned()
            });
        }
    }
}
