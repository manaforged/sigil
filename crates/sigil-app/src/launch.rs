//! `sigil::launch` — the one-call entry point.
//!
//! Owns the full frame pipeline: reactive scope → shell/window → renderer
//! → lazy surface → cx/layout/router → event drain → scene emit → paint.
//! Every failure mode listed in `phase-08a-ergonomics-pass.md`'s
//! bulletproof table becomes unreachable at this layer.

use std::sync::Arc;

use sigil_core::Cx;
use sigil_events::EventRouter;
use sigil_hal::{
    Color, Key, KeyPhase, LogicalSize, PhysicalSize, Renderer as _, Scale, Shell, ShellEvent,
    WindowEvent,
};
use sigil_layout::LayoutEngine;
use sigil_layout::prelude::Size;
use sigil_reactive::Runtime;
use sigil_renderer::{WgpuRenderer, WgpuSurface, emit_scene, into_window_handle};
use sigil_shell::{WinitShell, WinitWindowHandle};

use crate::config::AppConfig;
use crate::error::{Error, Result};
use crate::text_measure::DefaultTextMeasure;
use crate::tree_root::TreeRoot;

/// Launch a sigil app.
///
/// `config` accepts `()`, `&str`, `(title, w, h)`, or `AppConfig`.
/// `tree` accepts either `FnOnce(&mut Cx) -> NodeId` or a `NodeBuilder`.
///
/// Returns when the user closes the window or (by default) presses Esc.
///
/// # Errors
///
/// Returns an error if the shell, renderer, or layout engine fails to
/// initialise. Runtime errors during the frame loop (surface lost,
/// transient wgpu timeouts) are handled internally — the loop retries
/// on the next frame.
pub fn launch<C, T>(config: C, tree: T) -> Result<()>
where
    C: Into<AppConfig>,
    T: TreeRoot,
{
    let config = config.into();
    Runtime::scope("sigil-app", || run(config, tree))
}

fn run<T: TreeRoot>(config: AppConfig, tree: T) -> Result<()> {
    let logical_size = logical_size_from_config(config.size);

    let mut shell = WinitShell::new().map_err(Error::new)?;
    let window_handle = create_initial_window(&mut shell, &config, logical_size)?;

    let mut cx = Cx::new_root();
    let root = tree.install(&mut cx);

    let mut layout = LayoutEngine::with_text_measure(Box::new(DefaultTextMeasure::new()));
    layout_for_scale(&mut cx, root, &mut layout, Scale::ONE, logical_size).map_err(Error::new)?;

    let mut router = EventRouter::new();
    let mut renderer = WgpuRenderer::new()?;
    let mut surface: Option<WgpuSurface> = None;
    let mut viewport = (logical_size.width, logical_size.height);
    let mut exit_requested = false;

    shell
        .run(move |shell| {
            frame_once(FrameCtx {
                config: &config,
                shell,
                window_handle,
                cx: &mut cx,
                root,
                layout: &mut layout,
                router: &mut router,
                renderer: &mut renderer,
                surface: &mut surface,
                viewport: &mut viewport,
                exit_requested: &mut exit_requested,
            });
        })
        .map_err(Error::new)?;
    Ok(())
}

fn create_initial_window<S>(
    shell: &mut S,
    config: &AppConfig,
    logical_size: LogicalSize,
) -> Result<S::Window>
where
    S: Shell,
{
    shell
        .create_window_logical(&config.title, logical_size)
        .map_err(Error::new)
}

struct FrameCtx<'a> {
    config: &'a AppConfig,
    shell: &'a mut WinitShell,
    window_handle: WinitWindowHandle,
    cx: &'a mut Cx,
    root: sigil_core::NodeId,
    layout: &'a mut LayoutEngine,
    router: &'a mut EventRouter,
    renderer: &'a mut WgpuRenderer,
    surface: &'a mut Option<WgpuSurface>,
    viewport: &'a mut (f32, f32),
    exit_requested: &'a mut bool,
}

fn frame_once(ctx: FrameCtx<'_>) {
    let FrameCtx {
        config,
        shell,
        window_handle,
        cx,
        root,
        layout,
        router,
        renderer,
        surface,
        viewport,
        exit_requested,
    } = ctx;

    ensure_surface(EnsureSurfaceCtx {
        shell,
        window_handle,
        cx,
        root,
        layout,
        renderer,
        surface,
        viewport,
    });
    drain_events(DrainCtx {
        shell,
        cx,
        root,
        layout,
        router,
        surface,
        renderer,
        viewport,
        config,
        exit_requested,
    });
    if *exit_requested {
        shell.request_exit();
    }
    let logical = logical_size_from_viewport(*viewport);
    recompute_layout(cx, root, layout, cx.scale(), logical);
    paint(cx, root, renderer, surface, *viewport);
}

struct DrainCtx<'a> {
    shell: &'a mut WinitShell,
    cx: &'a mut Cx,
    root: sigil_core::NodeId,
    layout: &'a mut LayoutEngine,
    router: &'a mut EventRouter,
    surface: &'a mut Option<WgpuSurface>,
    renderer: &'a mut WgpuRenderer,
    viewport: &'a mut (f32, f32),
    config: &'a AppConfig,
    exit_requested: &'a mut bool,
}

