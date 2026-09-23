//! The single frame pipeline every host drives: `Host::frame` owns effect,
//! layout, focus, paint, and dirty state; hosts keep input decoding and
//! presentation.

use crate::animation_runtime::{active_count as active_animation_count, tick as tick_animations};
use crate::app::embed::Host;
use crate::app::exit::take_exit;
use crate::events::EventRouter;
use crate::layout::LayoutError;
use crate::reactive::{Runtime, tick_intervals};

/// Snapshot of frame state handed to the `paint` hook so the host can decide
/// whether presenting is worthwhile.
pub struct FrameInfo {
    /// A reactive write or explicit invalidation dirtied this frame.
    pub dirty: bool,
    /// Animators were active when the frame ticked.
    pub animating: bool,
    /// Layout completed without error.
    pub layout_ok: bool,
}

/// What `Host::frame` observed.
pub struct FrameOutput {
    /// `request_exit` fired while routing or ticking.
    pub exit: bool,
    /// Intervals that fired this frame.
    pub intervals: usize,
    /// Animators were active when the frame ticked.
    pub animating: bool,
    /// The layout pass result; `Err` carries the `LayoutError`.
    pub layout: Result<(), LayoutError>,
}

/// Effect-phase hook: decode input and route it through `Host::route`.
pub type FrameInputHook<'a> = Box<dyn FnOnce(&mut Host, &mut EventRouter) + 'a>;

/// Runs after a successful layout and focus repair, before paint.
pub type FrameLayoutHook<'a> = Box<dyn FnOnce(&mut Host, &mut EventRouter) + 'a>;

/// Paint-phase hook; the host presents from `FrameInfo`.
pub type FramePaintHook<'a> = Box<dyn FnOnce(&mut Host, &mut EventRouter, &FrameInfo) + 'a>;

/// Per-frame hooks. Each runs inside the shared pipeline at a fixed point;
/// unset hooks are no-ops.
#[derive(Default)]
pub struct FrameHooks<'a> {
    /// Runs inside the effect phase; see `FrameInputHook`.
    pub input: Option<FrameInputHook<'a>>,
    /// Runs after a successful layout and focus repair, before paint.
    pub after_layout: Option<FrameLayoutHook<'a>>,
    /// Runs inside the paint phase; see `FramePaintHook`.
    pub paint: Option<FramePaintHook<'a>>,
}

impl Host {
    /// Drive one frame: effect phase (input hook, intervals, animations),
    /// layout, focus repair, paint phase (paint hook), end of frame.
    pub fn frame(
        &mut self,
        router: &mut EventRouter,
        viewport: (f32, f32),
        dt: f32,
        hooks: FrameHooks<'_>,
    ) -> FrameOutput {
        let _env = self.enter();
        Runtime::begin_effect();
        if let Some(input) = hooks.input {
            input(self, router);
        }
        let intervals = tick_intervals();
        let animating = active_animation_count() > 0;
        if animating {
            tick_animations(dt);
        }
        let exit = take_exit();
        Runtime::begin_layout();
        let layout = self.layout(viewport).map(|_| ());
        let layout_ok = layout.is_ok();
        if layout_ok {
            self.apply_focus_requests(router);
            self.repair_focus(router);
            self.sync_hover(router);
            if let Some(after_layout) = hooks.after_layout {
                after_layout(self, router);
            }
        }
        Runtime::begin_paint();
        let info = FrameInfo {
            dirty: Runtime::frame_dirty(),
            animating,
            layout_ok,
        };
        if let Some(paint) = hooks.paint {
            paint(self, router, &info);
        }
        Runtime::end_frame();
        FrameOutput {
            exit,
            intervals,
            animating,
            layout,
        }
    }
}
