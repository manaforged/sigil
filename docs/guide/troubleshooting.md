# Troubleshooting

## The window does not open on a machine with no GPU

The window host reads `SIGIL_BACKEND` when it starts:

| Value | Renderer |
| --- | --- |
| `auto` or unset | wgpu. If the wgpu surface fails, Sigil logs a warning and uses the CPU renderer. |
| `wgpu` | wgpu only. The app fails if no adapter is available. |
| `cpu` | The CPU renderer. |

Any other value logs a warning and acts as `auto`.

The frame HUD (`SIGIL_HUD=1`) shows the active backend. A fallback shows its cause, for example `cpu (no-adapter)` or `gpu-software (no-adapter)`.

If the app still fails to start, set the CPU renderer:

```bash
SIGIL_BACKEND=cpu cargo run
```

The terminal host needs no GPU.

## Images do not show in tmux or screen

The terminal host selects an image protocol from the environment:

- Kitty protocol: Kitty, Ghostty, WezTerm, and Konsole.
- iTerm2 protocol: iTerm2 and Warp.
- Sixel: foot, contour, mlterm, mintty, rio, yaft, and any `TERM` that contains `sixel`.

If the environment names none of these, images show as sextant glyphs. Sigil does not ask the terminal: a multiplexer can answer a graphics query for an outer terminal that cannot draw images.

Inside tmux (`TMUX` is set), Sigil wraps each image sequence in the tmux passthrough form. tmux forwards it only when its `allow-passthrough` option is on:

```bash
tmux set -g allow-passthrough on
```

Inside GNU screen (`STY` is set), Sigil does not send Kitty or Sixel images.

To choose the protocol yourself, set `SIGIL_GRAPHICS` to `kitty`, `iterm2`, or `sixel`. `SIGIL_GRAPHICS=off` (or `none`) turns images off and shows glyphs. Other values are ignored.

## Colours are wrong in the terminal

The terminal host picks a colour depth once, at start:

1. If `NO_COLOR` is set and not empty, it uses no colour.
2. If `TERM` is `dumb`, it uses no colour.
3. If `COLORTERM` contains `truecolor` or `24bit`, or `TERM` ends in `-direct`, it uses 24-bit colour. A recognised terminal other than Apple Terminal also gets 24-bit colour.
4. If `TERM` contains `256`, or the terminal is Apple Terminal, it uses 256 colours.
5. If `TERM` is set, it uses 16 colours.
6. On Windows with no `TERM`, it uses 24-bit colour. Elsewhere it uses no colour.

If your terminal supports 24-bit colour but Sigil uses fewer colours, set `COLORTERM=truecolor`.

Box borders use ASCII when `TERM` is `dumb` or when the locale (`LC_ALL`, `LC_CTYPE`, or `LANG`) is not UTF-8.

## Linux: Wayland and X11

The window host uses winit, which supports both Wayland and X11.

The system clipboard uses `arboard` without its Wayland data-control feature. On a Wayland session, clipboard access goes through XWayland. If XWayland is not running, `copy` and `paste` fail.
