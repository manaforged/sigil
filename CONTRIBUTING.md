# Contributing

Sigil is an internal-origin the author project in the process of opening up.
Bug reports and well-scoped PRs are welcome; expect maintainers to be opinionated
about the contracts below — they exist so many parallel agents (human and AI) can
work the repo without corrupting each other.

## Ground rules

- **Branch conventions:** `main` is trunk. `swarm-*` / `wip/*` / `agent-*`
  branches activate the repo's swarm hooks (auto-commit, fast-check, stop-guard).
- **File caps:** ≤ 400 LoC per `.rs` file, ≤ 50 lines per function, ≤ 5 args per
  function; ≤ 250 LoC per widget file under `crates/sigil/src/widgets/`.
  CI-enforced via clippy thresholds in `clippy.toml`.
- **Hygiene contract:** no `unwrap`/`panic`/indexing in library code, typed
  error enums (no thiserror/anyhow in the workspace), `unsafe` denied
  workspace-wide.
- **Determinism:** same seed must produce same output across runs.
- **Facade rule:** user-facing API must read cleanly via `sigil::prelude::*`
  only — no consumer imports of internal `sigil-*` leaf crates.
- **Honesty:** no feature stubs, no half-landed functions, no "TODO" shims on
  `main`. Claims of "tested" require real evidence.

## Verification (required on every PR)

```bash
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo check --workspace --all-targets
cargo test --workspace
cargo deny check
```

Visual changes additionally require regenerated goldens and the cross-vendor
visual review (see `the contributor guide` → "Visual quality gates").

## Coordinated builds

This repo builds through `the build router` (see `the contributor guide` → "the build router: build and
coordination contract"): `the build router check` / `the build router test` route to the packages
affected by your diff. Claim scopes before large sweeps
(`the build router claim --agent <id> --task "<what>" <paths>`).

## Plans and decisions

Non-trivial work starts as a plan in `docs/plans/` (date-prefixed) and
architectural calls are recorded as DRs in `docs/decisions/`.
