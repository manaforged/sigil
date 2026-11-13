use crate::Error;
use crate::app::embed::Host;
use crate::app::exit::take_exit;
#[cfg(any(host, debug_assertions))]
use crate::kernel::FrameDelta;
use crate::reactive::{Runtime, tick_frame};

const MAX_ANIMATION_FRAME_SECS: f32 = 0.1;

/// One frame of a `Host`: what `Host::frame` returns, and what the paint hook sees before the frame ends.
#[non_exhaustive]
#[derive(Debug)]
pub struct Frame {
    pub(crate) exit: bool,
    pub(crate) intervals: usize,
    pub(crate) animating: bool,
    pub(crate) dirty: bool,
    pub(crate) layout: Result<(), Error>,
    #[cfg(any(host, debug_assertions))]
    pub(crate) delta: FrameDelta,
}

impl Frame {
    /// Returns whether the app asked to exit since the last frame.
    #[must_use]
    pub const fn exit(&self) -> bool {
        self.exit
    }

    /// Returns how many interval timers fired in this frame.
    #[must_use]
    pub const fn intervals(&self) -> usize {
        self.intervals
    }

    /// Returns whether an animation or a clock is still running, so another frame is due.
    #[must_use]
    pub const fn animating(&self) -> bool {
        self.animating
    }

    /// Returns whether a signal write or another change asked for a frame.
    #[must_use]
    pub const fn dirty(&self) -> bool {
        self.dirty
    }

    /// Returns `Ok` when layout finished, or the error that stopped it.
    pub fn layout(&self) -> Result<(), Error> {
        self.layout.clone()
    }
}

/// Runs before layout and may return a new viewport.
pub type FrameInputHook<'a> = Box<dyn FnOnce(&mut Host) -> Option<(f32, f32)> + 'a>;

/// Runs after layout.
pub type FrameLayoutHook<'a> = Box<dyn FnOnce(&mut Host) + 'a>;

/// Runs when the frame paints.
pub type FramePaintHook<'a> = Box<dyn FnOnce(&mut Host, &Frame) + 'a>;

/// The optional hooks that `Host::frame` runs.
#[must_use]
#[derive(Default)]
pub struct FrameHooks<'a> {
    pub(crate) input: Option<FrameInputHook<'a>>,
    pub(crate) after_layout: Option<FrameLayoutHook<'a>>,
    pub(crate) paint: Option<FramePaintHook<'a>>,
}

impl<'a> FrameHooks<'a> {
    /// Runs `input` before layout; a returned size replaces the viewport.
    pub fn input(mut self, input: impl FnOnce(&mut Host) -> Option<(f32, f32)> + 'a) -> Self {
        self.input = Some(Box::new(input));
        self
    }

    /// Runs `after_layout` after layout and before paint.
    pub fn after_layout(mut self, after_layout: impl FnOnce(&mut Host) + 'a) -> Self {
        self.after_layout = Some(Box::new(after_layout));
        self
    }

    /// Runs `paint` when the frame paints.
    pub fn paint(mut self, paint: impl FnOnce(&mut Host, &Frame) + 'a) -> Self {
        self.paint = Some(Box::new(paint));
        self
    }
}

impl core::fmt::Debug for FrameHooks<'_> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("FrameHooks").finish_non_exhaustive()
    }
}

#[cfg(host)]
impl Host {
    /// Returns whether a frame has work to do without new input: a signal write, an animation, a clock, or a timer that is due.
    #[must_use]
    pub fn frame_due() -> bool {
        Runtime::frame_dirty()
            || crate::animation::runtime::animating()
            || crate::reactive::next_interval_deadline()
                .is_some_and(|due| due <= web_time::Instant::now())
    }
}

impl Host {
    /// Runs one frame: effects, layout, animation, and the paint hook.
    pub fn frame(&mut self, mut viewport: (f32, f32), dt: f32, hooks: FrameHooks<'_>) -> Frame {
        #[cfg(hot_patch)]
        self.remount_patched();
        #[cfg(feature = "terminal")]
        self.drop_cell_screen();
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
        self.cx().flush_announcements();
        Runtime::begin_paint();
        let dirty = Runtime::frame_dirty();
        let delta = self.cx().take_delta();
        #[cfg(not(any(host, debug_assertions)))]
        self.cx().reuse_delta(delta);
        let mut frame = Frame {
            exit: exit.0,
            intervals,
            animating,
            dirty,
            layout,
            #[cfg(any(host, debug_assertions))]
            delta,
        };
        if let Some(paint) = hooks.paint {
            paint(self, &frame);
        }
        frame.animating |= crate::animation::runtime::animating();
        #[cfg(debug_assertions)]
        if frame.layout.is_ok()
            && !frame.animating
            && (frame.delta.structure || !frame.delta.layout.is_empty())
        {
            self.report_flaws();
        }
        #[cfg(any(host, debug_assertions))]
        self.cx().reuse_delta(std::mem::take(&mut frame.delta));
        Runtime::end_frame();
        crate::reactive::run_dispatch_deferred();
        frame
    }

    fn layout_phase(
        &mut self,
        viewport: (f32, f32),
        after_layout: Option<FrameLayoutHook<'_>>,
    ) -> Result<(), Error> {
        Runtime::begin_layout();
        let layout = {
            let _layout = crate::profile::span(crate::profile::Phase::Layout);
            self.settle_layout(viewport).and_then(|()| {
                let focus = self.focus();
                self.apply_focus_requests();
                self.repair_focus();
                let hover = self.sync_hover();
                if hover || self.focus() != focus {
                    self.settle_layout(viewport)
                } else {
                    Ok(())
                }
            })
        };
        if layout.is_ok()
            && let Some(after_layout) = after_layout
        {
            after_layout(self);
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
