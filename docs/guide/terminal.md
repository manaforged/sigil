# Terminal host

The `terminal` feature of `sigil-ui` paints the same widget tree on a
character grid. Without `desktop` it does not build wgpu or winit.

## Install

```bash
cargo add sigil-ui --no-default-features --features terminal
```

`--no-default-features` keeps the GPU stack out. `sigil::prelude` is
the same.

## Usage

```rust,no_run
use sigil::prelude::*;

fn main() -> sigil::AppResult<()> {
    sigil::run(AppConfig::new("Sigil - hello-terminal").terminal(), app_tree)
}

fn app_tree(cx: &mut Cx) -> NodeId {
    cx.add(text("hello"))
}
```

`sigil::run` with `.terminal()` opens the terminal, runs the frame loop,
and restores the screen on exit. `sigil::terminal::run_with` is the
same loop with `Hooks`: an external event pump and an after-frame
callback. The terminal host installs an OSC
52 clipboard so widget copy/cut reaches the host terminal's clipboard.

## Capabilities

`run` detects one `sigil::terminal::Caps` when it starts. It reads the
environment first. If the environment does not name the terminal, `run`
sends a Kitty graphics query, a DEC 2026 mode query, a Kitty keyboard
query, and DA1, and waits up to 150 ms for the reply. `Caps::detect`
runs the environment step alone.

| Capability | Environment | Query | Fallback |
| --- | --- | --- | --- |
| Color depth | `NO_COLOR`, `COLORTERM`, `TERM_PROGRAM`, `TERM` | none | truecolor, then 256, then 16, then no color |
| Graphics | `SIGIL_GRAPHICS`, `TERM`, `TERM_PROGRAM`, `KITTY_WINDOW_ID` | Kitty graphics query, DA1 attribute 4 (Sixel) | sextant glyphs |
| Multiplexer | `TMUX`, `STY` | none | direct output |
| Keyboard | known terminal | `CSI ? u` | legacy keys, no enhancement flags |
| Synchronized output | known terminal | `CSI ? 2026 $ p` | frames without DEC 2026 |
| Hyperlinks | known terminal other than Apple Terminal | none | off |
| Underline styles | known terminal other than Apple Terminal, Alacritty, and Windows Terminal | none | off |
| Borders | `TERM`, `LC_ALL`, `LC_CTYPE`, `LANG` | none | rounded, then light, then ASCII |
| Cursor shape | known terminal other than Apple Terminal | none | block cursor |
| Clipboard | macOS without `SSH_TTY` | none | OSC 52 |

Color depth:

- `NO_COLOR` set to a non-empty value, or `TERM=dumb`, selects no color.
  Text keeps bold and italic.
- `COLORTERM=truecolor` or `24bit`, a `TERM` that ends in `-direct`, or a
  known terminal other than Apple Terminal selects truecolor.
- A `TERM` that contains `256`, or Apple Terminal, selects 256 colors.
  An exact color maps to the nearest xterm cube or gray entry.
- Any other `TERM` selects 16 colors. An exact color maps to the nearest
  ANSI color.

Known terminals are Kitty, Ghostty, WezTerm, iTerm2, foot, Alacritty,
Contour, Windows Terminal, VTE terminals, and Apple Terminal.

Borders:

- `TERM=dumb`, or a locale that is not UTF-8, selects ASCII (`+`, `-`,
  `|`).
- `TERM=linux` selects light box-drawing glyphs.
- Any other terminal selects rounded glyphs for a rounded box, heavy
  glyphs for a border of 2 px or more, and light glyphs otherwise.

## Scene mapping

The terminal rasteriser draws the same `CommandScene` as the window
renderer. Each scene command has one terminal form and one fallback.

