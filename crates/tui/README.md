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

A TUI-only binary depends on `sigil-ui` with `default-features = false`
and `features = ["tui"]`, as in the sample. A direct dependency on
`sigil-tui` also works: the entry point is `sigil_tui::run`, there is no
prelude, and the OS clipboard is not installed for you.
`cargo tree -p sigil-tui` contains no `wgpu`, `winit`, or `sigil-renderer`.
