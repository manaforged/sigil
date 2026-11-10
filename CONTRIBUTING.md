# Contributing

Issues are welcome. Pull requests are by invitation: open an issue first, and send a patch only when asked.

## Build

Use the Rust 1.98.1 toolchain pinned in `rust-toolchain.toml`. The declared minimum supported Rust version is 1.95. On Linux, install the libraries that the window host links against:

```bash
sudo apt-get install libgtk-3-dev libudev-dev libasound2-dev \
  libxkbcommon-dev libxkbcommon-x11-dev libxdo-dev
```

```bash
cargo build --workspace
```

## Run the gate

Run the same gate that CI runs before you push:

```bash
scripts/gate.sh
```

CI runs each gate job on its own runner. `scripts/gate.sh --list` names the jobs, and `scripts/gate.sh test` runs one.

The gate needs `node`, `cargo-hack`, `cargo-deny`, `cargo-fuzz`, `mdbook`, and `wasm-bindgen-test-runner`. It names the install command for a missing tool. It installs the MSRV and nightly toolchains through `rustup`.

`node tools/profile.mjs report TRACE` summarizes a `SIGIL_PROFILE` trace, and `crates/sigil/perf_budgets.toml` holds the budgets the perf gate test reads.

On a machine with a GPU adapter, run the hardware tier too:

```bash
scripts/gate.sh --gpu
```

## Code shape

- Keep functions under 50 lines. Clippy enforces this.
- Do not add `//` comments. A public item may have one doc comment (`///`). `missing_docs` stays off.
- User docs live in `README.md`, `crates/sigil/README.md`, `crates/sigil/docs/`, and `docs/guide/`.
- Put tests beside the code under test. Do not inline tests in production modules.
- Search the prelude and `cargo doc -p sigil-ui --open` before you add a helper. Give each new public item a one-line doc comment in the same change.
- User code imports `sigil::prelude::*` and nothing else from this repository.
- Visual changes re-bake the goldens. Check the output at native size.

## Commit messages

Use `area: summary` as the subject, then a body:

```text
Problem: what is wrong, and who hits it.
Fix: what the change does.
Impact: what changes for a user or operator. "None" is fine.
Test: the command you ran and the line that proves it.
```

Keep the subject lowercase after the colon, imperative, and under 72 characters.
