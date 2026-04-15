# sigil

A production-grade, Rust-native desktop UI component library. Built on a
deliberately minimal floor (winit, wgpu, Taffy, Parley, AccessKit) and
focused on the layers every existing Rust UI framework leaves open:
accessibility from day one, real IME, a designed component system with
variant ergonomics, data-heavy patterns, and documentation.

**Status:** Phase 0 — evidence and decision records. Not usable yet.

## Design plan

The full design plan lives outside this repo at
`a local document`. Decision records
are in [`docs/decisions/`](docs/decisions/).

## Workspace layout

```
crates/
  ui-hal/            Hardware abstraction layer — traits for Renderer,
                     Shell, Layout, TextEngine, A11yBridge, Scheduler.
                     Every downstream crate depends on these traits,
                     never on upstream crates directly.

examples/
  hello-rect/        Phase 0 proof app: opens a window, clears the
                     surface to a colour, toggles through three
                     colours on click, exits cleanly. Uses winit +
                     wgpu directly (pre-HAL). A real rect-drawing
                     pipeline lands with `ui-renderer` in Phase 1.1.

docs/
  decisions/         Architecture decision records (DR-000..DR-008).
```

## Phase 0 verification

```bash
cargo check --workspace
cargo clippy --workspace -- -D warnings
cargo run -p hello-rect
```

`cargo run -p hello-rect` should open a window, clear the surface
to a colour, change colour when clicked, and close cleanly when the
window is closed. This is the Phase 0 gate.

## Principles

- **Lean:** small files, one concern per module, no god objects, no
  speculative abstractions. Soft cap ~400 LoC per file.
- **Honest:** no feature stubs, no half-baked functions, no "TODO" shims
  landed on main. If it is not done, it does not merge.
- **Accessible from day one:** semantic identity lives on the node model
  from the first primitive, not Phase 9.
- **Verification honesty:** claims of "tested" or "verified" require real
  evidence. Roundtrip tests prove nothing.

## CI

[![CI](https://github.com/ThomGardiner/sigil/actions/workflows/ci.yml/badge.svg)](https://github.com/ThomGardiner/sigil/actions/workflows/ci.yml)

Every push and PR runs on `{ubuntu, macos, windows} × {stable, 1.90 MSRV}`:
`cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`,
`cargo check --workspace --all-targets`, `cargo test --workspace`, and
`cargo build -p hello-rect`. Ubuntu stable additionally runs a
`ui-reactive` bench smoke test and `cargo deny check` for licenses,
advisories, bans, and sources.

## License

Dual-licensed under MIT or Apache-2.0, at your option.
