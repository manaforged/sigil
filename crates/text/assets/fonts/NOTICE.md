# Bundled fonts

All fonts in this directory are licensed under the SIL Open Font License,
Version 1.1 (see `LICENSE-OFL.txt`). Their embedding here does not extend
Sigil's own license to them; each remains under the OFL.

| File | Family | Copyright | Reserved Font Name |
|------|--------|-----------|--------------------|
| `FiraSans-Regular.ttf` | Fira Sans | Copyright © 2014, The Mozilla Foundation and Telefonica S.A. | "Fira" |
| `CascadiaMono-Regular.ttf` | Cascadia Mono | Copyright © Microsoft Corporation | "Cascadia" |
| `NotoColorEmoji-subset.ttf` | Noto Color Emoji | Copyright © Google LLC | "Noto" |

## Noto Color Emoji subset

`NotoColorEmoji-subset.ttf` is a **subset** of Google's Noto Color Emoji,
reduced to a curated set of common single-scalar emoji so the color-emoji
raster path (`crate::color_glyph`) has a small, deterministic, cross-platform
glyph source. It keeps the original `CBDT`/`CBLC` color bitmap strikes.

Produced with `fonttools` (`pyftsubset`) from the upstream `NotoColorEmoji.ttf`
(<https://github.com/googlefonts/noto-emoji>). To expand or refresh the set,
re-run `pyftsubset` against the upstream font with the desired `--unicodes`
list; keep the result under the crates.io package-size budget.