| Scene command | Terminal form | Fallback |
| --- | --- | --- |
| Solid fill, rounded rect | Cell background. An edge that ends inside a cell uses a half-block glyph (`▀`, `▄`, `▌`, `▐`). | truecolor, then 256, then 16 colors |
| Border | Box-drawing glyphs. A fill inside the bordered box keeps the border cells. A divider that meets a side becomes a tee (`├`, `┤`). | heavy or rounded, then light, then ASCII |
| Text run | Bold (weight 700 or more), dim (weight below 400), italic, underline, strikethrough | The terminal drops what it does not support |
| Underline styles | Curly underline (`SGR 4:3`) and underline color (`SGR 58`) | Plain underline (`SGR 4`) |
| Link | OSC 8 hyperlink for a text node whose accessible description is a URL | Underlined text; the URL stays in the accessibility tree |
| Image | Kitty graphics, then iTerm2, then Sixel | Sextant glyphs. A transparent image draws only its covered sextants over the cell background. A chevron icon of at most 2×2 cells becomes one glyph (`▸`, `◂`, `▾`, `▴`). |
| Opacity, translucent layer | Blend against the cell below with the active theme | none |
| Popup, menu, tooltip, toast | Drawn last, above clips | none |
| Shadow, blur, gradient | Omitted. A popup keeps its border. | none |
| Caret | Terminal cursor: a steady bar at the caret cell | Block cursor |
| Selection | Selection background blended over the text | none |

A caret is a filled box of at most half a cell wide inside the focused
node. The rasteriser hides the terminal cursor when no caret is visible.

The writer compares cells after it maps their colors to the detected
color depth. A row whose output bytes do not change is not written
again, so a color transition at 256 colors sends a row only when a cell
changes on screen.

`run` restores the terminal on exit, on `SIGTERM`, `SIGHUP`, and
`SIGINT`, and on a panic on any thread. When no input arrives, no state
is dirty, and no interval or animation is due, `run` does not build a
frame. It waits for input until the next interval is due.

## Limits

- No IME.
- No shadows, blur, or gradients.
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
  `KeyPhase::Up` and repeat sets `repeat` on `KeyEvent`. Insert, lock,
  media, menu, and modifier keys map to their named `Key`.
- Mouse input uses SGR mouse mode with any-motion tracking. A click,
  a drag, and motion with no button held become `ShellEvent::Pointer`
  events at the cell centre, so hover styles and the `drag_drop`
  helpers work as in a window. A wheel notch becomes a
  `ShellEvent::Scroll` of three lines. The terminal reports motion
  only when the pointer enters a new cell.
- Focus in and out become `WindowEvent::Focused`, and a resize
  becomes `WindowEvent::Resized` in cell-pixel units. Both reach
  `EventRouter` like the window host events. File drops from the
  desktop are window-only.
- `run` installs an OSC 52 clipboard: copy and cut emit
  `ESC ] 52 ; c ; <base64> BEL`, wrapped in DCS passthrough under tmux
  or screen. Clipboard reads are not wired; paste reaches widgets
  through bracketed paste. The `clipboard` feature of `sigil-ui` and
  an OS clipboard are not needed.
- `SIGIL_TUI_TRANSCRIPT=1` turns on a transcript stream on stderr:
  each flushed frame writes the `sigil::a11y::snapshot` of the
  accessibility tree when the tree changed, else the text rows that
  changed. This is the screen-reader and agent-transcript surface.
- The cell grid changes the paint, not the tree. The widgets, events,
  and signals are the same as the desktop host.
- Write ordinary pixel geometry. Layout snaps it to the cell grid: a
  size rounds to whole cells, a padding, gap, or margin rounds to the
  nearest cell, and each box edge rounds to the nearest cell line, so
  flex siblings keep their gaps. A border reserves a whole cell so its
  box glyph does not overwrite content. A box with a declared height of
  one cell keeps the one-row form: its border does not reserve a cell
  and its label sits on the border row. A box thinner than one cell
  paints as a rule glyph instead of a block, so a 1 px divider stays a
  line.
- The `terminal` feature enables `image-decode`, so a markdown
  `data:` image renders. The image keeps its width in cells and takes
  half its height in rows, because a cell is twice as tall as it is
  wide.

## Check the tree

```bash
cargo tree -p sigil-ui --no-default-features --features terminal
```

The output contains no `wgpu` and no `winit`.
