# Bundled fonts

The fonts in the table below are licensed under the SIL Open Font License, Version 1.1 (see `LICENSE-OFL.txt`). `SigilSymbols.ttf` is under the Bitstream Vera and Arev licenses (see `LICENSE-DejaVu.txt`). Embedding them here does not extend Sigil's own license to them; each remains under its own license.

| File | Family | Copyright | Reserved Font Name |
|------|--------|-----------|--------------------|
| `FiraSans-Regular.ttf` | Fira Sans | Copyright © 2014, The Mozilla Foundation and Telefonica S.A. | "Fira" |
| `FiraSans-Medium.ttf` | Fira Sans Medium | Copyright © 2012-2015, The Mozilla Foundation and Telefonica S.A. | "Fira" |
| `FiraSans-SemiBold.ttf` | Fira Sans SemiBold | Copyright © 2012-2016, The Mozilla Foundation and Telefonica S.A. | "Fira" |
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

## Sigil Symbols

`SigilSymbols.ttf` is a subset of DejaVu Sans 2.37 (<https://github.com/dejavu-fonts/dejavu-fonts>) reduced to the symbol ranges a user interface draws: arrows (U+2190–21FF), miscellaneous technical keys such as ⌘ ⌥ ⌃ (U+2300–23FF), geometric shapes (U+25A0–25FF), miscellaneous symbols (U+2600–26FF), dingbats such as ✓ and ✕ (U+2700–27BF), U+2B00–2BFF, ×, •, and …. It is the fallback the text engine reaches for after the sans and mono faces. Fonts are (c) Bitstream; DejaVu changes are in the public domain; glyphs imported from Arev are (c) Tavmjong Bah. As the Bitstream Vera license requires for a modified font, the subset is renamed "Sigil Symbols" and does not use the words "Bitstream" or "Vera". Produced with `pyftsubset` (fontTools) and the name table rewritten with fontTools.
