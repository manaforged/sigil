// W5.2 — `sigil-reactive` is a dev-dep used only by `tests/reactive_flow.rs`.
// Phase 1.3.C — `sigil` is a dev-dep used only by `tests/live_regions.rs`.
// RC-focus finding 1 — `layout` is a dev-dep used only by
// `tests/bounds_projection.rs`. Cargo's lib-test target pulls all
// dev-deps into the lib compilation unit, which trips
// `unused_crate_dependencies` here. Aliases silence the lint without
// adding a real import.
use sigil as _;
use sigil_layout as _;
use sigil_reactive as _;
