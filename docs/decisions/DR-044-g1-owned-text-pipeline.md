---
id: DR-044
status: proposed
date: 2026-05-16
covers: G1 owned text pipeline (sub-tasks G1.1 through G1.7)
depends_on: [DR-040, DR-014, DR-031, DR-032, DR-033]
relates: docs/plans/2026-05-16-g1-owned-text-pipeline-dispatch.md
---

# DR-044 — G1 Owned Text Pipeline (skrifa + Slug + unicode-segmentation)

**Status:** Proposed
**Date:** 2026-05-16
**Covers:** Phase G1 sub-tasks G1.1 (cargo skeleton), G1.2 (segmentation), G1.3 (owned shaper), G1.4 (engine + cache rewrite), G1.5 (Slug glyph-run integration), G1.6 (opt-in `shaping-complex` HarfRust), G1.7 (parley dep removal + verification)
**Depends on:** DR-040 (sigil thesis lock), DR-014 (Slug renderer), DR-031 (vector primitive set), DR-032 (Slug as universal vector engine), DR-033 (three-way sprite split)
**Unlocks:** G2 entity state model can land independently; G12 catalog widgets that ship text editor surfaces; Phase AI3 chat panel; Phase AI4 inline completion overlay (both depend on owned text run pipeline for streaming text + animated cursor render).

---

## Context

`sigil-text` ships today as a Parley-backed engine: `parley = "0.8"` is the
runtime dep, the engine wraps `parley::FontContext` + `parley::LayoutContext`,
and `paragraph.rs` shapes runs through Parley's API. The text path consumes
`sigil-style`'s `StyledTextEngine` trait and produces `sigil-hal::ParagraphMetrics`
+ positioned glyph runs that the renderer hands to the Slug glyph rasterizer
(landed in R-foundation per DR-014 + DR-032).

