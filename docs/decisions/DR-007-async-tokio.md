# DR-007 — Async: one Tokio runtime per app

**Status:** Accepted
**Date:** 2026-04-14

## Context

Components need to load data, stream events, and coordinate background
work without blocking the main thread. The reactive arena (DR-005) is
main-thread-only, so background work must funnel through a dispatcher.

Research on winit + async integration (April 2026):

- winit `ApplicationHandler` methods are synchronous (DR-002).
- `EventLoopProxy::wake_up()` coalesces intent, not payload. "There may
  be several executions of the event loop between `wake_up()` being
  called and the event being delivered."
- `tokio::runtime::Handle::current()` inside `ApplicationHandler`
  callbacks is reported to spawn duplicate runtimes in the wild. The
  safe pattern is to own the `Runtime` explicitly and enter its guard
  once at app construction.
- **There is no shipped Rust UI framework using winit + Tokio.** Bevy
  has its own scheduler, Blitz uses `pollster` for surface init only,
  Floem has its own executor, GPUI has its own foreground/background
  abstraction. We are writing the canonical pattern.

## Decision

Each `App` owns a **single Tokio multi-thread runtime**, not a runtime
per window. Windows share the runtime through a `Handle`.

- The `Runtime` is constructed explicitly by the app, its guard is
  entered once at startup, and the `Handle` is stored on the app
  context.
- Consumers use `app.spawn(future)`, never `tokio::spawn` directly.
- Background tasks publish `SignalUpdate` messages through an MPSC
  into a main-thread dispatcher. They then call
  `EventLoopProxy::wake_up()` to wake the main thread.
- `EventLoopProxy::wake_up()` coalesces intent. The MPSC is the payload
  channel; the proxy is the wake signal.
- The dispatcher drains the MPSC at the start of every frame, applies
  the writes to the reactive arena, and triggers the resulting effects
  synchronously on the main thread.
- Public async API is one function:
  `spawn_to_signal(future) -> Signal<Status<T>>` where `Status<T>` is
  `Pending | Ok(T) | Err(E)`. Components observe the signal like any
  other value; threading is invisible.
- Cancellation is tied to owner disposal (DR-005). When the owner that
  created the resource is disposed, the underlying future is aborted.
- The dispatcher is part of the `Shell` trait (DR-002) so it can be
  stubbed for headless testing.

Surface creation uses `pollster::block_on` inside the winit handler.
Tokio is **not** used for surface init. This matches Blitz's approach.

## Consequences

- Tokio is a heavy dependency, accepted in exchange for the most
  battle-tested async runtime in Rust. Custom executors and `smol` were
  considered and rejected: `smol` is less proven at our scale; a custom
  executor is a multi-month subsystem that buys nothing user-visible.
- We ship and document the canonical winit+Tokio integration pattern as
  public reference material. This is a differentiator.
- Owner-disposal-based cancellation means that naive `drop` of a
  component's scope also cancels its async work. No separate cleanup
  required.

## Kill criteria

Reconsider if:

- A critical Tokio bug or maintainership change materially affects
  stability.
- The main-thread dispatcher shows measurable contention under load
  that cannot be fixed by batching or frame coalescing.
- A compelling alternative (`smol`, `async-executor`, or a Rust
  platform executor) ships with better winit integration.

## References

- [tokio](https://github.com/tokio-rs/tokio)
- [winit EventLoopProxy](https://docs.rs/winit/latest/winit/event_loop/struct.EventLoopProxy.html)
- [pollster](https://github.com/zesterer/pollster)
