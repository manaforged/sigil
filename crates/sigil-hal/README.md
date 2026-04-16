# sigil-hal

Hardware abstraction layer. Every downstream crate in the workspace
depends on the traits defined here, never on upstream crates like
`wgpu`, `winit`, `taffy`, `parley`, or `accesskit` directly.

## Traits

| Trait | Backed by | Decision record |
| --- | --- | --- |
| `Renderer` | `wgpu` | [DR-001](../../docs/decisions/DR-001-renderer-wgpu.md) |
| `Shell` | `winit` | [DR-002](../../docs/decisions/DR-002-shell-winit.md) |
| `Layout` | `taffy` | [DR-003](../../docs/decisions/DR-003-layout-taffy.md) |
| `TextEngine` | `parley` | [DR-004](../../docs/decisions/DR-004-text-parley.md) |
| `A11yBridge` | `accesskit` | [DR-008](../../docs/decisions/DR-008-a11y-accesskit.md) |
| `Scheduler` | `tokio` + MPSC | [DR-007](../../docs/decisions/DR-007-async-tokio.md) |

## Status

`Layout` became load-bearing when `sigil-layout` landed over Taffy. The
other trait shapes are still foundation sketches until their first real
adapters harden them.

## Remaining Phase 0 Traits

These trait shapes are sketches, not load-bearing contracts.

Phase 0 shipped the names, the decision records behind them, and a
minimum of shape to make the crate compile. `examples/hello-rect`
deliberately uses `winit` and `wgpu` directly so that Phase 1.1 can let
an actual adapter inform the real shape rather than designing
speculatively.

Every remaining provisional trait in this crate will change when its
first real adapter lands. Do not build downstream crates that depend on
any specific method signature there until the relevant adapter decision
is accepted. The invariants documented in the DRs are load-bearing; the
method signatures are provisional.
