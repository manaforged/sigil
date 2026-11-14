use crate::kernel::{Cx, FrameDelta, Node, NodeId};
use crate::profile::{Counter, Phase};
use crate::reactive::Runtime;
use crate::scene::emit::{EmitMemo, emit_scene_into};
use crate::scene::{CommandScene, SceneCommand, paint_backdrop};
use crate::style::Theme;

use crate::app::launch::backend::{Backend, RenderSurface};

use crate::app::embed::{emit_viewport_scene, viewport_rect};

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
            scale: cx.scale().to_bits(),
        }
    }
}

#[derive(Default)]
pub(super) struct SceneCache {
    key: Option<SceneKey>,
    body: CommandScene,
    scene: CommandScene,
    memo: EmitMemo,
    backdrop: bool,
    built_backdrop: bool,
    missed: bool,
    #[cfg(test)]
    emissions: usize,
}

impl SceneCache {
    pub(super) fn scene(
        &mut self,
        cx: &Cx,
        (root, viewport): (NodeId, (f32, f32)),
        (allow_replay, delta): (bool, Option<&FrameDelta>),
    ) -> &CommandScene {
        let key = SceneKey::new(cx, root, viewport);
        if allow_replay
            && !self.missed
            && self.key == Some(key)
            && self.built_backdrop == self.backdrop
        {
            crate::profile::count(Counter::SceneReplay, 1);
        } else {
            crate::profile::count(Counter::SceneEmit, 1);
            self.rebuild(cx, (root, viewport, key), delta);
            #[cfg(test)]
            {
                self.emissions += 1;
            }
        }
        note_scene(&self.scene);
        &self.scene
    }

    fn rebuild(
        &mut self,
        cx: &Cx,
        (root, viewport, key): (NodeId, (f32, f32), SceneKey),
        delta: Option<&FrameDelta>,
    ) {
        let clip = Some(viewport_rect(viewport));
        let delta = if std::mem::take(&mut self.missed) {
            None
        } else {
            delta
        };
        let changed = match delta {
            Some(delta) if self.key == Some(key) => {
                emit_scene_into(cx, (root, clip), delta, &mut self.body, &mut self.memo)
            }
            Some(delta) => {
                self.body.commands.clear();
                emit_scene_into(cx, (root, clip), delta, &mut self.body, &mut self.memo)
            }
            None => {
                self.memo = EmitMemo::default();
                self.body = emit_viewport_scene(cx, root, viewport);
                true
            }
        };
        if changed || self.key != Some(key) || self.built_backdrop != self.backdrop {
            compose_window_scene(
                cx,
                (root, viewport, self.backdrop),
                &self.body,
                &mut self.scene,
            );
            self.built_backdrop = self.backdrop;
        }
        self.key = Some(key);
    }

    pub(super) const fn skip(&mut self) {
        self.missed = true;
    }

    pub(super) fn paint(&mut self, request: PaintRequest<'_>) {
        let PaintRequest {
            cx,
            root,
            renderer,
            surface,
            viewport,
            allow_replay,
            delta,
        } = request;
        self.backdrop = !renderer.is_transparent();
        let Some(surface) = surface.as_mut() else {
            self.skip();
            return;
        };
        {
            let _emit = crate::profile::span(Phase::Emit);
            self.scene(cx, (root, viewport), (allow_replay, delta));
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

#[cfg(any(test, feature = "inspect"))]
pub(super) fn window_scene(
    cx: &Cx,
    root: NodeId,
    viewport: (f32, f32),
    backdrop: bool,
) -> CommandScene {
    let body = emit_viewport_scene(cx, root, viewport);
    let mut scene = CommandScene::default();
    compose_window_scene(cx, (root, viewport, backdrop), &body, &mut scene);
    scene
}

fn compose_window_scene(
    cx: &Cx,
    (root, viewport, backdrop): (NodeId, (f32, f32), bool),
    body: &CommandScene,
    out: &mut CommandScene,
) {
    out.commands.clear();
    out.commands
        .reserve(body.commands.len() + usize::from(backdrop));
    if backdrop {
        let theme = cx
            .get(root)
            .and_then(Node::extension::<Theme>)
            .unwrap_or_else(crate::style::theme::fallback_theme);
        paint_backdrop(out, theme, viewport);
    }
    out.commands.extend_from_slice(&body.commands);
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
            SceneCommand::Rects(draws) => rects += draws.len() as u64,
            command if command.is_image() => images += 1,
            _ => {}
        }
    }
    crate::profile::count(Counter::SceneGlyphs, glyphs);
    crate::profile::count(Counter::SceneRects, rects);
    crate::profile::count(Counter::SceneImages, images);
}

pub(super) struct PaintRequest<'a> {
    pub(super) cx: &'a Cx,
    pub(super) root: NodeId,
    pub(super) renderer: &'a mut Backend,
    pub(super) surface: &'a mut Option<RenderSurface>,
    pub(super) viewport: (f32, f32),
    pub(super) allow_replay: bool,
    pub(super) delta: Option<&'a FrameDelta>,
}

#[cfg(test)]
#[path = "scene_cache_tests.rs"]
mod tests;
