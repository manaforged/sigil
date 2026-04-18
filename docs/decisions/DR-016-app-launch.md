# DR-016 — Application shell: `sigil::launch`

**Status:** Accepted
**Date:** 2026-04-16
**Closes:** F-002 (shell async handle) partially — see Consequences
**Depends on:** DR-002 (winit shell), DR-005 (reactive owner tree), DR-010 (sigil-core), DR-011 (sigil-layout), DR-015 (fragment semantics)
**Unlocks:** Phase 08 (widgets), Phase 08b (DPI)

## Context

Before this DR, every Sigil app reimplemented the same 370–395 lines of
winit / wgpu / layout / router orchestration to reach a painted window.
`examples/hello-style/src/main.rs` was the worst offender at 370 lines;
`hello-events` at 395. The tree itself — the part that varies between
apps — was ~60 lines. Everything else was identical boilerplate that
every new Sigil user would write, debug, and then maintain when an
internal API shifted.

Widgets (Phase 08) will be composed by library consumers, not Sigil
itself. Landing `Button`, `Text`, `Input` on top of the pre-DR surface
meant every widget-using app started from 370 lines of ceremony. That
is the opposite of DR-005 / DR-010's "bulletproof AND easy" posture and
makes the framework a poor choice for the data-heavy apps the project
targets.

Surveying the 2026 Rust UI ecosystem, the convergent entry-point idiom
across Xilem, Dioxus, and Floem is **one call, function-as-root,
optional config, `Result` return**. Builders (Iced, eframe) are the
older pattern; attribute macros (SwiftUI) are fashion-forward but
debuggability-hostile.

## Decision

Ship a new crate `sigil-app` exposing `launch(config, tree_fn) -> Result<()>`,
plus a facade crate `sigil` that re-exports the API and a `prelude`
module for user code.

### Public surface

```rust
use sigil::prelude::*;

fn main() -> sigil::Result<()> {
    sigil::launch(("Sigil - demo", 1280, 720), app_tree)
}

fn app_tree(cx: &mut Cx) -> NodeId {
    cx.add(
        container()
            .layout(LayoutStyle::flex_column().fixed_size(1280.0, 720.0))
            .class("bg-surface")
            .child(container().class("bg-accent rounded-md")),
    )
}
```

`launch`'s first argument accepts any `Into<AppConfig>`:

- `()` → defaults (720×480, "Sigil")
- `&str` / `String` → custom title, default size
- `(&str, u32, u32)` / `(String, u32, u32)` → title + size
- `AppConfig::new(title).size(…).esc_exits(false)` → full config

`launch`'s second argument accepts any `TreeRoot`:

- `FnOnce(&mut Cx) -> NodeId` — the common case, gives access to `cx`
  for later event wiring.
- `NodeBuilder` — direct-pass for the simplest demos.

### Bulletproof contract

Each failure mode the old boilerplate exposed becomes unreachable from
the new surface:

| Failure mode | How `launch` prevents it |
|---|---|
| Forget `Runtime::scope`, signals never dispose | `launch` owns the scope |
| Create surface before window resumes, wgpu panics | `launch` lazily creates on first resumed frame |
| Forget `layout.compute`, all geometry is zero | `launch` computes on install + on resize |
| Forget resize, surface stretches | `launch` reconfigures on `Resized` |
| Forget ESC, no quit | `launch` registers Esc=exit by default (`AppConfig::esc_exits`) |
| Zero-size window on minimize, wgpu panics | `launch` clamps `PhysicalSize::new(w.max(1), h.max(1))` |
| Forget to pass `&layout` to `router.route`, hit-test broken | `launch` owns the frame pipeline |

### Crate shape

- `crates/sigil-app` — the orchestrator. Owns `WinitShell`,
  `WgpuRenderer`, `Cx`, `LayoutEngine`, `EventRouter`, lazy
  `WgpuSurface`. Exports `launch`, `AppConfig`, `TreeRoot`, `Error`,
  `Result`, `DefaultTextMeasure`, `PanicPolicy`.
- `crates/sigil` — thin facade. `pub use sigil_app::*` plus a
  `prelude` module pulling in `sigil_core::prelude::*`,
  `sigil_events::prelude::*`, `sigil_layout::prelude::*`, `signal`,
  `effect`, `StyleAtom`, `Theme`, `Style`, `NodeStyleExt`, and the
  launch API. Users write `use sigil::prelude::*;` and get everything
  they need.

## Rationale

