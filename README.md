# Sigil

Sigil is a Rust-native desktop UI system for building serious,
data-heavy applications without retreating to a webview. It owns the
experience layer above the hard foundations: reactive state, typed node
identity, layout, rendering, accessibility, IME-ready text, headless
interaction primitives, a designed component catalog, and the app
patterns that make dense desktop software feel composed instead of
assembled from parts.

**Status:** `sigil-layout` landed. The workspace has a headless node tree
with stable AccessKit-compatible IDs plus a Taffy-backed layout adapter
that writes computed rectangles back onto nodes.

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
                     never on upstream crates directly.
  sigil-reactive/       Main-thread signal/effect owner tree.
  sigil-core/           Stable node identity, arena storage, Component,
                     Semantics, typed extensions, and builders.
  sigil-layout/         Taffy-backed layout adapter. Mirrors sigil-core
                     subtrees, computes layout, and attaches results.
  sigil-async/          Tokio runtime and owner-targeted main-thread dispatch.
  sigil-renderer/       wgpu-backed renderer adapter.
  sigil-shell/          winit shell adapter.

examples/
  hello-rect/        Window proof app: opens a window, clears the
                     surface to a colour, toggles through three
                     colours on a timer, and exits cleanly.
  hello-tree/        Headless proof app: builds a `sigil-core` node tree
                     and prints its shape.
  hello-layout/      Headless proof app: computes rectangles for a
                     `sigil-core` tree and prints them.

docs/
  decisions/         Architecture decision records (DR-000..DR-013).
  plans/             Repo-local execution plans tied to accepted DRs.
```

## Verification

```bash
cargo fmt --check
cargo check --workspace --all-targets
cargo clippy --workspace --all-targets -- -D warnings -W clippy::pedantic -W clippy::nursery
cargo test --workspace
cargo run -p hello-rect
cargo run -p hello-tree
cargo run -p hello-layout
cargo bench -p sigil-core
cargo bench -p sigil-layout
```

`cargo run -p hello-rect` should open a window, clear the surface to a
colour, cycle colours on a timer, and close cleanly when the window is
closed. `cargo run -p hello-tree` should print a deterministic headless
tree outline. `cargo run -p hello-layout` should print deterministic
computed rectangles.

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
