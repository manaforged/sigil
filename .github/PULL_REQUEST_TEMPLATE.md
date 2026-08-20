## What

<!-- one paragraph: what changes and why. -->

## Verification

- [ ] `cargo fmt --check`
- [ ] `cargo clippy --workspace --all-targets -- -D warnings`
- [ ] `cargo check --workspace --all-targets`
- [ ] `cargo test --workspace`
- [ ] Public API changes: rustdoc updated; `cargo doc --workspace --no-deps` clean
- [ ] Visual changes: goldens regenerated + cross-vendor visual review (see `the contributor guide`)
- [ ] Style/parser changes: `trybuild` expectations regenerated
