# DR-014 — Text Renderer: Slug GPU Algorithm

**Status:** Accepted
**Date:** 2026-04-15

## Context

L3 (hit-testing, caret geometry, selection) shipped over Parley. L4 is the
first layer that makes text visually appear on screen. The renderer needs a
strategy for rasterizing shaped glyphs onto the wgpu surface.

Candidates evaluated:

### 1. Vello (vector renderer by the Linebender/Google Fonts team)

Vello uses a GPU compute pipeline with five stages:

> flatten curves → bin to 16×16 tiles (prefix-sum) → PTCL bytecode → fine rasterize

Advantages: well-maintained, used by Xilem. Disadvantages:
- **No glyph cache** — every frame rerenders every glyph from scratch.
- Shader-compilation overhead at startup (large WGSL compute shaders).
- Bounded-memory issues at high glyph counts in early benchmarks.
- Adds a large dependency with significant compile-time cost.

### 2. CPU rasterizer + atlas (swash / ab_glyph style)

Rasterize glyphs to a CPU bitmap, upload to a GPU atlas, draw textured quads.

Advantages: simple, well-understood. Disadvantages:
- Resolution-dependent: a separate atlas entry per (glyph, size, DPI) tuple.
- CPU rasterization on every cache miss; blurry at non-native DPI.
- Large atlas for CJK / emoji / variable fonts.

### 3. Slug (Lengyel, 2017)

A fragment-shader algorithm: each glyph is a screen-space quad. The fragment
shader ray-casts through a preprocessed "Bézier band" texture and computes
the winding number analytically per pixel.

Algorithm summary:
1. **CPU preprocessing (once per glyph):** divide the glyph bounding box into
   N horizontal bands. For each band, collect the quadratic Bézier curves that
   intersect it. Store as a compact flat-float buffer.
2. **GPU fragment shader (every pixel):** determine the pixel's band from UV.y,
   iterate curves in that band, solve the quadratic `B(t).y = pixel.y` for
   t ∈ [0,1], count winding contributions from crossings where `B(t).x > pixel.x`.
   Coverage = 1 if winding ≠ 0.

**Patent status:** US 10,373,352 was granted to Terathink (Lengyel) in 2019.
The patent **entered the public domain on 17 March 2026** (20-year term from
filing date 17 March 2006, including continuation chain). As of the date of
this DR, the algorithm is freely implementable.

Advantages:
- Resolution-independent: the preprocessing step produces normalized [0,1]
  coordinates, so the band buffer is scale-invariant. No per-size cache entries.
- Simple cache key: `(font_id, glyph_id)` — no DPI or size variant.
- Fragment-only: no compute shaders, no prefix-sum pipeline. Works on any
  wgpu-supported backend including WebGL2 (via Naga).
- Variable fonts: no pre-rasterization means instance changes are free.
- `skrifa` (Google's Fontations parser, already a transitive dep via Parley)
  provides Bézier outlines directly, making integration straightforward.
- Commercial pedigree (used in production game engines, now patent-free).

Disadvantages:
- Analytic antialiasing (the full Slug contribution) is deferred to Phase 2;
  Phase 1 ships binary coverage.
- Per-pixel curve iteration: O(MAX_CURVES_PER_BAND) per fragment. Acceptable
  for UI text; benchmarks deferred to the perf-discipline milestone.
- Novel approach — fewer off-the-shelf examples than atlas-based rendering.

## Decision

Implement Slug as the glyph rasterizer in `sigil-renderer`. Use `skrifa`
(direct dep, version pinned to the one already transitively provided by
Parley) for outline extraction.

### Architecture

```
Parley layout (L3)
  → Paragraph::rendered_glyphs() + Paragraph::fonts()   [sigil-text]
      → SlugTextPipeline::draw_paragraph()               [sigil-renderer]
          → GlyphCache: (font_id, font_index, glyph_id) → BandData  [CPU]
          → wgpu storage buffer upload                   [GPU upload]
          → WGSL Slug fragment shader                    [GPU render]
```

### Phase rollout

- **L4 Phase 1 (this DR):** binary coverage, one storage buffer per glyph
  per draw call, one draw call per glyph.
- **L4 Phase 2:** analytic antialiasing; band-buffer atlas (single buffer,
  per-glyph offsets); instanced batching.

### Adapter wall

`sigil-renderer` is the only crate that imports `skrifa` and `wgpu`. The swap
from any future alternative (e.g., a re-evaluation of Vello once it gains a
glyph cache) is contained entirely within `sigil-renderer`. All upstream
crates (`sigil-text`, `sigil-hal`, `sigil-core`) are unaffected.

## Consequences

- `sigil-renderer` gains direct deps: `skrifa`, `bytemuck` (already workspace).
- `sigil-renderer` gains a dep on `sigil-text` to receive `Paragraph` and the
  glyph/font data it exposes. This is a clean one-way dep (renderer is at the
  top of the dep graph).
- `sigil-hal::Color` requires `Default` impl (needed to satisfy Parley's
  `Brush: Default` bound when parameterising `Layout<Color>`).
- Text rendering is intentionally NOT added to the `sigil-hal::Renderer` trait
  in this phase. The scene recording API (Phase 3) will define the correct
  abstraction once the scene graph exists.

## Kill Criteria

Reconsider if:
- A measured fragment-shader bottleneck exceeds the Phase 3 performance budget
  (target: 60 fps at 1440p with a full-screen text editor).
- Vello ships a stable glyph-cache API and the compile-time cost is acceptable.
- A hardware target we care about cannot run the required storage-buffer binding
  (unlikely: wgpu requires `STORAGE_RESOURCE_BINDING_ARRAY` only for arrays;
  a single `var<storage, read>` is Tier 1).

## References

- Lengyel, E. (2017). *GPU-Centered Font Rendering Directly from Glyph Outlines.*
  Journal of Computer Graphics Techniques, 6(2).
- US Patent 10,373,352 — filed 2006-03-17, expired 2026-03-17.
- [EricLengyel/Slug](https://github.com/EricLengyel/Slug) — original OpenGL reference impl.
- [skrifa](https://github.com/googlefonts/fontations) — Fontations outline parser.
- [DR-001](./DR-001-renderer-wgpu.md) — wgpu backend decision.
- [DR-004](./DR-004-text-parley.md) — Parley shaping decision.
