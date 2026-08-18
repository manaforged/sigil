# Sigil

Desktop UI in Rust. `sigil::launch` opens a window. `sigil::ui::Host`
embeds the same tree in another engine. The widget catalog
(`button`, `text_field`, `table`, `virtual_list`, `modal`, `sheet`,
`toast`) lives in this crate.

```bash
mdbook serve docs/book
# live catalog
cargo run -p widget-playground
```

Source will be public later. Not taking pull requests yet.

## Owned-host verification

GitHub Actions is not part of the test path. Run these gates directly on every supported native
builder before promotion:

```bash
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo check --workspace --all-targets
cargo test --workspace --no-fail-fast
cargo deny check
```

Sigil is a Rust-native desktop UI system for building serious,
data-heavy applications without retreating to a webview. It owns the
experience layer above the hard foundations: reactive state, typed node
identity, layout, rendering, accessibility, IME-ready text, headless
interaction primitives, a designed component catalog, and the app
patterns that make dense desktop software feel composed instead of
assembled from parts.

## Book

The author guide and the architecture chapters live in
[`docs/book/`](docs/book/):

```bash
mdbook serve docs/book
```

Signals, the owner tree, the frame loop, layout, the command scene,
`spawn`, and the crate map are in the Architecture section.

**Status:** alpha. The pieces below are implemented and tested; the API
is not yet stable.

Sigil retains its node tree across frames. A signal write marks the
nodes it touched, and both layout and paint are gated on that dirty
set, so an idle frame costs nothing and a one-node change repaints one
node. `crates/bench-parity` measures this against GPUI on shared
workloads and refuses to print a comparison unless both sides report
the same mode, host, and equivalence verdicts.

The layout engine is owned (DR-041) — flexbox, block, and grid, with
content-hash cache keys; Taffy remains only as a bench corpus. Text is
owned end to end (DR-044): HarfRust shaping, skrifa metrics, bidi, line
breaking, font fallback, and a paragraph cache. Glyphs and SVG icons
share one GPU pipeline through a Slug fragment rasteriser (DR-014,
DR-032), so mixed text and icon batches need no pipeline switch.

Frame phases are a typed state machine (DR-047): writes during paint
panic in debug and defer to the next effect phase in release.
Accessibility lives in the node type rather than beside it, and
AccessKit, IME, drag-and-drop, and animation are wired.

Only `sigil-renderer` depends on `wgpu` (the facade carries it as a
dev-dependency for GPU tests). Everything else depends on the
`sigil-hal` traits, which is what lets `sigil::ui::Host` embed the same
tree in another engine's frame loop, and `sigil-tui` render it to a
terminal.

## Design plan

The roadmap reconciliation lives in
[`DR-013`](docs/decisions/DR-013-roadmap-reconciliation.md). Repo-local
execution plans live in [`docs/plans/`](docs/plans/), and decision
records live in [`docs/decisions/`](docs/decisions/).

## Workspace Layout

