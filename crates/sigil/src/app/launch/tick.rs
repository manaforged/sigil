use crate::app::config::PanicPolicy;
use crate::app::error::AppResult;
use crate::app::window::set_current_window;
use crate::shell::{WinitShell, WinitWindowHandle};
use crate::tasks::AsyncRuntime;

use super::backend::Backend;
use super::clock::AnimationClock;
use super::lifecycle::{apply_commands, dispatch_events, open_requested};
use super::record::{SignalWrite, WindowRecord, Windows};
use super::{request_animation_frame, run_frame};

pub(super) struct FrameCtx<'a> {
    pub(super) shell: &'a mut WinitShell,
    pub(super) record: &'a mut WindowRecord,
    pub(super) renderer: &'a mut Backend,
    pub(super) exit_requested: &'a mut bool,
    pub(super) async_rt: &'a mut AsyncRuntime,
    pub(super) clock: &'a mut AnimationClock,
    pub(super) signal_writes: Vec<SignalWrite>,
}

pub(super) struct LoopState {
    windows: Windows,
    renderer: Backend,
    async_rt: AsyncRuntime,
    clock: AnimationClock,
    exit_requested: bool,
    panic_policy: PanicPolicy,
}

impl LoopState {
    pub(super) fn new(
        windows: Windows,
        renderer: Backend,
        async_rt: AsyncRuntime,
        panic_policy: PanicPolicy,
    ) -> Self {
        Self {
            windows,
            renderer,
            async_rt,
            clock: AnimationClock::default(),
            exit_requested: false,
            panic_policy,
        }
    }

    pub(super) fn tick(&mut self, shell: &mut WinitShell) -> AppResult<()> {
        open_requested(shell, &mut self.windows)?;
        dispatch_events(shell, &mut self.windows);
        let target = shell
            .redraw_target()
            .or_else(|| self.windows.first_handle());
        if let Some(handle) = target {
            self.frame_window(shell, handle)?;
        }
        apply_commands(shell, &mut self.windows, target);
        open_requested(shell, &mut self.windows)?;
        dispatch_events(shell, &mut self.windows);
        if crate::app::exit::take_exit() {
            self.exit_requested = true;
        }
        if self.exit_requested || self.windows.is_empty() {
            shell.request_exit();
        }
        request_animation_frame(shell, target);
        Ok(())
    }

    fn frame_window(&mut self, shell: &mut WinitShell, handle: WinitWindowHandle) -> AppResult<()> {
        let signal_writes = self.windows.take_signal_writes();
        let Some(record) = self.windows.get_mut(handle) else {
            return Ok(());
        };
        let previous = set_current_window(Some(record.view.id));
        let result = run_frame(
            self.panic_policy,
            FrameCtx {
                shell,
                record,
                renderer: &mut self.renderer,
                exit_requested: &mut self.exit_requested,
                async_rt: &mut self.async_rt,
                clock: &mut self.clock,
                signal_writes,
            },
        );
        set_current_window(previous);
        let painted = self
            .windows
            .get_mut(handle)
            .is_some_and(|entry| std::mem::take(&mut entry.view.painted));
        if painted {
            self.windows.mark_others_for_repaint(handle);
            shell.request_redraws();
        }
        result
    }

    #[cfg(feature = "inspect")]
    pub(super) fn service_inspect(&mut self, inspect: &crate::app::inspect::Inspect) {
        let renderer = &mut self.renderer;
        let Some(record) = self.windows.first_mut() else {
            return;
        };
        let WindowRecord { host, router, view } = record;
        let viewport = view.viewport;
        let surface = view.surface.as_ref();
        let mut shot = |app: &mut crate::app::embed::Host| {
            super::backend::inspect_shot(app, renderer, surface, viewport)
        };
        inspect.service(
            host,
            router,
            crate::app::inspect::View {
                cell: None,
                text: None,
                shot: Some(&mut shot),
            },
        );
    }
}
