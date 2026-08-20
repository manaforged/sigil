# sigil-hal

Hardware abstraction layer. Every downstream crate in the workspace
depends on the traits defined here, never on upstream crates like
`wgpu`, `winit`, or `accesskit` directly.

## Traits

| Trait | Backed by |
| --- | --- |
| `Renderer` | `wgpu` |
| `Shell` | `winit` |
| `TextEngine` | `sigil-text` (owned pipeline) |
| `A11yBridge` | `accesskit` |
| `Scheduler` | `tokio` |

Layout has no HAL trait — the owned engine in `sigil-layout` consumes
the plain constraint types (`Dim`, `Size`) defined here.

## Status

The trait shapes here harden as their real adapters land. Until then,
treat every provisional method signature as subject to change; the
documented invariants are the load-bearing part.
