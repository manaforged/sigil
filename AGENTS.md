# AGENTS.md

Sigil is a Rust UI library. One widget tree, three hosts: `sigil::run`
(window, or terminal with `.terminal()`), and `sigil::Host` (embed).

This file is for an agent that edits this repository. Writing an app
that depends on Sigil: read [docs/api.md](docs/api.md).

Humans: [README](README.md). Patching this repo: [CONTRIBUTING.md](CONTRIBUTING.md).

## Architecture invariants

- Shared API: [docs/api.md](docs/api.md). Search it before you add a helper. A new public item gets a row there in the same change.
- One framework: one `Host::frame`, one `CommandScene`, one entry point `sigil::run`. Do not add a second host loop or entry point.
- The terminal host renders the same widget tree with the same features as the window host.
- A defect that a dependent app hits in Sigil is fixed here, not worked around in the app.
- `mermaid` and `html` are product widgets. Fix them; do not cut or feature-gate them.
- Nothing is published yet. Public API breaks are allowed before v0.1.0.

## Layout

- `crates/sigil` — package `sigil-ui`, library `sigil`; terminal host in `src/terminal` (feature `terminal`)
- `crates/sign` — package `sigil-sign`, binary `sigil-pack`
- `crates/kittest` — package `sigil-kittest`, PNG golden harness for dependents
- `crates/sigil/examples` — hello-counter, hello-host, widget demos
- `docs` — the book (`mdbook serve docs`): `docs/api.md`, `docs/guide`, `docs/guide/arch`, `docs/guide/decisions`

## Commands

MSRV 1.90. This repo pins 1.98.1. Linux desktop needs GTK3, udev, ALSA,
xkbcommon, and xdo headers.

```bash
cargo fmt --check
cargo clippy --workspace --all-targets --features sigil-ui/terminal -- -D warnings
cargo clippy -p sigil-ui --lib --target wasm32-unknown-unknown -- -D warnings
cargo test --workspace --features sigil-ui/terminal --no-fail-fast
cargo deny check
```

GPU tests are ignored by default. With an adapter:

```bash
cargo test --workspace -- --ignored
```

`SIGIL_BACKEND=cpu` selects the software renderer.

## Conventions

- No comments in source, rustdoc included. No `missing_docs` lint. Docs go in README, `docs/api.md`, and `docs/guide`.
- Tests sit next to the code as `*_tests.rs`.
- Clippy caps functions at 50 lines.
- `unsafe` is denied except `crates/sigil/src/app/menu.rs`.
- Do not add a crate when std or an existing dependency already does the job.
- First-party git dependencies track `main`; `Cargo.lock` holds the commit. Move it with `cargo update -p <crate>`.
- Commits: `area: summary` then `Problem:`, `Fix:`, `Impact:`, `Test:`.

## Do not

- Add Copilot instruction files or a second agent guide.
- Put renderer showcases or profiling tools back in `examples/`.
- Feed untrusted markdown, HTML, or SVG into the parsers (no fuzz suite yet).
