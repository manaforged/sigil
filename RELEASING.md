# Releasing sigil

All `sigil-*` crates publish in lockstep from the workspace root (one version,
one tag, one command). **Nothing on this page runs without Thomas's explicit
go.** The everyday state of this repo is "everything staged up to the publish
button": the CI dry-run gate keeps it that way.

## Preconditions

- `CHANGELOG.md`: the `Unreleased` section is renamed to the version + date.
- CI green on `main`: fmt / clippy / test matrix, MSRV 1.90 leg, docs job,
  `cargo deny check`, and the publish dry-run gate.
- crates.io name preflight re-run for **every** package in the publish set.
  Names are first-come-first-served; our one-time preflight
  (2026-07-18, all `sigil-*` free) reserves nothing.
- Publishing credentials: **trusted publishing** (crates.io GitHub OIDC) is the
  target setup — no long-lived `CARGO_REGISTRY_TOKEN` anywhere. Until that is
  configured, publish from a maintainer machine with a freshly minted,
  narrowly scoped token, and revoke it afterwards.

## Version bump (lockstep)

1. `workspace.package.version = "<next>"` in the root `Cargo.toml`.
2. Every internal entry in `[workspace.dependencies]`: `version = "<next>"`.
   (Normal/build deps only — internal dev-deps stay version-less path deps so
   cargo strips them from the published manifests.)
3. `cargo check --workspace` to regenerate `Cargo.lock`; commit with the
   CHANGELOG edit.

Internal dep reqs use the plain lockstep version (e.g. `"0.1.0-alpha.1"`). A
caret requirement over a prerelease stays on that prerelease line, which is
what the lockstep policy wants; bumping means editing every req together.
Never bump a subset.

## Gate (always, before any real publish)

```bash
cargo publish --workspace --dry-run --locked --all-features
```

Then inspect the packages themselves:

```bash
cargo package --list -p <crate>   # per publish-set crate
```

Every `.crate` must contain `src/**`, `Cargo.toml`, `README.md`,
`LICENSE-MIT`, `LICENSE-APACHE` — and **no** `tests/` fixtures (the OFL font
files), golden PNGs, or bench corpora. Check compressed sizes; crates.io
enforces a 10 MB cap server-side, so a green dry-run does not prove the cap.

## Publish (go-time only, with Thomas's explicit approval)

```bash
cargo publish --workspace --locked --all-features
```

Multi-package publish (Cargo ≥ 1.90 — conveniently our MSRV) verifies the set
against a local overlay registry and uploads in dependency order. There is no
manual topological sequencing and no per-crate publish loop.

## After

- `git tag v<version>` and push the tag — this creates the baseline the
  `cargo-semver-checks` CI job compares against from then on.
- Verify the docs.rs build for `sigil-ui` (it builds with `all-features`,
  so the optional `render` surface appears).
- `CHANGELOG.md`: open a new `Unreleased` section.