DR-040 locks the architectural decision: own the text pipeline. Parley is on
the **never-adopt** list ("Linebender stack alignment isn't worth the dep
weight when we're not on Vello"). The owned-text bet is the sharpest part of
DR-040's "own the foundation like GPUI" thesis and the Slug differentiator
(R1-R14, ~95% of UI text is Latin + CJK ideographic + punctuation, which
skrifa-driven owned shaping handles without HarfBuzz).

Three reasons this DR exists now:

1. **G2 entity state model is the next critical-path move**, and G1 is the only
   G-substrate task that hasn't moved. The dispatch plan locks G0→G1→G2 order
   and G2 cannot dispatch ahead of G1.
2. **The 2026-05-16 research check found Parley switched from swash to HarfRust**
   (Google's HarfBuzz Rust port) in Oct 2025. Our text-rental cost on Parley
   now includes HarfRust transitively, which is a larger surface than DR-040
   characterised. The replacement gets harder the longer we wait.
3. **HarfBuzz 14.0 shipped `libharfbuzz-gpu`** in March 2026 — the first credible
   open-source Slug-equivalent. Our Slug moat shrinks if HarfBuzz GPU
   stabilises; the answer is to lock the owned-text pipeline now with HarfRust
   as the opt-in escape hatch rather than commit to either text stack.

---

## Decision

Replace the Parley backend in `crates/text/src/engine.rs` + `paragraph.rs` +
`cache.rs` with an owned shaper on **skrifa + unicode-segmentation + lyon_geom**.
Slug already exists in the renderer; G1 hands it positioned glyph runs instead
of Parley-shaped runs. Complex-script shaping (Arabic / Hebrew / Devanagari /
Khmer / Thai / Lao) lands behind an opt-in `shaping-complex` cargo feature that
pulls in HarfRust as the shaper backend; the default build is HarfRust-free.

Land in 7 sub-tasks (G1.1–G1.7). Each sub-task is a single dispatch unit.

### Module plan

| Module | Disposition | Why |
|---|---|---|
| `engine.rs` | **REWRITE** in G1.4 — owned `TextEngine` against the same `StyledTextEngine` trait surface | Parley type leaks (`FontContext`, `LayoutContext`) replaced with owned types under the same trait. |
| `paragraph.rs` | **KEEP-PUBLIC** surface, **REWRITE** body in G1.4 | `Paragraph` shape stays. Internal types swap. |
| `cache.rs` | **REWRITE** in G1.4 against owned shaper output | Cache-key shape stays (matches DR-024). Body rewires from Parley-cluster output to owned-cluster output. |
| `config.rs` | **KEEP** — no parley types in API | Config wraps font directories + DPI + feature flags. No parley leak. |
| `error.rs` | **REWRITE** one variant | `TextError::ParleyInvalid` → `TextError::ShapeFailed { reason }`. |
| `lib.rs` | **REWRITE** the prelude exports | Replace `parley::{...}` re-exports with owned types. |
| `shaper.rs` | **ADD-NEW** in G1.3 | Owned shaper. Latin + CJK punctuation + emoji passthrough. Pure-Rust on skrifa. |
| `segment.rs` | **ADD-NEW** in G1.2 | unicode-segmentation wrapper for grapheme / word / sentence iteration; Unicode 15.1 baseline. |
| `bidi.rs` | **ADD-NEW** in G1.2 | Simple bidi-level computation per Unicode UAX #9 baseline. Latin-only paragraphs short-circuit to level 0; mixed-direction paragraphs hit the full algorithm. |
| `glyph.rs` | **ADD-NEW** in G1.3 | Glyph-run output type: positioned `GlyphId` + advance + cluster index + baseline offset. The substrate Slug consumes. |
| `shaping_complex.rs` | **ADD-NEW** in G1.6 | Gated behind `shaping-complex` cargo feature. HarfRust wrapper. When feature off, the module compiles to an empty stub so call sites are stable. |

### API trait sketch (frozen for downstream dispatches)

```rust
// crates/text/src/lib.rs — public re-exports stay name-stable from G1.4:
pub use engine::TextEngine;
pub use paragraph::Paragraph;
pub use error::TextError;

// crates/text/src/engine.rs — owned engine:
pub struct TextEngine {
    fonts: FontRegistry,    // skrifa-backed
    shaper: ShaperPool,     // per-thread shaper instances
    cache: ShapeCache,      // cluster-keyed shape memoization
    config: Config,
}

impl TextEngine {
    pub fn new(config: Config) -> Self;
    pub fn paragraph(&self, content: &TextContent, style: &TextStyle) -> Paragraph;
    pub fn metrics(&self, content: &TextContent, style: &TextStyle, max_width: f32) -> ParagraphMetrics;
}

impl StyledTextEngine for TextEngine { /* trait body unchanged */ }

// crates/text/src/glyph.rs — landing in G1.3:
pub struct GlyphRun {
    pub glyphs: SmallVec<[GlyphPos; 16]>,
    pub baseline: f32,
    pub bbox: Rect,
}
pub struct GlyphPos {
    pub id: GlyphId,           // skrifa-native id
    pub advance: f32,
    pub offset_x: f32,
    pub offset_y: f32,
    pub cluster: u32,          // byte index in source text
}

// crates/text/src/shaper.rs — landing in G1.3:
pub(crate) struct Shaper { /* skrifa-backed */ }
impl Shaper {
    pub(crate) fn new(font: &Font) -> Self;
    pub(crate) fn shape(&self, segment: &TextSegment, style: &TextStyle) -> GlyphRun;
}

// crates/text/src/segment.rs — landing in G1.2:
pub struct GraphemeIter<'a> { /* unicode-segmentation::Graphemes wrapper */ }
pub struct WordIter<'a>     { /* unicode-segmentation::UnicodeWords wrapper */ }
pub struct SentenceIter<'a> { /* unicode-segmentation::UnicodeSentences wrapper */ }

// Public API consumed by widgets:
pub fn graphemes(input: &str) -> GraphemeIter<'_>;
pub fn words(input: &str) -> WordIter<'_>;
pub fn sentences(input: &str) -> SentenceIter<'_>;
```

All method names are **one word** per `the host engine/docs/architecture/authoring-syntax.md`.
No `with_` prefix. No trivial getters.

### Cargo dep swap (G1.7)

Remove from `crates/text/Cargo.toml`:
- `parley = "0.8"` — REMOVED.

Add:
- `skrifa = "0.34"` — workspace pin; font parsing + outline glyph data.
- `unicode-segmentation = "1"` — grapheme / word / sentence break iterators.
- `lyon_geom = "1"` — already on forever-rent list per DR-040; geometry primitives for cluster bounding boxes.
- `unicode-bidi = "0.3"` — UAX #9 bidi level computation. Pure Rust, zero transitive deps.

Optional (gated behind `shaping-complex` feature):
- `harfrust = "0.6"` — Google's HarfBuzz Rust port. Loaded only with the feature flag.

Dev-only (no runtime adoption):
- `parley = "0.8"` — moved to `[dev-dependencies]` as the **shaping oracle** for G1.3 property tests. Same pattern as `ropey` for I-T2. DR-040 own-what-we-replace clause permits dev-dep oracles.

### Cross-platform implications

- **No new platform branching.** Skrifa is pure-Rust, runs identically on
  Windows / macOS / Linux / wasm. Font discovery is the only OS-touching
  surface and that already lives in `crates/text/src/config.rs` via the HAL.
- **Web target (wasm32).** GPUI ships wasm32 as of Feb 2026. Sigil's web
  story stays undefined per DR-040, but skrifa + unicode-segmentation
  + unicode-bidi all compile to wasm32 cleanly. We do not gate web work on
  G1, but G1 does not block it either.
- **Complex-script feature handshake.** When `shaping-complex` is off, the
  `Shaper` Latin path handles ~95% of real UI text correctly. CJK punctuation
  + emoji + RTL marks render but bidi reordering is single-level. Arabic
  joining + Indic reordering + Khmer reordering produce visible bugs. The
  feature flag is documented as required for any consumer shipping a CJK /
  Arabic / Indic / Khmer / Thai / Lao UI.

---

## Consequences

- **G2 entity state model dispatches immediately after G1.4 lands** (engine
  + cache rewrite ships the new surface). G2 can compile against G1.1's
  skeleton until G1.4 ships the body — same pattern as P0 did with
  block.rs/flex.rs stubs against the post-P0.1 surface.
- **Slug rasterizer surface stays unchanged.** Existing R-foundation Slug
  code (`crates/renderer/src/slug/...`) consumes a `GlyphRun` shape that G1.3
  defines; the rasterizer body is unchanged. R14 widgets render identically.
- **Cache shape unchanged.** DR-024 layout-cache-key contract stays. Cache
  identity from G1.4 onward keys on `(font_id, style_atom, segment_hash)` —
  identical to today's Parley keying but the segment hash is grapheme-cluster
  based instead of Parley's internal hash.
- **Bench parity is non-negotiable.** G1.4 must hit ≥Parley parity on the
  existing `crates/text/benches/...` criterion suite at the same hardware.
  G1.6 (HarfRust opt-in) may regress complex-script shaping cost; that's
  acceptable for the gated feature.
- **HarfRust adoption decision is deferred** by this DR. We rent HarfRust
  conditionally (only when `shaping-complex` is on) without committing to
  it as the default backend. If HarfBuzz GPU stabilises in 2027 and
  consumers want it, we re-evaluate then.
- **Parley dev-dep oracle pays for itself.** Property test in G1.3 asserts
  `owned_shape(text, style) ≈ parley_shape(text, style)` within glyph-position
  tolerance for the Latin baseline. Any divergence surfaces immediately.

---

## Kill criteria

Revisit this DR if any of:

- **HarfBuzz GPU reaches API stability AND lands in a Rust text stack** (Parley
  swaps to HarfBuzz GPU, cosmic-text adopts, etc.) — Slug differentiation
  weakens; lean into ergonomic API + AI surface instead.
- **Parley adds a "Vello-free" mode with Slug-class text quality** — composability
  bet from Linebender becomes more credible.
- **Skrifa breaks API stability** — pin to a known-good version + monitor
  upstream churn; consider forking if churn becomes routine.
- **Owned shaper consistently regresses vs Parley** — re-evaluate whether
  the bench gap is worth the dep removal. Acceptance: ≥parity on
  paragraph-shape latency at 1k / 10k / 100k character bodies.

---

## Open questions

1. **HarfRust version pin.** Track the `shaping-complex` feature against the
   most recent HarfRust release at G1.6 dispatch time. Re-pin on every minor
   bump until HarfBuzz declares 1.0 GPU API stable.
2. **Font discovery on Linux.** Cross-distro fontconfig consistency is a
   solved problem at the OS level but a hard problem at the embedded level
   (snap / flatpak / appimage). Document the failure mode + escape hatch
   (`SIGIL_FONT_PATH` env var? config-file font list?) in G1.4 surface design.
3. **Emoji color subtable selection.** GPUI ships polychrome sprite split
   (DR-033) which lets sigil render color emoji via the polychrome path
   today. G1 confirms: `Shaper` produces emoji glyph IDs that map to the
   polychrome atlas via the existing R7 sprite-split mechanism. No new
   primitive needed.
4. **CJK punctuation correctness.** Specifically, half-width vs full-width
   punctuation in mixed Latin-CJK paragraphs. Document a sanity-check fixture
   in G1.3.

---

## References

- **DR-040** sigil thesis lock — `docs/decisions/DR-040-sigil-thesis.md`
- **DR-041** owned layout engine — `docs/decisions/DR-041-owned-layout-engine.md`
  (same own-it-now-don't-rent pattern; G1 follows P0's playbook one phase down).
- **DR-014** Slug renderer — `docs/decisions/DR-014-renderer-slug.md`.
- **DR-032** Slug as universal vector engine — `docs/decisions/DR-032-slug-as-universal-vector-engine.md`.
- **DR-033** three-way sprite split — `docs/decisions/DR-033-three-way-sprite-split.md`.
- **Strategic plan (re-synth)** — `~/.reviewer/plans/sigil-strategic-plan-2026-05-16.md`.
- **GPUI parity integration plan** — `docs/plans/2026-05-16-gpui-parity-integration.md`.
- **2026-05-16 research check** — Linebender stack drift (Parley → HarfRust);
  HarfBuzz 14.0 GPU release: `phoronix.com/news/HarfBuzz-14.0-Released`.
