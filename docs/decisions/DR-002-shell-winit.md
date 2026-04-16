# DR-002 — Shell: winit 0.30+ with async bridge

**Status:** Accepted
**Date:** 2026-04-14

## Context

We need cross-platform window management, input event delivery, and event
loop orchestration. `winit` is the de-facto standard Rust windowing crate,
consumed upstream by Bevy, Blitz, Floem, and every other serious Rust UI
framework.

winit 0.30 introduced `ApplicationHandler`, a trait-based event loop
contract replacing the closure-based API. Key constraints:

- `ApplicationHandler` methods are **synchronous**. Maintainers state
  "winit was never designed to be async and won't be async" (issue
  #1199, discussion #3667).
- `EventLoopProxy::wake_up()` is the canonical cross-thread wake primitive.
  Its contract is "there may be several executions of the event loop
  between `wake_up()` being called and the event being delivered" — so it
  coalesces intent, not payload.
- Creating `wgpu::Surface`s is async but must happen inside
  `resumed` / `can_create_surfaces`. The sanctioned bridge is
  `pollster::block_on` at surface creation time.
- `tokio::runtime::Handle::current()` inside `ApplicationHandler` callbacks
  is reported to spawn duplicate runtimes in the wild. The safe pattern is
  to own the `Runtime` explicitly and enter its guard once.

## Decision

Depend on `winit` 0.30+ upstream. Wrap behind a `Shell` trait in `sigil-hal`.
Downstream crates name the trait, never `winit` directly.

The `Shell` trait surfaces:

- Window creation and lifecycle
- Pointer, keyboard, IME, and scroll event delivery
- Redraw request and frame callback
- Cross-thread wake (via `EventLoopProxy::wake_up()`)
- Main-thread dispatcher for async → signal updates (see DR-007)

Surface creation uses `pollster::block_on` synchronously inside the
winit handler, never Tokio.

## Consequences

- The library is fundamentally main-thread-driven. Signal reads and
  writes from background threads are funneled through the dispatcher.
- We pay the `winit` migration cost when it ships `winit-next`. The
  `Shell` trait confines that migration to `sigil-renderer` + `sigil-shell`.
- We are writing the **canonical winit+Tokio integration pattern**. No
  shipped Rust UI framework uses this combination today (Bevy uses its
  own scheduler, Floem has its own executor, GPUI has its own
  foreground/background abstraction). We document the pattern as public
  reference material.

## Kill criteria

Reconsider if:

- `winit-next` lands with an async-friendly API that makes the bridge
  unnecessary.
- A platform bug in `winit` on Windows, macOS, or Linux cannot be worked
  around and blocks shipping.

## References

- [winit — ApplicationHandler](https://docs.rs/winit/latest/winit/application/trait.ApplicationHandler.html)
- [winit async discussion #3667](https://github.com/rust-windowing/winit/discussions/3667)
- [winit issue #1199 — async considerations](https://github.com/rust-windowing/winit/issues/1199)
