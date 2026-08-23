# Contributing

Sigil is MIT-licensed. Copyright stays with the author.

**Issues yes. Unsolicited pull requests no.** Open a GitHub issue for
bugs and questions. If maintainers want a patch, they will ask.

The contracts below exist so parallel agents (human and AI) can work
the repo without corrupting each other.

## Ground rules

- **Branch conventions:** `main` is trunk. `swarm-*` / `wip/*` / `agent-*`
  branches activate the repo's swarm hooks (auto-commit, fast-check, stop-guard).
- **File caps:** ≤ 400 LoC per `.rs` file, ≤ 50 lines per function, ≤ 5 args per
  function; ≤ 250 LoC per widget file under `crates/widgets/src/`.
  CI-enforced via clippy thresholds in `clippy.toml`.
- **Hygiene contract:** no `unwrap`/`panic`/indexing in library code, typed
  error enums (no thiserror/anyhow in the workspace), `unsafe` denied
  workspace-wide.
- **Determinism:** same seed must produce same output across runs.
- **Facade rule:** user-facing API must read cleanly via `sigil::prelude::*`
  only — no consumer imports of internal `sigil-*` leaf crates.
- **Honesty:** no feature stubs, no half-landed functions, no "TODO" shims on
  `main`. Claims of "tested" require real evidence.

## Verification (required before a change lands)

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

This repo builds with plain `cargo`. Scope each command to the packages your
diff touches (`cargo check -p <pkg>`, `cargo test -p <pkg>`). When several
agents work the same repo, give each one its own git worktree
(`scripts/agent-hooks/agent-worktree.sh`) rather than sharing a checkout.

## Plans and decisions

Non-trivial work starts as a date-prefixed plan and architectural calls are
recorded as numbered decision records. Both are internal working documents
— they live in the maintainers' local trees, not in the published repo.
