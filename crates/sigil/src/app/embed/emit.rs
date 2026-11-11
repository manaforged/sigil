#[cfg(terminal_host)]
use crate::kernel::FrameDelta;
use crate::scene::CommandScene;
use crate::scene::emit::CleanScene;
#[cfg(terminal_host)]
use crate::scene::emit::{EmitDiff, EmitMemo, emit_scene_since};

use super::{Host, viewport_rect};

impl Host {
    pub(super) fn emit_cached(&mut self, viewport: (f32, f32)) -> CommandScene {
        let _env = self.enter();
        self.apply_pending_theme();
        let Some(root) = self.root else {
            self.reset_emit();
            return CommandScene::default();
        };
        self.clean_emit.emit(
            &self.cx,
            (root, Some(viewport_rect(viewport))),
            self.layout.generation(),
        )
    }

    #[cfg(terminal_host)]
    #[must_use]
    pub(crate) fn emit_since(
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

    kittest_pub! {
        /// Clears the memo that incremental painting reuses.
        pub fn reset_emit(&mut self) {
            #[cfg(terminal_host)]
            {
                self.emitted = EmitMemo::default();
            }
            self.clean_emit = CleanScene::default();
        }
    }
}
