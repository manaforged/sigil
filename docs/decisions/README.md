# Decision records

Architectural calls get a numbered record here: one file per decision,
`DR-<number>-<slug>.md`, committed. Citing a DR number in code or manifests
requires the record to exist in this repository — a citation to a document
an outsider cannot read is a defect, not a shorthand.

## Provenance of DR-001 through DR-057

DR-001 through DR-057 predate this rule. They were kept as local-only
working notes (gitignored until 2026-08-26) and were never committed; those
files no longer exist. The decisions themselves are codified in the ~180
source files that cite them — the citing comments state the rule and the
reason. Notable examples:

- `DR-005 §4` — main-thread-only reactive writes (see `crates/hal/src/scheduler.rs`).
- `DR-008` — tokio (not async-io) for `accesskit_unix`, matching the zbus
  single-runtime constraint (see the workspace `Cargo.toml`).
- `DR-047` — the `DrawPhase` write fence (see `crates/reactive`).
- `DR-048` — frame-loop phase order (see `crates/app/src/launch.rs`).
- `DR-050 §1` — `accesskit_winit` feature selection (workspace `Cargo.toml`).
- `DR-054` — the `render` feature surface for embed hosts (see `crates/sigil/Cargo.toml`).

Do not cite DR-001–057 as authority in new work; restate the rule, or write
a new record starting at the next number.

## Current records

- `DR-058-signals-over-entity.md` — why `Signal` is the author type, not an
  entity with a render method.
