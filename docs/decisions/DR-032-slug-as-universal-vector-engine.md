# DR-032 — Slug as Universal Vector Engine (Text + Icons + Logos)

**Status:** Accepted
**Date:** 2026-05-15
**Depends on:** DR-014 (Slug renderer), DR-031 (vector primitive set)
**Unlocks:** R8 (icon baker), future logo/branding assets

## Context

After the Slug glyph pipeline shipped (DR-014, R5-era), the renderer had a
resolution-independent fragment-shader rasteriser that worked on any closed
quadratic-Bézier contour. The question for R8 was whether to use the existing
Slug infrastructure for icons, or introduce a second pipeline.

The two main second-pipeline candidates were:

### MSDF (Multi-Channel Signed Distance Field)

MSDF (Chlumský 2014) stores a three-channel SDF in a pre-rasterised atlas.
At runtime a `median(r,g,b)` sample reconstructs smooth coverage, giving
sharp edges even when the icon is scaled up.

Limitations evaluated:
- **Per-icon atlas cost.** A 24×24 icon needs ~16 KB in the MSDF atlas at
  the quality level needed to avoid sharp-corner rounding. CJK + emoji
  equivalents are a concern for text; icon sets of 200–400 symbols push
  atlas size to the 6–8 MB range at acceptable quality.
- **Separate pipeline.** A distinct fragment shader, a different bind group
  layout, a new pipeline cache entry. In a single-render-pass frame this
  means a pipeline switch per batch boundary between text and icon.
- **Quality ceiling.** MSDF's corner reconstruction fails at scale ratios
  below ~0.5× (icons shrunk to very small sizes). The median-of-three trick
  doesn't recover sub-pixel accuracy in the way Slug's analytic winding does.
- **No subpixel support.** MSDF gives sharp edges, not subpixel-anti-aliased
  edges. Text and icons rendered side by side with different AA strategies
  produce a visible quality mismatch on LCD screens.

### SDF (Single-Channel Signed Distance Field)

Simpler than MSDF: one float per texel, `smoothstep` at the boundary.
Cheaper atlas, but worse corner quality than MSDF. Rejected for the same
reasons as MSDF, amplified.

## Decision

Extend the existing Slug pipeline to rasterise SVG icons through the same
band-buffer + winding-fill path used for glyphs.

### Architecture

```
SVG source string                                [crates/icons/src/parser.rs]
  → usvg parse + transform-flatten
  → cubic→quadratic approximation (lyon_geom)   [crates/icons/src/quadratic.rs]
  → 8-band vertical partition                   [crates/icons/src/bands.rs]
  → 4×4 SSAA CPU rasterizer → box-filter        [crates/icons/src/raster.rs]
  → BakedIcon { alpha: Vec<u8> }                [crates/icons/src/lib.rs]
      → upload to sprite atlas region
      → emit SceneCommand::DrawSprite(kind=Mono) [crates/renderer/src/scene_command.rs]
          → existing Slug fragment shader path   [crates/renderer/src/fill_paint.wgsl]
```

The critical step is the cubic→quadratic approximation. SVG paths use cubic
Bézier curves (`C` / `c` commands); Slug requires quadratic (`QuadCurve`).
`lyon_geom`'s `CubicBezierSegment::quadratic_bezier_approximations()` does
the conversion with a configurable flatness tolerance. The result feeds into
the same `BandBuffer` structure the glyph pipeline already uses.

### One pipeline, one atlas, one shader

Text glyphs and icons both end up as `SpriteKind::Mono` regions in the same
sprite atlas. The Slug fragment shader does not need to distinguish between
a glyph and an icon — both are band buffers containing quadratic curves and a
winding fill. The `sprite_kind` field on `DrawSprite` controls the tinting
path, not the rasterisation path.

This means:
- Zero pipeline switches when text and icons appear in the same draw batch.
- One cache key: `(icon_id, width, height)` — no DPI or size variant, same
  as the glyph cache.
