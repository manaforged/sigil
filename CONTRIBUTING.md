# Contributing

Issues are welcome. Pull requests are by invitation: open an issue
first, and send a patch only when asked. Coding agents: [AGENTS.md](AGENTS.md).

## Build

Use the Rust 1.98.1 toolchain pinned in `rust-toolchain.toml`. The
declared minimum supported Rust version is 1.90. On Linux, install the
libraries that the desktop host links against:

```bash
sudo apt-get install libgtk-3-dev libudev-dev libasound2-dev \
  libxkbcommon-dev libxkbcommon-x11-dev libxdo-dev
```

```bash
cargo build --workspace
```

## Run the gate

Run the same checks that CI runs before you push:

```bash
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --no-fail-fast
cargo deny check
```

Install `cargo deny` once if it is missing: `cargo install cargo-deny`.

The default test run skips tests that need a GPU adapter. On a machine
with a GPU adapter, run the hardware tier too:

```bash
cargo test --workspace -- --ignored
```

## Code shape

- Keep functions under 50 lines. Clippy enforces this.
- Do not add code comments. User docs live in `README.md`,
  `crates/sigil/README.md`, `docs/api.md`, and `docs/guide/`.
- Put tests beside the code under test. Do not inline tests in
  production modules.
- User code imports `sigil::prelude::*` and nothing else from this
  repository.
- Visual changes re-bake the goldens. Check the output at native size.

## Commit messages

Use `area: summary` as the subject, then a body:

```text
Problem: what is wrong, and who hits it.
Fix: what the change does.
Impact: what changes for a user or operator. "None" is fine.
Test: the command you ran and the line that proves it.
```

Keep the subject lowercase after the colon, imperative, and under 72
characters.
