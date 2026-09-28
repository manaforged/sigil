use crate::kernel::{Cx, NodeId};
use crate::profile::{Counter, Phase};
use crate::reactive::Runtime;
use crate::scene::{CommandScene, SceneCommand};

use crate::app::launch::backend::{Backend, Surface};

use crate::app::embed::emit_viewport_scene;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct SceneKey {
    context: u64,
    root: NodeId,
    viewport: [u32; 2],
    scale: u32,
}

impl SceneKey {
    const fn new(cx: &Cx, root: NodeId, viewport: (f32, f32)) -> Self {
        Self {
            context: cx.instance_id(),
            root,
            viewport: [viewport.0.to_bits(), viewport.1.to_bits()],
            scale: cx.scale().factor().to_bits(),
        }
    }
}

#[derive(Default)]
pub(super) struct SceneCache {
    key: Option<SceneKey>,
    scene: CommandScene,
    #[cfg(test)]
    emissions: usize,
}

impl SceneCache {
    pub(super) fn scene(
        &mut self,
        cx: &Cx,
        root: NodeId,
        viewport: (f32, f32),
        allow_replay: bool,
    ) -> &CommandScene {
        let key = SceneKey::new(cx, root, viewport);
        if !allow_replay || self.key != Some(key) {
            crate::profile::count(Counter::SceneEmit);
            self.scene = emit_viewport_scene(cx, root, viewport);
            self.key = Some(key);
            #[cfg(test)]
            {
                self.emissions += 1;
            }
        } else {
            crate::profile::count(Counter::SceneReplay);
        }
        note_scene(&self.scene);
        &self.scene
    }

    pub(super) fn paint(&mut self, request: PaintRequest<'_>) {
        let PaintRequest {
            cx,
            root,
            renderer,
            surface,
            viewport,
            allow_replay,
        } = request;
        let Some(surface) = surface.as_mut() else {
            return;
        };
        {
            let _emit = crate::profile::span(Phase::Emit);
            self.scene(cx, root, viewport, allow_replay);
        }
        let base = self.scene.commands.len();
        if crate::hud::enabled() {
            crate::hud::overlay(&mut self.scene, viewport);
        }
        let clear = renderer.clear_color();
        let _render = crate::profile::span(Phase::RenderCpu);
        if let Err(err) = renderer.render(surface, &self.scene, clear, viewport) {
            tracing::warn!("sigil-app: render_command_scene failed: {err}");
            Runtime::mark_frame_dirty();
        }
        self.scene.commands.truncate(base);
        renderer.last_frame_stats().record_profile();
    }
}

fn note_scene(scene: &CommandScene) {
    if !crate::profile::enabled() {
        return;
    }
    let mut glyphs = 0_u64;
    let mut rects = 0_u64;
    let mut images = 0_u64;
    for command in &scene.commands {
        match command {
            SceneCommand::GlyphRun(_) => glyphs += 1,
            SceneCommand::Rect(_) | SceneCommand::Gradient(_) | SceneCommand::Pattern(_) => {
                rects += 1;
            }
            SceneCommand::Image(_) | SceneCommand::Sprite(_) => images += 1,
            _ => {}
        }
    }
    crate::profile::count_add(Counter::SceneGlyphs, glyphs);
    crate::profile::count_add(Counter::SceneRects, rects);
    crate::profile::count_add(Counter::SceneImages, images);
}

pub(super) struct PaintRequest<'a> {
    pub(super) cx: &'a Cx,
    pub(super) root: NodeId,
    pub(super) renderer: &'a mut Backend,
    pub(super) surface: &'a mut Option<Surface>,
    pub(super) viewport: (f32, f32),
    pub(super) allow_replay: bool,
}

#[cfg(test)]
#[path = "scene_cache_tests.rs"]
mod tests;
