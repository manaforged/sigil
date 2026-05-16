# DR-033 — Three-Way Sprite Split (Mono / Subpixel / Poly)

**Status:** Accepted
**Date:** 2026-05-15
**Covers:** Phase R7
**Depends on:** DR-014 (Slug renderer), DR-020 (scene command buffer)
**Unlocks:** R8 (icon baker — Mono delivery), Pack 3 text_field / icon_button, subpixel text quality

## Context

Before R7, `sigil-renderer` had one rendering path for anything sourced from
a texture atlas: a single `SlugTextPipeline` that treated every atlas region
as monochrome coverage. This was adequate for the glyph proof-of-concept
(binary coverage, single colour) but produced three quality problems once
the widget set required real production text:

1. **Icons were blurry when tinted.** A monochrome glyph-style quad works for
   text because glyphs are always black-on-transparent, tinted by the font
   colour. But icons in UI systems are often tinted to match theme colours
   (grey-on-hover, accent-on-active). Subpixel positioning for tinted icon
   coverage requires a different blend equation than full-colour assets.

2. **Subpixel text was absent.** LCD subpixel anti-aliasing routes RGB
   channels through *separate* winding queries at fractional pixel offsets
   (R, G, B at 0, 1/3, 2/3 pixel). A single monochrome path cannot express
   this — the R, G, B channel values would have to be independent.

3. **Emoji / full-colour sprites were wrong.** An RGBA emoji cannot be tinted:
   its RGB channels carry colour information, not coverage. Routing it through
   the mono coverage shader produces an incorrect result (tint colour is
   multiplied into the colour data).

GPUI's 2025 design notes identified the same three-way split as load-bearing:
their codebase separates these into three draw calls with three shader paths.

## Decision

Introduce `SpriteKind` with three variants, carried on every `DrawSprite`
command. The renderer dispatches to the correct blend / sampling path based
on `kind`.

```rust
pub enum SpriteKind {
    /// Single-channel alpha coverage, tinted by `DrawSprite::color`.
    /// Used for: icons (SDF/Slug baked), monochrome glyphs, symbols.
    Mono,

    /// RGB subpixel-positioned text edges.
    /// R / G / B channels carry coverage at fractional pixel offsets.
    /// Used for: anti-aliased text on LCD / OLED screens.
    Subpixel,

    /// Full-colour RGBA from atlas. Tint color is ignored.
    /// Used for: emoji, image thumbnails, full-colour sprites.
    Poly,
}
```

### Why three variants, not one enum with flags

A single `Sprite` type with an `is_subpixel: bool` and a `use_tint: bool`
creates four states, two of which (`is_subpixel && !use_tint`,
`!is_subpixel && !use_tint && use_tint == false`) overlap semantically. The
three-variant enum is exhaustive, unambiguous, and maps one-to-one to the
three shader paths. The Rust compiler can exhaustive-match on it; a bool pair
cannot be exhaustive-matched safely.

### Shader dispatch

The fill shader (`fill_paint.wgsl`) branches on `sprite_kind` at the fragment
level:

- `Mono`: `output.a = atlas_sample.r; output.rgb = tint.rgb * tint.a;`
  Premultiplied alpha blend into the render target.
- `Subpixel`: `output.rgb = vec3(atlas_sample.r, atlas_sample.g, atlas_sample.b) * tint.rgb;`
  Per-channel coverage multiplied by tint; written with per-channel alpha
  blend (dual-source blending where available, single-pass approximation
  otherwise).
- `Poly`: `output = atlas_sample;` Pass-through; tint field unused.

The branch is resolved at compile time on backends that support specialization
constants (DX12 DXIL, Metal MSL). On Vulkan/wgpu without specialization, it
is a dynamic branch on a uniform — branch coherency ensures no divergence
penalty in practice since all sprites in a batch share the same `kind`.

### SpriteSource and the subpixel phase field

```rust
pub struct SpriteSource {
    pub atlas: u32,
    pub region: Rect,
    /// Subpixel-x phase offset in `[0.0, 1.0)` for `SpriteKind::Subpixel`.
    /// Ignored for `Mono` and `Poly`.
    pub subpixel_offset_x: f32,
}
```

The `subpixel_offset_x` field carries the fractional-pixel phase for Subpixel
sprites. Each of the three RGB channels in the atlas region was rasterised
at a different sub-pixel offset; `subpixel_offset_x` tells the shader which
channel to use for which position. The field name is verbose (noted as a
follow-up rename to `phase` in the R14 B6 task); the semantics are correct.

### Icon delivery: Mono path

R8's `IconBaker` produces a single-channel alpha mask (`BakedIcon { alpha:
Vec<u8> }`). The caller uploads this to an atlas region and emits a
`DrawSprite { kind: Mono, color: theme_color, ... }`. The Mono branch in the
shader tints the coverage by `theme_color`, which is the correct result for
a themed monochrome icon. No separate icon pipeline is needed.

## Consequences

- `DrawSprite` carries `kind`, `source`, `destination`, `color`, `opacity`.
  `color` is well-defined for Mono/Subpixel (tint) and explicitly ignored for
  Poly. Widget authors are expected to pass `Color::WHITE` for Poly sprites
  to signal "no tint intended", even though the shader ignores it.
- The sprite atlas format is `R8Unorm` for Mono/Subpixel and `Rgba8Unorm`
  (or `Rgba8UnormSrgb`) for Poly. The atlas manager must track format per
  region; regions of different formats cannot be batched in the same atlas
  texture.
- Batching: all Mono sprites with the same tint in a frame can share a draw
  call. Subpixel sprites batch by themselves (different blend equation). Poly
  sprites batch by atlas texture. Pipeline switches occur at kind boundaries,
  not per-sprite.
- The three-variant split is the stable public API of `SceneCommand::DrawSprite`.
  A fourth kind (e.g., SDF-direct without the bake step) would require a new
  variant and a new DR.

## Kill Criteria

Revisit if:
- A hardware target does not support the per-channel coverage blend equation
  needed for Subpixel. In that case fall back to Mono for that target and
  document the downgrade.
- Subpixel AA is determined to look wrong on OLED (which has vertical-stripe
  or delta-layout subpixel grids). Add a `SubpixelVertical` variant or disable
  Subpixel on non-standard-stripe displays.
- Emoji rendering requires `Rgba8UnormSrgb` for correct colour space handling
  and the atlas manager cannot hold mixed-format atlases. In that case split
  Poly into `PolyLinear` / `PolySrgb` variants.

## Alternatives Rejected

| Alternative | Reason |
|---|---|
| Single `Sprite` with `is_subpixel: bool` + `has_tint: bool` | Four states from two bools; two states are redundant. Exhaustive match is impossible. |
| Separate pipeline per kind | Three pipeline objects, three bind groups, three shader files. Batching is still required within each pipeline, adding complexity without reducing draw calls. |
| Subpixel as a post-process pass | Post-processing the whole frame for subpixel text breaks compositing with non-text layers. Subpixel must be resolved per-glyph at draw time. |

## References

- `docs/plans/phase-R7-three-way-sprite-split.md`
- `crates/renderer/src/scene_command.rs` — `SpriteKind`, `SpriteSource`, `DrawSprite`
- `crates/renderer/src/sprite_paint.rs`, `sprite_paint.wgsl`
- [DR-014](./DR-014-renderer-slug.md) — Slug renderer (glyph atlas)
- [DR-031](./DR-031-vector-primitive-set.md) — Vector primitive set
- [DR-032](./DR-032-slug-as-universal-vector-engine.md) — Slug as universal vector engine (icon baker)
