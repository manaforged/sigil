# Decision records

Put each architectural decision in a numbered record here: one file per
decision, named `DR-<number>-<slug>.md`, committed to the repository.
If code or a manifest cites a DR number, the record must exist here. A
citation to a document that an outsider cannot read is a defect, not a
shorthand.

## Provenance of DR-001 through DR-057

DR-001 through DR-057 predate this rule. They were local-only working
notes and were never committed; those files no longer exist. The
decisions themselves are codified in the source files that cite them.
Each citing comment states the rule and the reason. Notable examples:

- `DR-005 section 4` — main-thread-only reactive writes (see
  `crates/sigil/src/hal/scheduler.rs`).
- `DR-008` — tokio (not async-io) for `accesskit_unix`, matching the
  zbus single-runtime constraint (see the workspace `Cargo.toml`).
- `DR-047` — the `DrawPhase` write fence (see
  `crates/sigil/src/reactive`).
- `DR-048` — frame-loop phase order (see
  `crates/sigil/src/app/launch.rs`).
- `DR-050 section 1` — `accesskit_winit` feature selection (workspace
  `Cargo.toml`).
- `DR-054` — the `render` feature surface for embed hosts (see
  `crates/sigil/Cargo.toml`).

Do not cite DR-001 through DR-057 as authority in new work. Restate the
rule, or write a new record starting at the next number.

## Current records

- `DR-058-signals-over-entity.md` — why `Signal` is the author type,
  not an entity with a render method.
