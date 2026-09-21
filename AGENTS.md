# AGENTS.md

Sigil is a Rust UI library. One widget tree, three hosts: `sigil::launch`
(window), `sigil::Host` (embed), `sigil_tui::run` (terminal).

This file is for an agent that edits this repository. Writing an app
that depends on Sigil: read [docs/api.md](docs/api.md).

Humans: [README](README.md). Patching this repo: [CONTRIBUTING.md](CONTRIBUTING.md).

## Layout

- `crates/sigil` — package `sigil-gui`, library `sigil`
- `crates/tui` — package `sigil-tui`
- `crates/sign` — package `sigil-sign`, binary `sigil-pack`
- `crates/sigil/examples` — hello-counter, hello-host, widget demos
- `docs` — the book (`mdbook serve docs`): `docs/api.md`, `docs/guide`, `docs/guide/arch`, `docs/guide/decisions`

## Commands

MSRV 1.90. This repo pins 1.98.1. Linux desktop needs GTK3, udev, ALSA,
xkbcommon, and xdo headers.

```bash
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --no-fail-fast
cargo deny check
```

GPU tests are ignored by default. With an adapter:

```bash
SIGIL_REQUIRE_GOLDENS=1 cargo test --workspace -- --ignored
```

`SIGIL_BACKEND=cpu` selects the software renderer.

## Conventions

- No comments in source. Docs go in README, rustdoc, `docs/api.md`, and `docs/guide`.
- Tests sit next to the code as `*_tests.rs`.
- Clippy caps functions at 50 lines.
- `unsafe` is denied except `crates/sigil/src/app/menu.rs`.
- Do not add a crate when std or an existing dependency already does the job.
- Pin git dependencies to a rev. Do not track `main`.
- Commits: `area: summary` then `Problem:`, `Fix:`, `Impact:`, `Test:`.

## Do not

- Add Copilot instruction files or a second agent guide.
- Put renderer showcases or profiling tools back in `examples/`.
- Feed untrusted markdown, HTML, or SVG into the parsers (no fuzz suite yet).
