#![allow(clippy::indexing_slicing)]
#![allow(clippy::missing_errors_doc, clippy::missing_panics_doc)]
#![allow(missing_docs)]

#[cfg(windows)]
use windows_core as _;

use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;
use std::time::Duration;

use sigil_hal::{Color, PhysicalSize, Renderer, Scheduler, Shell};
use sigil_reactive::{Runtime, signal};
use sigil_renderer::{WgpuRenderer, WgpuSurface, into_window_handle};
use sigil_shell::WinitShell;
use sigil_tasks::{AsyncRuntime, OwnerHandle, dispatch_to_owner};
use tokio as _;
use winit as _;

const COLORS: [Color; 3] = [
    Color::rgba(0.05, 0.07, 0.10, 1.0),
    Color::rgba(0.14, 0.45, 0.95, 1.0),
    Color::rgba(0.95, 0.35, 0.20, 1.0),
];

#[allow(
    clippy::too_many_lines,
    reason = "one sequential pass; splitting would scatter the ordering it documents"
)]
fn main() {
    let mut shell = WinitShell::new().expect("shell");

    let mut async_rt = AsyncRuntime::new().expect("async runtime");
    let waker = shell.waker();
    async_rt.set_waker(move || {
        let _ = WinitShell::wake(&waker);
    });
    let dispatcher = async_rt.dispatcher();
    let async_rt = Rc::new(RefCell::new(async_rt));

    let mut renderer = WgpuRenderer::new().expect("wgpu renderer");

    let window_handle = shell
        .create_window("Sigil - kernel-demo", PhysicalSize::new(720, 480))
        .expect("create window");

    Runtime::scope("kernel-demo", || {
        let color_index = signal(0_usize);
        let owner = OwnerHandle::current().expect("inside scope");

        let sendable = color_index.into_sendable();
        let dispatcher_bg = dispatcher.clone();

        let _task = async_rt.borrow().spawn(owner, async move {
            let mut i: usize = 0;
            loop {
                tokio::time::sleep(Duration::from_millis(600)).await;
                i = (i + 1) % COLORS.len();
                dispatch_to_owner(&dispatcher_bg, owner, move || {
                    sendable.into_signal().set(i);
                });
            }
        });

        let renderer_surface: Rc<RefCell<Option<WgpuSurface>>> = Rc::new(RefCell::new(None));

        let async_rt_frame = Rc::clone(&async_rt);
        let surface_frame = Rc::clone(&renderer_surface);

        shell
            .run(move |s| {
                async_rt_frame.borrow_mut().drain();

                let mut surface_slot = surface_frame.borrow_mut();
                if surface_slot.is_none()
                    && let Some(window) = s.window_for(window_handle)
                {
                    let size = window.inner_size();
                    let handle = into_window_handle(Arc::clone(window));
                    let surface = renderer
                        .create_surface(
                            handle,
                            PhysicalSize::new(size.width.max(1), size.height.max(1)),
                        )
                        .expect("create surface");
                    *surface_slot = Some(surface);
                }

                if let Some(surface) = surface_slot.as_mut() {
                    let idx = color_index.get();
                    let color = COLORS[idx];
                    if let Err(err) = renderer.clear(surface, color) {
                        eprintln!("clear error: {err}");
                    }
                }
            })
            .expect("event loop");
    });
}
