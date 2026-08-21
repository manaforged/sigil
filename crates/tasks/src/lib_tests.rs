// `criterion` is a dev-dep used only by `benches/async_ops.rs`. Cargo
// pulls every dev-dep into the lib-test compilation unit, which trips
// `unused_crate_dependencies` here. Alias silences the lint without
// adding a real import. Same pattern as `sigil-reactive`.
use criterion as _;
