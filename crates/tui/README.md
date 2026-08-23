# sigil-tui

Terminal host for Sigil. Same `Host`, widget builders, and `ShellEvent`
vocabulary as desktop. This crate paints cells; it does not add a second
widget catalog.

```rust,no_run
use sigil::prelude::*;

fn main() -> sigil::AppResult<()> {
    sigil::run_tui(("hello-tui", 80, 24), app_tree)
}

fn app_tree(cx: &mut Cx) -> NodeId {
    cx.add(text("hello"))
}
```

A TUI-only binary depends on `sigil` with `default-features = false` and
`features = ["tui"]`, or on this crate directly. `cargo tree -p sigil-tui`
must not contain `wgpu`, `winit`, or `sigil-renderer`.
