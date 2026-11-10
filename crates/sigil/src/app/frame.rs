use crate::Error;
use crate::app::embed::Host;
use crate::app::exit::take_exit;
#[cfg(any(feature = "terminal", debug_assertions))]
use crate::kernel::FrameDelta;
use crate::reactive::{Runtime, tick_frame};

const MAX_ANIMATION_FRAME_SECS: f32 = 0.1;

/// State of the current frame that `Host::frame` passes to the paint hook, with the frame's `FrameDelta` for `Host::emit_since`.
#[non_exhaustive]
#[derive(Debug)]
pub struct FrameInfo {
    /// Whether dirty.
    pub dirty: bool,
    /// Whether animating.
    pub animating: bool,
    /// Whether layout ok.
    pub layout_ok: bool,
    #[cfg(any(feature = "terminal", debug_assertions))]
    pub(crate) delta: FrameDelta,
}

/// Result of `Host::frame`: exit request, number of intervals fired, whether animations still run, and the layout result.
#[derive(Debug)]
pub struct FrameOutput {
    /// Whether exit.
    pub exit: bool,
    /// The intervals.
    pub intervals: usize,
    /// Whether animating.
    pub animating: bool,
    /// The layout.
    pub layout: Result<(), Error>,
}

/// Runs before layout and may return a new viewport.
pub type FrameInputHook<'a> = Box<dyn FnOnce(&mut Host) -> Option<(f32, f32)> + 'a>;

/// Runs after layout.
pub(crate) type FrameLayoutHook<'a> = Box<dyn FnOnce(&mut Host) + 'a>;

/// Runs when the frame paints.
pub type FramePaintHook<'a> = Box<dyn FnOnce(&mut Host, &FrameInfo) + 'a>;

/// The optional hooks that `Host::frame` runs.
#[derive(Default)]
pub struct FrameHooks<'a> {
    /// The input, if any.
    pub input: Option<FrameInputHook<'a>>,
    /// The after layout, if any.
    pub after_layout: Option<FrameLayoutHook<'a>>,
    /// The paint, if any.
    pub paint: Option<FramePaintHook<'a>>,
}

impl core::fmt::Debug for FrameHooks<'_> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("FrameHooks").finish_non_exhaustive()
    }
}

impl Host {
    /// Runs one frame: effects, layout, animation, and the paint hook.
    pub fn frame(
        &mut self,
        mut viewport: (f32, f32),
        dt: f32,
        hooks: FrameHooks<'_>,
    ) -> FrameOutput {
        #[cfg(all(feature = "hot", not(target_arch = "wasm32")))]
        self.remount_patched();
        let _env = self.enter();
        Runtime::begin_effect();
        if let Some(next) = hooks.input.and_then(|input| input(self)) {
            viewport = next;
        }
        crate::reactive::run_dispatch_deferred();
        self.apply_pending_theme();
        #[cfg(feature = "dialog")]
        self.sync_dialogs();
        let (intervals, animating) = tick_frame(dt.min(MAX_ANIMATION_FRAME_SECS));
        self.publish_viewport(viewport);
        let exit = ExitOnUnwind(take_exit());
        let layout = self.layout_phase(viewport, hooks.after_layout);
        let layout_ok = layout.is_ok();
        self.cx().flush_announcements();
        Runtime::begin_paint();
        let dirty = Runtime::frame_dirty();
        let delta = self.cx().take_delta();
        #[cfg(not(any(feature = "terminal", debug_assertions)))]
        drop(delta);
        let info = FrameInfo {
            dirty,
            animating,
            layout_ok,
            #[cfg(any(feature = "terminal", debug_assertions))]
            delta,
        };
        if let Some(paint) = hooks.paint {
            paint(self, &info);
        }
        #[cfg(debug_assertions)]
        if layout_ok && !animating && (info.delta.structure || !info.delta.layout.is_empty()) {
            self.report_flaws();
        }
        Runtime::end_frame();
        crate::reactive::run_dispatch_deferred();
        FrameOutput {
            exit: exit.0,
            intervals,
            animating,
            layout,
        }
    }

    fn layout_phase(
        &mut self,
        viewport: (f32, f32),
        after_layout: Option<FrameLayoutHook<'_>>,
    ) -> Result<(), Error> {
        Runtime::begin_layout();
        let mut layout = {
            let _layout = crate::profile::span(crate::profile::Phase::Layout);
            self.settle_layout(viewport)
        };
        if layout.is_ok() {
            let focus = self.focus();
            self.apply_focus_requests();
            self.repair_focus();
            if self.focus() != focus {
                layout = self.settle_layout(viewport);
            }
            self.sync_hover();
            if let Some(after_layout) = after_layout {
                after_layout(self);
            }
        }
        layout
    }
}

struct ExitOnUnwind(bool);

impl Drop for ExitOnUnwind {
    fn drop(&mut self) {
        if self.0 && std::thread::panicking() {
            crate::app::exit::defer_exit();
        }
    }
}
