//! `hello-rect` — internals / kernel-demo.
//!
//! This example predates `sigil::launch` and drives the kernel + scene-
//! command surface directly. New users should start with `hello-counter`
//! (the minimum viable reactive UI) or `hello-events` (the recommended
//! programming model) and reach for this only when working below the
//! `sigil::launch` facade.
//!
//! Opens a window, clears it to a colour, and cycles through three
//! colours every 600 ms. All five foundation crates cooperate:
//!
//! - `sigil-reactive` holds the colour index as a `Signal<usize>`.
//! - `sigil-async` owns a Tokio runtime. A background task ticks every
//!   600 ms and posts a write through `dispatch_to_owner`.
//! - `sigil-shell` runs the winit event loop and delivers wake events
//!   into the async dispatcher.
//! - `sigil-renderer` clears the surface to the current colour.
//! - `sigil-hal` is what every downstream crate names in its imports.
//!
// hello-rect example: indices are bounded by construction
// (modulo COLORS.len()). See docs/plans/native-ui/hygiene-cleanup-framework.md.
#![allow(clippy::indexing_slicing)]
#![allow(clippy::missing_errors_doc, clippy::missing_panics_doc)]
#![allow(missing_docs)]

// wgpu-hal 29.0.1 GLES/WGL backend needs `windows-core` with the
// `std` feature enabled. `sigil-renderer` carries the same workaround;
// this binary pulls it in too so the feature unification holds at
// link time across the whole executable. Remove once wgpu ships a
// fix.
#[cfg(windows)]
use windows_core as _;

use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;
use std::time::Duration;

use sigil_async::{AsyncRuntime, OwnerHandle, dispatch_to_owner};
use sigil_hal::{Color, PhysicalSize, Renderer, Scheduler, Shell};
use sigil_reactive::{Runtime, signal};
use sigil_renderer::{WgpuRenderer, WgpuSurface, into_window_handle};
use sigil_shell::WinitShell;
// `winit` and `tokio` arrive transitively via sigil-shell / sigil-async.
// The `use` markers silence `unused_crate_dependencies` on direct
// deps we only reference through the adapters.
use tokio as _;
use winit as _;

/// Background colours cycled every 600 ms (linear RGB).
const COLORS: [Color; 3] = [
    Color::rgba(0.05, 0.07, 0.10, 1.0), // near-black
    Color::rgba(0.14, 0.45, 0.95, 1.0), // accent blue
    Color::rgba(0.95, 0.35, 0.20, 1.0), // warm orange
];

// Example entry point — the setup + reactive scope + event-loop form one
// coherent narrative that cannot be split without losing the walkthrough
// value. 7 lines over the soft cap; allowed per the hygiene framework.
#[allow(clippy::too_many_lines)]
fn main() {
    // ----- construct the three runtimes on the main thread -----

    let mut shell = WinitShell::new().expect("shell");

    let mut async_rt = AsyncRuntime::new().expect("async runtime");
    // Wire the event loop proxy so background enqueues wake the main
    // thread. The waker fires once per enqueue; winit coalesces.
    let waker = shell.waker();
    async_rt.set_waker(move || {
        let _ = WinitShell::wake(&waker);
    });
    let dispatcher = async_rt.dispatcher();
    // AsyncRuntime needs mutable access for `drain` and `spawn`. It
    // is main-thread-only so Rc<RefCell<_>> is the right wrapper.
    let async_rt = Rc::new(RefCell::new(async_rt));

    let mut renderer = WgpuRenderer::new().expect("wgpu renderer");

    // ----- ask the shell for a window; it materialises on resumed -----

    let window_handle = shell
        // hello-rect still uses the pre-DR-016 path, so this size is
        // physical pixels. TODO(DR-018, Phase 08c): port to launch so
        // the example receives logical pixels like hello-style/events.
        .create_window("Sigil - kernel-demo", PhysicalSize::new(720, 480))
        .expect("create window");

    // ----- build the reactive state + spawn the cycling task -----

    // We run everything inside a single `Runtime::scope` so
    // disposal at program exit is clean.
    Runtime::scope("kernel-demo", || {
        let color_index = signal(0_usize);
        let owner = OwnerHandle::current().expect("inside scope");

        // Cross-thread handle: Signal<T> is !Send, so we hand a
        // SendableSignal to the Tokio task and re-hydrate it inside
        // the main-thread dispatch closure.
        let sendable = color_index.into_sendable();
        let dispatcher_bg = dispatcher.clone();

        // Spawn the ticker through the Scheduler trait. Cancellation
        // is tied to `owner` disposal — when the scope ends, the
        // task is aborted on the next drain.
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

        // ----- run the event loop -----

        // Surface is created lazily once winit materialises the window
        // inside ApplicationHandler::resumed.
        let renderer_surface: Rc<RefCell<Option<WgpuSurface>>> = Rc::new(RefCell::new(None));

        let async_rt_frame = Rc::clone(&async_rt);
        let surface_frame = Rc::clone(&renderer_surface);

        shell
            .run(move |s| {
                // Drain any pending signal updates from background
                // tasks before rendering.
                async_rt_frame.borrow_mut().drain();

                // Lazy surface creation on the first frame after the
                // window materialises.
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

                // Read current colour and clear. `.get` here would
                // subscribe an effect if one were active; we are in
                // the frame closure (no current_effect), so it just
                // reads.
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