**Why function-as-root, not builder.** Iced and eframe's builder pattern
(`Counter::run(Settings::default())` / `eframe::run_native(title, opts,
Box::new(|cc| …))`) is the older shape. The 2026 standard is a single
call taking a function, because:

- The builder's chained `.title(...).size(...).run()` reads worse for
  the 80% case than `launch("title", tree)`.
- The builder discourages factoring — every config access becomes a
  method, so adding config options bloats the surface.
- Function-as-root composes: `launch(cfg, some_existing_component_fn)`
  is idiomatic Rust.

**Why `Into<AppConfig>` + sealed `TreeRoot`.** Polymorphic args keep
the call site terse while preserving type safety. `Into` covers the
four config shapes without separate `launch_with_size` / `launch_with_title`
functions. Sealed `TreeRoot` covers `fn` and `NodeBuilder` with a single
`launch` name; the seal prevents third-party impls that could subvert
the frame contract (e.g. a `TreeRoot` that panics mid-install).

**Why a separate `sigil` facade.** User-facing ergonomics:
`use sigil::prelude::*;` reads better than `use sigil_app::...` or a
seven-line import block. The facade has zero runtime code — it's a
re-export shell.

**Why Esc-exits by default.** Demo apps and examples quit; shipping
apps opt out via `AppConfig::esc_exits(false)`. The demo case is the
majority, so the default matches it.

## Consequences

### Examples

- `hello-style` ported to launch: **370 → 62 lines** (with cargo-fmt's
  forced chain splits; the logical-line count is ~45).
- `hello-events` ported: **395 → 34 lines**.
- `hello-rect` **retained in its pre-DR form**: it exercises
  `sigil-async` + `dispatch_to_owner` + cross-thread wake, none of which
  launch owns today. Adding async integration to launch is explicit
  follow-up work (Phase 08c). Until then hello-rect is the "low-level
  wiring" reference example.

### Scope intentionally excluded

- **No async runtime in `launch`.** Apps that need background tasks
  bootstrap their own `AsyncRuntime` + waker as hello-rect does.
  Phase 08c (or later) introduces `AppConfig::on_startup(Fn)` and a
  frame-tick async drain; see F-002 in DR-013.
- **No multi-window.** One `launch` call = one window. Multi-window is
  a later extension and the signature does not preclude it.
- **No hot reload / dev tools / inspector.** Product-layer work.
- **No error boundaries yet.** `PanicPolicy::Isolate` is accepted as
  the default value so the API is stable, but the isolation machinery
  ships with Phase 16 (DR-025).

### Surface stability

The `launch(config, tree) -> Result<()>` signature is expected to be
the 1.0 shape. Future extensions (async startup, multi-window,
error-boundary policies) go through `AppConfig` or new `launch_*`
helpers — never by changing `launch`'s positional args.

## Alternatives rejected

| Alternative | Reason |
|---|---|
| Chained builder `App::new(...).title(...).run()` | Pre-2023 idiom; verbose; discourages refactoring. |
| Attribute macro `#[sigil::main]` | Hides control flow; hostile to debuggers and to LSP navigation. |
| Pure function `launch(fn)` with no config | Forces title/size into the tree; breaks separation between shell config and tree. |
| Fold `launch` into `sigil-shell` | `sigil-shell` is a winit adapter; adding orchestration bloats it and makes the winit swap harder. |

## References

- [`docs/plans/phase-08a-ergonomics-pass.md`](../plans/phase-08a-ergonomics-pass.md)
- [`docs/decisions/DR-013-roadmap-reconciliation.md`](DR-013-roadmap-reconciliation.md) (F-002)
- [`crates/sigil-app/src/launch.rs`](../../crates/sigil-app/src/launch.rs)
- [`crates/sigil-app/src/config.rs`](../../crates/sigil-app/src/config.rs)
- [`crates/sigil-app/src/tree_root.rs`](../../crates/sigil-app/src/tree_root.rs)
- [`crates/sigil/src/lib.rs`](../../crates/sigil/src/lib.rs)
- [`examples/hello-style/src/main.rs`](../../examples/hello-style/src/main.rs)
- [`examples/hello-events/src/main.rs`](../../examples/hello-events/src/main.rs)

## Kill Criteria

Revisit if:

- Async integration into `launch` turns out to need a breaking signature
  change (unlikely — the `AppConfig::on_startup` escape hatch is the
  planned extension).
- Multi-window support requires a tree-per-window model that the single
  `TreeRoot` argument cannot accommodate.
- A future platform target (web / mobile) can't fit the
  `config + tree_fn -> Result` shape.