```text
crates/
  hal/ (sigil-hal)              Hardware abstraction layer: traits for Renderer,
                               Shell, Layout, TextEngine, A11yBridge, Scheduler.
                               Every downstream crate depends on these traits,
                               never on upstream crates directly. no_std.
  reactive/ (sigil-reactive)   Main-thread signal/effect owner tree.
  kernel/ (sigil-kernel)       Stable node identity, arena storage, Component,
                               Semantics, typed extensions, and builders.
  layout/ (sigil-layout)       Owned incremental layout engine.
                               Mirrors sigil-kernel subtrees, computes layout,
                               publishes cached results. Taffy corpus is
                               benches only.
  events/ (sigil-events)       Input routing, z-order hit-testing, focus
                               traversal, pointer capture, and handler builders.
  style/ (sigil-style)         StyleAtom vocabulary, Theme, StyleEngine,
                               classes! resolver, Style struct API, and
                               style_reactive signal binding.
  style-macros/ (sigil-style-macros)
                               Proc-macro crate backing classes!; a phf
                               structural-token table is generated at build time
                               from grammar/structural.toml.
  tasks/ (sigil-tasks)         Tokio runtime and owner-targeted main-thread
                               dispatch. (Was sigil-async; `async` is reserved.)
  text/ (sigil-text)           Owned text pipeline: shaping, measurement,
                               paragraph cache, font fallback.
  renderer/ (sigil-renderer)   wgpu-backed renderer adapter. Includes
                               WgpuRenderer::render_scene default pipeline (DR-017).
  shell/ (sigil-shell)         winit shell adapter.
  a11y/ (sigil-a11y)           AccessKit bridge over sigil-kernel semantics.
  actions/ + actions_macros/   Typed actions + #[derive(Action)].
  animation/ + animation_runtime/
                               Animation primitives + runtime.
  icons/ (sigil-icons)         SVG icon baking (usvg → sprites).
  app/ (sigil-app)             App orchestrator: sigil::launch, AppConfig,
                               TreeRoot, frame loop, DPI-aware logical sizing,
                               bulletproof defaults (DR-016, DR-018).
  profile/ (sigil-profile)     Frame profiler: SIGIL_PROFILE, Perfetto JSON,
                               phase table. Wired in launch; see docs/book/05-profile.md.
  sigil/ (sigil-ui)            User-facing facade with prelude — the package is
                               `sigil-ui` (the `sigil` name was taken), the
                               library is `sigil`: `use sigil::prelude::*;`
                               is the one import line user apps need.
  ide-core/                    IDE primitives (publish = false).
  engine_adapter/               the host engine-engine host trait (publish = false).
  bench-parity*/               Benchmark harnesses (publish = false).

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
  widget-playground/ The widget catalog, live.
  data-table/        1:1 of GPUI 0.2.2 `examples/data_table.rs`
                     (10k quotes, 24 columns, `virtual_list`).
  */                 Dogfood apps (csv-viewer, chat-client, kanban-board,
                     api-explorer, file-browser, …) — each ships a
                     FRICTION.md log that drives API ergonomics work.

docs/
  decisions/         Architecture decision records (DR-000 onward, 47+).
  plans/             Repo-local execution plans tied to accepted DRs.
```

## Verification

```bash
cargo fmt --check
cargo check --workspace --all-targets
cargo clippy --workspace --all-targets -- -D warnings -W clippy::pedantic -W clippy::nursery
cargo test --workspace            # unit + integration + doctests
cargo deny check
RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps
cargo publish --workspace --dry-run --locked --all-features   # packaging gate
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

Run the gates with plain `cargo`, scoped to the packages your diff
touches (`cargo check -p <pkg>`, `cargo test -p <pkg>`). See
[`RELEASING.md`](RELEASING.md) for the publish procedure (nothing publishes
without an explicit go).

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

Every push and PR runs:

- **`{ubuntu, macos, windows} × stable`:** `cargo fmt --check`,
  `cargo clippy --workspace --all-targets -- -D warnings`,
  `cargo check --workspace --all-targets`, `cargo test --workspace`
  (doctests on Linux/macOS; the Windows leg skips them pending re-verification
  of the historical Windows-msvc merged-doctest issue), and `cargo deny check`
  for licenses, advisories, bans, and sources.
- **`ubuntu × 1.90` (MSRV):** `cargo check --workspace --all-targets --locked
  --all-features` — check-only; fmt/clippy stay on stable to avoid lint
  version-skew.
- **docs:** `cargo doc --workspace --no-deps --all-features` with
  `RUSTDOCFLAGS="-D warnings"` (all-features matches the docs.rs build of the
  facade's feature-gated `render` surface).
- **publish dry-run:** `cargo publish --workspace --dry-run --locked
  --all-features` — verifies the publish set against a local overlay registry;
  uploads nothing.
- **semver-checks:** `cargo semver-checks` against the latest `v*` tag once one
  exists (skips cleanly pre-first-publish).

## License

Dual-licensed under MIT or Apache-2.0, at your option.