struct EnsureSurfaceCtx<'a> {
    shell: &'a WinitShell,
    window_handle: WinitWindowHandle,
    cx: &'a mut Cx,
    root: sigil_core::NodeId,
    layout: &'a mut LayoutEngine,
    renderer: &'a mut WgpuRenderer,
    surface: &'a mut Option<WgpuSurface>,
    viewport: &'a mut (f32, f32),
}

fn ensure_surface(ctx: EnsureSurfaceCtx<'_>) {
    let EnsureSurfaceCtx {
        shell,
        window_handle,
        cx,
        root,
        layout,
        renderer,
        surface,
        viewport,
    } = ctx;
    if surface.is_some() {
        return;
    }
    let Some(window) = shell.window_for(window_handle) else {
        return;
    };

    let scale = scale_from_f64(window.scale_factor());
    cx.set_scale(scale);
    let logical = logical_size_from_viewport(*viewport);
    recompute_layout(cx, root, layout, scale, logical);

    let handle = into_window_handle(Arc::clone(window));
    match renderer.create_surface(handle, scale.to_physical(logical)) {
        Ok(new_surface) => *surface = Some(new_surface),
        Err(err) => tracing::warn!("sigil-app: create_surface failed: {err}"),
    }
}

fn drain_events(ctx: DrainCtx<'_>) {
    let DrainCtx {
        shell,
        cx,
        root,
        layout,
        router,
        surface,
        renderer,
        viewport,
        config,
        exit_requested,
    } = ctx;
    for event in shell.drain_events() {
        if let ShellEvent::Window(window_event) = &event {
            let mut window_ctx = WindowEventCtx {
                cx,
                root,
                layout,
                renderer,
                surface,
                viewport,
                exit_requested,
            };
            handle_window_event(&mut window_ctx, window_event);
        }
        match &event {
            ShellEvent::Key(key_event)
                if config.esc_exits
                    && key_event.key == Key::Escape.into()
                    && key_event.phase == KeyPhase::Down =>
            {
                *exit_requested = true;
            }
            _ => {}
        }
        router.route(cx, layout, event);
    }
}

struct WindowEventCtx<'a> {
    cx: &'a mut Cx,
    root: sigil_core::NodeId,
    layout: &'a mut LayoutEngine,
    renderer: &'a mut WgpuRenderer,
    surface: &'a mut Option<WgpuSurface>,
    viewport: &'a mut (f32, f32),
    exit_requested: &'a mut bool,
}

fn handle_window_event(ctx: &mut WindowEventCtx<'_>, event: &WindowEvent) {
    match event {
        WindowEvent::CloseRequested | WindowEvent::Destroyed => {
            *ctx.exit_requested = true;
        }
        WindowEvent::Resized(size) => handle_resized(ctx, *size),
        WindowEvent::ScaleFactorChanged(scale) => handle_scale_changed(ctx, *scale),
        WindowEvent::Focused(_) | WindowEvent::Occluded(_) => {}
    }
}

fn handle_resized(ctx: &mut WindowEventCtx<'_>, physical: PhysicalSize) {
    resize_surface(ctx.renderer, ctx.surface, physical);
    let scale = ctx.cx.scale();
    let logical = scale.to_logical(clamp_physical_size(physical));
    set_viewport(ctx.viewport, logical);
    recompute_layout(ctx.cx, ctx.root, ctx.layout, scale, logical);
}

fn handle_scale_changed(ctx: &mut WindowEventCtx<'_>, scale: Scale) {
    ctx.cx.set_scale(scale);
    let logical = logical_size_from_viewport(*ctx.viewport);
    resize_surface(ctx.renderer, ctx.surface, scale.to_physical(logical));
    recompute_layout(ctx.cx, ctx.root, ctx.layout, scale, logical);
}

fn resize_surface(
    renderer: &mut WgpuRenderer,
    surface: &mut Option<WgpuSurface>,
    physical: PhysicalSize,
) {
    if let Some(surface) = surface.as_mut()
        && let Err(err) = renderer.resize(surface, physical)
    {
        tracing::warn!("sigil-app: resize failed: {err}");
    }
}

fn recompute_layout(
    cx: &mut Cx,
    root: sigil_core::NodeId,
    layout: &mut LayoutEngine,
    scale: Scale,
    logical: LogicalSize,
) {
    if let Err(err) = layout_for_scale(cx, root, layout, scale, logical) {
        tracing::warn!("sigil-app: layout recompute failed: {err}");
    }
}

fn layout_for_scale(
    cx: &mut Cx,
    root: sigil_core::NodeId,
    layout: &mut LayoutEngine,
    _scale: Scale,
    logical_viewport: LogicalSize,
) -> core::result::Result<(), sigil_layout::LayoutError> {
    layout.compute(
        cx,
        root,
        Size::fixed(logical_viewport.width, logical_viewport.height),
    )
}

