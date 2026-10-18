use crate::kernel::FrameDelta;
use crate::scene::CommandScene;
use crate::scene::emit::{EmitDiff, EmitMemo, emit_scene_since};

use super::{Host, viewport_rect};

impl Host {
    #[must_use]
    pub fn emit_since(
        &mut self,
        viewport: (f32, f32),
        delta: &FrameDelta,
        prev: &CommandScene,
    ) -> (CommandScene, EmitDiff) {
        let _env = self.enter();
        self.apply_pending_theme();
        let Some(root) = self.root else {
            self.emitted = EmitMemo::default();
            return (
                CommandScene::default(),
                EmitDiff::between_scenes(&prev.commands, &[]),
            );
        };
        emit_scene_since(
            &self.cx,
            root,
            Some(viewport_rect(viewport)),
            (delta, prev),
            &mut self.emitted,
        )
    }

    #[cfg(feature = "terminal")]
    pub(crate) fn emit_unchanged(
        &mut self,
        viewport: (f32, f32),
        delta: &FrameDelta,
        prev: &CommandScene,
    ) -> bool {
        let _env = self.enter();
        self.apply_pending_theme();
        self.root.is_some_and(|root| {
            crate::scene::emit::emit_unchanged(
                &self.cx,
                (root, Some(viewport_rect(viewport))),
                (delta, prev),
                &self.emitted,
            )
        })
    }

    pub fn reset_emit(&mut self) {
        self.emitted = EmitMemo::default();
    }
}
