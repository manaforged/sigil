use crate::animation_runtime::{
    active_count as active_animation_count, running_clocks, tick as tick_animations, tick_clocks,
};
use crate::app::embed::Host;
use crate::app::exit::take_exit;
use crate::events::EventRouter;
use crate::kernel::FrameDelta;
use crate::layout::LayoutError;
use crate::reactive::{Runtime, tick_intervals};

const MAX_ANIMATION_FRAME_SECS: f32 = 0.1;

#[non_exhaustive]
pub struct FrameInfo {
    pub dirty: bool,
    pub animating: bool,
    pub layout_ok: bool,
    pub delta: FrameDelta,
}

pub struct FrameOutput {
    pub exit: bool,
    pub intervals: usize,
    pub animating: bool,
    pub layout: Result<(), LayoutError>,
}

pub type FrameInputHook<'a> =
    Box<dyn FnOnce(&mut Host, &mut EventRouter) -> Option<(f32, f32)> + 'a>;

pub type FrameLayoutHook<'a> = Box<dyn FnOnce(&mut Host, &mut EventRouter) + 'a>;

pub type FramePaintHook<'a> = Box<dyn FnOnce(&mut Host, &mut EventRouter, &FrameInfo) + 'a>;

#[derive(Default)]
pub struct FrameHooks<'a> {
    pub input: Option<FrameInputHook<'a>>,
    pub after_layout: Option<FrameLayoutHook<'a>>,
    pub paint: Option<FramePaintHook<'a>>,
}

impl Host {
    pub fn frame(
        &mut self,
        router: &mut EventRouter,
        mut viewport: (f32, f32),
        dt: f32,
        hooks: FrameHooks<'_>,
    ) -> FrameOutput {
        #[cfg(all(feature = "hot", not(target_arch = "wasm32")))]
        self.remount_patched(router);
        let _env = self.enter();
        Runtime::begin_effect();
        if let Some(next) = hooks.input.and_then(|input| input(self, router)) {
            viewport = next;
        }
        crate::reactive::run_dispatch_deferred();
        self.apply_pending_theme();
        #[cfg(feature = "dialog")]
        self.sync_dialogs();
        let intervals = tick_intervals();
        let animating = advance(dt.min(MAX_ANIMATION_FRAME_SECS));
        self.publish_viewport(viewport);
        let exit = ExitOnUnwind(take_exit());
        Runtime::begin_layout();
        let layout = {
            let _layout = crate::profile::span(crate::profile::Phase::Layout);
            self.settle_layout(viewport)
        };
        let layout_ok = layout.is_ok();
        if layout_ok {
            self.apply_focus_requests(router);
            self.repair_focus(router);
            self.sync_hover(router);
            if let Some(after_layout) = hooks.after_layout {
                after_layout(self, router);
            }
        }
        self.cx().flush_announcements();
        Runtime::begin_paint();
        let info = FrameInfo {
            dirty: Runtime::frame_dirty(),
            animating,
            layout_ok,
            delta: self.cx().take_delta(),
        };
        if let Some(paint) = hooks.paint {
            paint(self, router, &info);
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
}

fn advance(dt: f32) -> bool {
    let animating = active_animation_count() > 0;
    if animating {
        tick_animations(dt);
    }
    tick_clocks(dt);
    animating || running_clocks() > 0
}

struct ExitOnUnwind(bool);

impl Drop for ExitOnUnwind {
    fn drop(&mut self) {
        if self.0 && std::thread::panicking() {
            crate::app::exit::defer_exit();
        }
    }
}