- Icons benefit automatically from any future Slug analytic-AA improvements.

### Slug algorithm reference

Lengyel (2017): each pixel determines its band from UV.y, iterates the curves
in that band, solves `B(t).y = pixel.y` for `t ∈ [0, 1]`, and counts winding
contributions where `B(t).x > pixel.x`. Coverage = 1 if winding ≠ 0.

The patent (US 10,373,352) entered the public domain 2026-03-17 (20-year term
from filing 2006-03-17). See DR-014 for the full patent analysis.

### Current baking path vs. future direct Slug path

R8 ships a CPU rasteriser (4×4 SSAA → box-filter) as the baking output,
not a live fragment shader over band data. This is deliberate:

1. Icon rendering is not per-frame-path-dependent: an icon at 32×32 does not
   change shape frame to frame. Pre-bake once at startup; upload to atlas.
2. The live Slug fragment shader carries per-pixel iteration cost proportional
   to curves per band. For complex icons (SVG logos with 200+ segments), a
   pre-baked alpha mask is faster per draw call than runtime winding queries.
3. The bake path is a strict subset of the glyph path: the same `QuadCurve`
   format, the same `BandBuffer` structure. Future migration to live fragment
   rasterisation (if needed for very large icons or animated SVG) requires
   only a change to `IconBaker::bake` output format, not a pipeline change.

## Consequences

- `crates/icons` gains deps: `usvg` (SVG parse), `lyon_geom` (cubic→quad).
  Both are lightweight; `usvg` is already in the ecosystem as a transitive
  dep from `resvg`-family users.
- `sigil-renderer` gains no new deps. Icon baking is entirely in `sigil-icons`.
- The atlas upload path is shared with glyph upload. No new GPU buffer type.
- Icons must be baked at startup before the first frame. Lazy baking is
  possible in future if the icon set grows large enough to warrant it, but
  the current builtin set (≤ 100 icons) bakes in < 2 ms.
- Resolution independence: bake once at the UI's logical size (16/24/32 px).
  If the window DPI scale changes, rebake at the new physical size. This is
  the same lifecycle as the glyph cache.

## Kill Criteria

Revisit if:
- The icon set grows to 1000+ symbols and startup bake time becomes a
  measurable problem (> 50 ms). Lazy bake per first-use, or pre-bake to
  a binary asset file, would be the fix — not a new pipeline.
- Animated SVG icons become a requirement. Live fragment rasterisation of
  time-varying paths would then be worth the per-pixel cost.
- A hardware target is identified that cannot sustain the per-pixel curve
  iteration at the icon sizes in use (unlikely for any post-2016 GPU).

## Alternatives Rejected

| Alternative | Reason |
|---|---|
| MSDF atlas (separate pipeline) | Pipeline split cost, 16 KB/icon atlas overhead, corner-quality ceiling at small scales, no subpixel support. See Context. |
| SDF atlas | Worse quality than MSDF, same pipeline cost. Strictly dominated. |
| Vello for icons | No glyph/icon cache; per-frame rerender; large compute-pipeline overhead. See DR-014. |
| Raw bitmap PNG icons | Resolution-dependent; blurry at non-native DPI; large asset bundle. |

## References

- Lengyel, E. (2017). *GPU-Centered Font Rendering Directly from Glyph Outlines.* JCGT 6(2).
- Chlumský, V. (2014). *Shape Decomposition for Multi-Channel Distance Fields.* CTU thesis.
- `docs/plans/phase-R8-icon-baker.md`
- `crates/icons/src/lib.rs`, `bands.rs`, `quadratic.rs`, `raster.rs`
- [DR-014](./DR-014-renderer-slug.md) — Slug renderer decision (glyph path, patent analysis)
- [DR-031](./DR-031-vector-primitive-set.md) — Vector primitive set
- [DR-033](./DR-033-three-way-sprite-split.md) — Three-way sprite split (Mono delivery path)
