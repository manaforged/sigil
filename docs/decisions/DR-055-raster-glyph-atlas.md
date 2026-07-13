---
id: DR-055
status: accepted
date: 2026-07-12
covers: Raster glyph atlas + pipeline shape for the Wave-3 hinted small-text path
relates: [DR-044 (owned text stack), plan-2026-07-07-text-quality-and-gap-closure Wave 3, 9c0615a (W3-D1 rasterize_glyph)]
---

# DR-055 — Raster glyph atlas + pipeline shape (Wave 3 D2)

**Status:** Accepted · **Date:** 2026-07-12

## Context

W3-D1 landed the CPU hinted rasterizer (`renderer/src/raster.rs`): skrifa hinted
outlines → zeno 8-bit alpha mask, 4 subpixel x-bins, `RasterKey` cache identity.
D2 must put those masks on screen: an atlas, a GPU pipeline, and draw-time
selection between the raster path and vector Slug. Three shape questions decide
the integration.

## Decision

1. **Separate `RasterTextPipeline` beside `SlugTextPipeline`, not a merged
   instance buffer.** Slug instances carry band-buffer offsets and per-polarity
   gamma; raster instances carry atlas UVs. Merging the two vertex layouts
   would put a discriminant branch in the hot fragment shader for zero
   batching win (the two paths already split by glyph size, so they rarely
   share a draw). Each pipeline keeps its own instance buffer; the text draw
   entry partitions glyphs once and issues at most two draws per run.

2. **One shared R8 atlas texture, shelf-packed, 1024² growing once to 2048².**
   Keyed by `RasterKey` (bit-exact physical px + bin). No eviction in D2: when
   the atlas is full, new glyphs fall back to vector Slug — correctness never
   depends on atlas capacity. A 2048² R8 atlas holds ~10k small-glyph masks
   (~16×16 px each), far beyond a realistic UI working set.
   `// ponytail: no eviction — add LRU shelf reuse if a real app fills 2048²`.

3. **Selection at the text-draw entry point, runtime-gated.** A glyph run takes
   the raster path when `size_px <= RASTER_MAX_LOGICAL_PX` (20, from the plan)
   AND the run is untransformed (today all UI text is; the check is a seam for
   future animated/transformed text) AND the new `TextTuning.raster` flag is
   on. Runtime flag over a cargo feature: the tuning panel can A/B raster vs
   vector live, exactly like the gamma/sharpen/dilation knobs, and the flag
   default stays OFF until the raster path beats vector on the
   glyph_pixel_check sharpness metric at all four scales (the plan's gate).

4. **Stem darkening stays vector-only.** Hinted masks are already grid-fit;
   applying `pow(cov, 1/gamma)` on top would double-darken. The raster
   fragment shader samples the atlas and multiplies by instance color alpha —
   nothing else.

## Consequences

- Two pipelines to keep in lockstep on surface format / blend state; both hang
  off the same `DrawParams`, so the seam is one struct.
- Atlas-full behavior is silent vector fallback — a `tracing::warn!` once per
  session marks it so a real overflow is observable.
- The gate metric needs a raster-path variant of the stem-sharpness test
  before the flag can default on (D3).

## Kill criteria

- If profiling shows the two-draw split measurably regresses glyphs/frame on
  the HUD bench, revisit the merged-pipeline decision with numbers.
- If a real application fills the 2048² atlas (the once-per-session warn
  fires in normal use), implement LRU shelf reuse.

## Rejected alternatives

- **Merged pipeline with per-instance path discriminant** — hot-shader branch,
  two vertex layouts crammed into one stride, no measured batching benefit.
- **Compile-time cargo feature for the raster path** — kills live A/B, splits
  golden baselines across feature matrices, and the plan's gate explicitly
  wants measured side-by-side comparison.
- **Guillotine/skyline packer with eviction** — solves a capacity problem the
  working-set math says we don't have; shelf packing is ~40 lines and O(1).
