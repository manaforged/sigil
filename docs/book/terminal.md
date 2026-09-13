# Terminal host

The `sigil-tui` crate paints the same widget tree on a character grid.
It does not build wgpu or winit.

## Install

```bash
cargo add sigil-tui
cargo add sigil-gui --no-default-features
```

`sigil-gui` with `--no-default-features` provides `sigil::prelude` and
pulls no GPU stack.

## Usage

```rust,no_run
use sigil::prelude::*;

fn main() -> sigil::AppResult<()> {
    sigil_tui::run(("Sigil - hello-tui", 80, 24), app_tree)
}

fn app_tree(cx: &mut Cx) -> NodeId {
    cx.add(text("hello"))
}
```

`sigil_tui::run` opens the terminal, runs the frame loop, and restores
the screen on exit. `sigil-tui` has no prelude and does not install the
OS clipboard.

## Limits

- No IME.
- No rounded corners and no shadows.
- Images render through the terminal graphics protocol when the
  terminal supports it (Kitty, Ghostty, WezTerm, Konsole, iTerm2, or
  Warp). Other terminals show half-block cells.
- The cell grid changes the paint, not the tree. The widgets, events,
  and signals are the same as the desktop host.

## Check the tree

```bash
cargo tree -p sigil-tui
```

The output contains no `wgpu` and no `winit`.
