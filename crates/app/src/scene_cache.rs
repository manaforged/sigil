use sigil_hal::Color;
use sigil_kernel::{Cx, NodeId};
use sigil_reactive::Runtime;
use sigil_renderer::{CommandScene, SceneCommand};

use crate::launch::backend::{Backend, Surface};

use crate::embed::emit_viewport_scene;

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
            sigil_profile::count("scene.emit");
            self.scene = emit_viewport_scene(cx, root, viewport);
            self.key = Some(key);
            #[cfg(test)]
            {
                self.emissions += 1;
            }
        } else {
            sigil_profile::count("scene.replay");
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
        let scene = {
            let _emit = sigil_profile::span("emit");
            self.scene(cx, root, viewport, allow_replay)
        };
        let clear = if renderer.is_transparent() {
            Color::rgba(0.0, 0.0, 0.0, 0.0)
        } else {
            Color::rgba(0.03, 0.04, 0.05, 1.0)
        };
        let _gpu = sigil_profile::span("gpu");
        if let Err(err) = renderer.render(surface, scene, clear, viewport) {
            tracing::warn!("sigil-app: render_command_scene failed: {err}");
            Runtime::mark_frame_dirty();
        }
        note_gpu(renderer.last_frame_stats());
    }
}

fn note_scene(scene: &CommandScene) {
    if !sigil_profile::enabled() {
        return;
    }
    sigil_profile::count_add("scene.cmds", scene.commands.len() as u64);
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
    sigil_profile::count_add("scene.glyphs", glyphs);
    sigil_profile::count_add("scene.rects", rects);
    sigil_profile::count_add("scene.images", images);
}

fn note_gpu(stats: sigil_renderer::FrameStats) {
    if !sigil_profile::enabled() {
        return;
    }
    sigil_profile::count_add("gpu.rects", stats.rectangle_instances as u64);
    sigil_profile::count_add("gpu.glyphs", stats.glyph_runs as u64);
    sigil_profile::count_add("gpu.sprites", stats.sprite_draws as u64);
    sigil_profile::count_add("gpu.layers", stats.offscreen_layers as u64);
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
