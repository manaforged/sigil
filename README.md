# Sigil

Sigil is a Rust-native desktop UI system for building serious,
data-heavy applications without retreating to a webview. It owns the
experience layer above the hard foundations: reactive state, typed node
identity, layout, rendering, accessibility, IME-ready text, headless
interaction primitives, a designed component catalog, and the app
patterns that make dense desktop software feel composed instead of
assembled from parts.

**Status:** `sigil::launch` landed (DR-016) and Sigil is DPI-aware
(DR-018). The workspace has a headless
node tree with stable AccessKit-compatible IDs, a Taffy-backed layout
adapter, an input/focus router over shell-delivered pointer, keyboard,
IME, scroll, and lifecycle events, the DR-006 three-layer style API
(`classes!` macro + `Style` + `style_reactive`), and a one-call `sigil::launch`
entry point (DR-016) with a default scene-paint pipeline (DR-017).
`hello-events` is now 34 lines; `hello-style` 62.

## Design plan

The roadmap reconciliation lives in
[`DR-013`](docs/decisions/DR-013-roadmap-reconciliation.md). Repo-local
execution plans live in [`docs/plans/`](docs/plans/), and decision
records live in [`docs/decisions/`](docs/decisions/).

## Workspace Layout

```text
crates/
  sigil-hal/            Hardware abstraction layer: traits for Renderer,
                     Shell, Layout, TextEngine, A11yBridge, Scheduler.
                     Every downstream crate depends on these traits,
                     never on upstream crates directly. Scale and
                     logical-size primitives live here.
  sigil-reactive/       Main-thread signal/effect owner tree.
  sigil-kernel/           Stable node identity, arena storage, Component,
                     Semantics, typed extensions, and builders.
  sigil-layout/         Taffy-backed layout adapter. Mirrors sigil-kernel
                     subtrees, computes layout, and attaches results.
  sigil-events/         Input routing, z-order hit-testing, focus
                     traversal, pointer capture, and handler builders.
  sigil-style/          StyleAtom vocabulary, Theme, StyleEngine,
                     classes! resolver, Style struct API, and
                     style_reactive signal binding.
  sigil-style-macros/   Proc-macro crate backing classes!; a phf
                     structural-token table is generated at build time
                     from grammar/structural.toml.
  sigil-async/          Tokio runtime and owner-targeted main-thread dispatch.
  sigil-renderer/       wgpu-backed renderer adapter. Includes
                     WgpuRenderer::render_scene default pipeline (DR-017).
  sigil-shell/          winit shell adapter.
  sigil-app/            App orchestrator: sigil::launch, AppConfig,
                     TreeRoot, frame loop, DPI-aware logical sizing,
                     bulletproof defaults (DR-016, DR-018).
  sigil/                User-facing facade with prelude. `use sigil::prelude::*;`
                     is the one import line user apps need.

examples/
  hello-counter/     Minimum viable reactive UI on `sigil::launch`:
                     a signal-backed counter with a primary button
                     and a derived label. ~20 lines of user code.
  hello-events/      Window proof app using `sigil::launch`: click
                     toggles the surface tint, Tab cycles focus, Esc
                     quits. 34 lines of user code.
  hello-style/       Window proof app using `sigil::launch`: two
                     static panes + a hover-reactive pane driven by
                     style_reactive. 62 lines.
  hello-tree/        Headless proof app: builds a `sigil-kernel` node tree
                     and prints its shape.
  hello-layout/      Headless proof app: computes rectangles for a
                     `sigil-kernel` tree and prints them.
  internals/
    hello-rect/      Direct kernel + scene-command surface; predates
                     `sigil::launch`. Advanced reference — opens a
                     window, clears to a colour, cycles three colours
                     on a timer, and exits cleanly.

docs/
  decisions/         Architecture decision records (DR-000..DR-018).
  plans/             Repo-local execution plans tied to accepted DRs.
```

## Verification

```bash
cargo fmt --check
cargo check --workspace --all-targets
cargo clippy --workspace --all-targets -- -D warnings -W clippy::pedantic -W clippy::nursery
cargo test --workspace
cargo run -p hello-counter
cargo run -p hello-events
cargo run -p hello-style
cargo run -p hello-tree
cargo run -p hello-layout
cargo run -p hello-rect
cargo bench -p sigil-kernel
cargo bench -p sigil-layout
cargo bench -p sigil-events
cargo bench -p sigil-style
```

All user-facing sizes and positions are logical pixels; the shell and
launch layers convert to physical pixels at the winit/wgpu boundary.
`cargo run -p hello-rect` should open a window, clear the surface to a
colour, cycle colours on a timer, and close cleanly when the window is
closed. `cargo run -p hello-tree` should print a deterministic headless
tree outline. `cargo run -p hello-layout` should print deterministic
computed rectangles. `cargo run -p hello-events` should open a window,
toggle colour on click, move a visible focus ring with Tab, and exit on
Escape. `cargo run -p hello-style` should open a window with three panes
— a static-styled pane, a hover-reactive pane, and a merge-override
pane demonstrating last-wins atom resolution — and exit on Escape.

## Principles

- **Lean:** small files, one concern per module, no god objects, no
  speculative abstractions. Soft cap around 400 LoC per file.
- **Honest:** no feature stubs, no half-baked functions, no "TODO" shims
  landed on main. If it is not done, it does not merge.
- **Accessible from day one:** semantic identity lives on the node model
  from the first primitive, not Phase 9.
- **Verification honesty:** claims of "tested" or "verified" require real
  evidence. Roundtrip tests prove nothing.

## CI

[![CI](https://github.com/ThomGardiner/sigil/actions/workflows/ci.yml/badge.svg)](https://github.com/ThomGardiner/sigil/actions/workflows/ci.yml)

Every push and PR runs on `{ubuntu, macos, windows} x {stable, 1.90 MSRV}`:
`cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`,
`cargo check --workspace --all-targets`, `cargo test --workspace`, and
`cargo build -p hello-rect`. Ubuntu stable additionally runs a
`sigil-reactive` bench smoke test and `cargo deny check` for licenses,
advisories, bans, and sources.

## License

Dual-licensed under MIT or Apache-2.0, at your option.
