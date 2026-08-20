# Releasing sigil

All `sigil-*` crates publish in lockstep from the workspace root (one version,
one tag, one command). **Nothing on this page runs without Thomas's explicit
go.** The everyday state of this repo is "everything staged up to the publish
button": the owned-host gate (README → Owned-host verification) keeps it that
way.

## Preconditions

- `CHANGELOG.md`: the `Unreleased` section is renamed to the version + date.
- Owned-host gate green on `main`, run directly on every supported native
  builder: `cargo fmt --check`,
  `cargo clippy --workspace --all-targets -- -D warnings`,
  `cargo check --workspace --all-targets`,
  `cargo test --workspace --no-fail-fast`, `cargo deny check`, plus the MSRV
  1.90 check (`--locked --all-features`), the docs build
  (`RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps
  --all-features`), and the publish dry-run gate below.
- crates.io name preflight re-run for **every** package in the publish set.
  Names are first-come-first-served; our one-time preflight
  (2026-07-18, all `sigil-*` free) reserves nothing.
- Publishing credentials: publish from a maintainer machine with a freshly
  minted, narrowly scoped `crates.io` token, and revoke it afterwards. (The
  original trusted-publishing plan assumed GitHub Actions OIDC; Actions is no
  longer part of this repo's test path, so maintainer-machine tokens are the
  mechanism.)

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

- `git tag v<version>` and push the tag — this creates the baseline
  `cargo-semver-checks` compares against from then on (run it as part of the
  owned-host gate once a baseline exists).
- Verify the docs.rs build for `sigil-ui` (it builds with `all-features`,
  so the optional `render` surface appears).
- `CHANGELOG.md`: open a new `Unreleased` section.
