# sigil-tui

Terminal host for Sigil. It paints the same widget tree as the desktop
host onto a character grid. It does not add a second widget catalog.

## Install

Sigil is not published on crates.io. The repository is currently private;
you need GitHub access and Git authentication to use this dependency.

```bash
cargo add sigil-tui --git https://github.com/manaforged/sigil --rev <sha>
cargo add sigil-gui --git https://github.com/manaforged/sigil --rev <sha> --no-default-features
```

The second line provides `sigil::prelude`. Keep `--no-default-features`;
the desktop defaults pull wgpu and winit.

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

`sigil_tui::run` is the entry point. There is no prelude, and the OS
clipboard is not installed for you.

## Limits

- No IME, no rounded corners, and no shadows.
- Images render through the terminal graphics protocol where the
  terminal supports it (Kitty, Ghostty, WezTerm, Konsole, iTerm2, or
  Warp). Other terminals show half-block cells.
- `cargo tree -p sigil-tui` contains no `wgpu` and no `winit`.
