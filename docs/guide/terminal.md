# Terminal host

The `sigil-tui` crate paints the same widget tree on a character grid.
It does not build wgpu or winit.

## Install

Sigil is not published on crates.io. The repository is currently private;
you need GitHub access and Git authentication to use this dependency.

```bash
cargo add sigil-tui --git https://github.com/manaforged/sigil --rev <sha>
cargo add sigil-gui --git https://github.com/manaforged/sigil --rev <sha> --no-default-features
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
the screen on exit. `sigil-tui` has no prelude. `run` installs an OSC
52 clipboard so widget copy/cut reaches the host terminal's clipboard.

## Limits

- No IME.
- No rounded corners and no shadows.
- Images render through the Kitty, iTerm2, or Sixel graphics protocol
  when the terminal advertises one (`TERM`, `KITTY_WINDOW_ID`, or
  `TERM_PROGRAM`). Detection order is Kitty, then iTerm2, then Sixel;
  detected terminals include Kitty, Ghostty, WezTerm, Konsole, iTerm2,
  Warp, foot, Contour, mlterm, mintty, and yaft. Inside tmux or GNU
  screen a detected protocol's sequences are wrapped in DCS
  passthrough instead of disabling images. `SIGIL_GRAPHICS=kitty`,
  `=iterm2`, or `=sixel` forces a protocol and `SIGIL_GRAPHICS=off`
  disables images. Other terminals show the image as sextant glyphs —
  six pixels per cell — with per-cell foreground and background
  colors. Sixel output is paletted (a bounded 216-color cube plus 24
  grays) and scaled to the cell box at a nominal 8×16-px cell.
- Keys come from crossterm: press, release, and repeat events plus
  modifiers and bracketed paste. `run` pushes the Kitty keyboard
  enhancement flags (disambiguate escape codes, report event types,
  report alternate keys) on entry and pops them on exit; terminals
  without the protocol keep reporting plain presses. Release maps to
  `KeyPhase::Up` and repeat sets `repeat` on `KeyEvent`.
- `run` installs an OSC 52 clipboard: copy and cut emit
  `ESC ] 52 ; c ; <base64> BEL`, wrapped in DCS passthrough under tmux
  or screen. Clipboard reads are not wired; paste reaches widgets
  through bracketed paste. The `clipboard` feature of `sigil-gui` and
  an OS clipboard are not needed.
- `SIGIL_TUI_TRANSCRIPT=1` turns on a transcript stream on stderr:
  each flushed frame writes the `sigil::a11y::snapshot` of the
  accessibility tree when the tree changed, else the text rows that
  changed. This is the screen-reader and agent-transcript surface.
- The cell grid changes the paint, not the tree. The widgets, events,
  and signals are the same as the desktop host.
- Write ordinary pixel geometry. Layout snaps it to the cell grid: a
  size rounds to whole cells, a padding, gap, or margin below one cell
  rounds down to zero, and a border reserves a whole cell so its box
  glyph does not overwrite content. A box thinner than one cell paints
  as a rule glyph instead of a block, so a 1 px divider stays a line.

## Check the tree

```bash
cargo tree -p sigil-tui
```

The output contains no `wgpu` and no `winit`.