const fn set_viewport(viewport: &mut (f32, f32), logical: LogicalSize) {
    viewport.0 = logical.width;
    viewport.1 = logical.height;
}

const fn logical_size_from_config(size: (u32, u32)) -> LogicalSize {
    LogicalSize::new(f32_from_u32(size.0), f32_from_u32(size.1))
}

const fn logical_size_from_viewport(viewport: (f32, f32)) -> LogicalSize {
    LogicalSize::new(viewport.0, viewport.1)
}

fn clamp_physical_size(size: PhysicalSize) -> PhysicalSize {
    PhysicalSize::new(size.width.max(1), size.height.max(1))
}

#[allow(clippy::cast_precision_loss)]
const fn f32_from_u32(v: u32) -> f32 {
    v as f32
}

#[allow(clippy::cast_possible_truncation)]
fn scale_from_f64(scale: f64) -> Scale {
    Scale::new(scale as f32)
}

fn paint(
    cx: &Cx,
    root: sigil_core::NodeId,
    renderer: &mut WgpuRenderer,
    surface: &mut Option<WgpuSurface>,
    viewport: (f32, f32),
) {
    let Some(surface) = surface.as_mut() else {
        return;
    };
    let scene = emit_scene(cx, root);
    let clear = Color::rgba(0.03, 0.04, 0.05, 1.0);
    if let Err(err) = renderer.render_command_scene(surface, &scene, clear, viewport) {
        tracing::warn!("sigil-app: render_command_scene failed: {err}");
    }
}

#[cfg(test)]
mod tests {
    #![allow(clippy::unwrap_used)]

    use std::fmt;

    use sigil_core::{Cx, container};
    use sigil_hal::{LogicalSize, PhysicalSize, Scale, Shell, ShellEvent};
    use sigil_layout::prelude::{LayoutBuilderExt, LayoutEngine, LayoutStyle};
    use sigil_reactive::Runtime;

    use crate::config::AppConfig;
    use crate::launch::{create_initial_window, layout_for_scale};
    use crate::text_measure::DefaultTextMeasure;

    #[derive(Debug)]
    struct MockError;

    impl fmt::Display for MockError {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            f.write_str("mock shell error")
        }
    }

    impl std::error::Error for MockError {}

    #[derive(Clone)]
    struct MockWaker;

    #[derive(Debug, PartialEq)]
    enum CreateCall {
        Physical(String, PhysicalSize),
        Logical(String, LogicalSize),
    }

    #[derive(Default)]
    struct MockShell {
        calls: Vec<CreateCall>,
    }

    impl Shell for MockShell {
        type Error = MockError;
        type Waker = MockWaker;
        type Window = usize;

        fn waker(&self) -> Self::Waker {
            MockWaker
        }

        fn wake(_waker: &Self::Waker) -> Result<(), Self::Error> {
            Ok(())
        }

        fn create_window(
            &mut self,
            title: &str,
            initial_size: PhysicalSize,
        ) -> Result<Self::Window, Self::Error> {
            self.calls
                .push(CreateCall::Physical(title.to_owned(), initial_size));
            Ok(self.calls.len() - 1)
        }

        fn create_window_logical(
            &mut self,
            title: &str,
            initial: LogicalSize,
        ) -> Result<Self::Window, Self::Error> {
            self.calls
                .push(CreateCall::Logical(title.to_owned(), initial));
            Ok(self.calls.len() - 1)
        }

        fn drain_events(&mut self) -> std::vec::IntoIter<ShellEvent> {
            Vec::new().into_iter()
        }

        fn run<F>(self, _frame: F) -> Result<(), Self::Error>
        where
            F: FnMut(&mut Self) + 'static,
        {
            Ok(())
        }
    }

    #[test]
    fn initial_window_uses_logical_shell_creation() {
        let mut shell = MockShell::default();
        let config = AppConfig::new("test").size(720, 480);
        let logical = LogicalSize::new(720.0, 480.0);

        let handle = create_initial_window(&mut shell, &config, logical).unwrap();

        assert_eq!(handle, 0);
        assert_eq!(
            shell.calls,
            vec![CreateCall::Logical("test".to_owned(), logical)]
        );
    }

    #[test]
    fn layout_for_scale_keeps_logical_rects_stable() {
        Runtime::scope("test", || {
            let mut cx = Cx::new_root();
            let root = cx.add(container().layout(LayoutStyle::block().fixed_size(720.0, 480.0)));
            let logical = LogicalSize::new(720.0, 480.0);
            let mut layout = LayoutEngine::with_text_measure(Box::new(DefaultTextMeasure::new()));

            layout_for_scale(&mut cx, root, &mut layout, Scale::ONE, logical).unwrap();
            let one_x = layout.result(root).unwrap().rect();

            layout_for_scale(&mut cx, root, &mut layout, Scale::new(2.0), logical).unwrap();
            let two_x = layout.result(root).unwrap().rect();

            assert_eq!(one_x, two_x);
            assert_eq!(two_x.width.to_bits(), 720.0_f32.to_bits());
            assert_eq!(two_x.height.to_bits(), 480.0_f32.to_bits());
        });
    }
}
