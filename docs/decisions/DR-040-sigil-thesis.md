---
id: DR-040
status: accepted
date: 2026-05-16
covers: Strategic lock for Phase F, Phase P0, Phase G, Phase I, Phase AI, Phase FX, Phase X
depends_on: [DR-014, DR-031, DR-032, DR-034]
relates: docs/plans/2026-05-16-sigil-r15-dispatch-plan.md
---

# DR-040 — Sigil Thesis: General-Purpose Rust Desktop UI Library with First-Class Game-Engine + IDE + AI Integration

**Status:** Accepted
**Date:** 2026-05-16
**Covers:** Strategic lock for Phase F, Phase P0, Phase G (G-substrate + G-surface + G-catalog), Phase I, Phase AI, Phase FX, Phase X
**Depends on:** DR-014 (Slug renderer), DR-031 (vector primitive set), DR-032 (Slug as universal vector engine), DR-034 (layer pool + two-phase frame)
**Unlocks:** Every DR after DR-040 (DR-041+ resolve the open questions catalogued at the bottom of this file)

## Context

Sigil shipped its renderer foundation (R1–R13) and a Pack 3 / Pack 4 widget catalogue
through R14. The next eighteen months take the project from "renderer foundation +
R14 widget catalog" to "general-purpose Rust desktop UI library." That phrase is
ambiguous and the ambiguity is dangerous — three plausible products fit it:

1. A game-engine-tied UI library (sigil-as-the host engine-companion).
2. A general-desktop framework competing with Floem / Xilem / Iced.
3. An IDE substrate competing with GPUI / Zed / Lapce.

The adversarial review of the May 2026 strategic plan (`strategic-plan.md`)
noted correctly: "three products = ship none." This DR is the answer to that note.

Three external signals fired between Q1 and Q2 2026 that make this DR time-sensitive:

- **GPUI extraction signals.** Zed published commits splitting `gpui` into renderer
  + `ui` crates, telegraphing an eventual general-purpose extraction. If Zed ships
  a clean extraction in 12 months, substrate differentiation alone evaporates.
- **Xilem 1.0 trajectory.** Linebender's stack (Vello + parley + cosmic-text + taffy
  + AccessKit) is approaching 1.0 with real backing. By 2027 it will be the
  default-recommended Rust UI framework for new desktop projects unless something
  else is shipping with a clear differentiator.
- **AI-first IDE emergence.** Cursor + Zed-AI have demonstrated the chat-pane + agent
  loop + inline completion combination as a category. Neither has tier-1 Rust support;
  neither has a game editor; neither is open-substrate. The "AI-first Rust IDE that
  is also a game editor" niche is unclaimed at the time of this DR.
- **Unity trust collapse.** Unity's 2024-2025 pricing reversal opened a credibility
  gap in the engine-tooling market. Indies actively shopping for alternatives.
  Godot has the install-base; nobody has the editor + AI combo.

**Why this DR exists:** without it, Phase G dispatches will fork the architecture
in three directions (game-companion API decisions, general-desktop API decisions,
IDE-substrate API decisions) and the resulting surface will satisfy none of them.
Future-Thomas + future-teammates cite this DR to skip yaks that don't serve the
thesis.

## Decision

**Sigil is a general-purpose Rust desktop UI library, shipping today, with first-class
integration for three consumer classes:**

1. **Game engines.** the host engine is the primary consumer; `sigil-bevy` adapter (F2) owns
   the Bevy ecosystem niche. The the host engine inspector demo (F8) is the proof workload.
2. **IDEs.** the host engine's editor is a Zed-class IDE consumer — not "editor with IDE
   widgets" but the real article, on the same plugin host (I5) that any third-party
   IDE consumer would use. `sigil-ide-core` (I-track) is a library-quality crate
   that depends on `kernel` but not on `sigil` (the widget surface), so it is
   reusable by non-sigil IDE consumers.
3. **AI-first surfaces.** Chat panel (AI3), agent loop primitives (AI2), inline
   completion overlay (AI4), context-window inspector (AI5), AI-routed command
   palette entries (AI6) all ship as **native sigil widgets**. No webview. The AI
   surface is the sharpest differentiator in the developer-tools pitch and it is
   not delegated to anything except the model provider (AI1) itself.

The three consumer classes share one substrate. They are not three products;
they are three plugin sets on a shared base. The base is:

- **P0** — Owned layout engine (replaces taffy at the foundation; flexbox + block,
  rayon-parallel measure across `Position::Absolute` boundaries + layer scopes).
- **G-substrate** (G0–G5) — Owned text pipeline (G1), entity state model (G2),
  action dispatch + focus context (G3), DrawPhase state machine (G4), async UI
  bridges (G5).
- **G-surface** (G6–G11) — Ergonomic shortcut trait (G6), IME completeness (G7),
  drag-and-drop (G8), animation primitives (G9), AccessKit wiring (G10),
  cross-platform validation (G11).
- **G-catalog** (G12) — Tier-1 widget set including editor-critical promotions
  (code editor surface, file tree, command palette, find/replace, splitter panes,
  tabs, status bar, breadcrumb, diagnostics gutter, terminal emulator, debugger UI,
  diff view).
- **I-track + Phase I** — Workspace + project model (I1), document/buffer model
  (I2), LSP client (I3), DAP client (I4), plugin host (I5), settings + keymap
  engine (I6), theming + syntax color (I7), git integration (I8), wasm plugin
  sandbox R&D (I9). Library-quality primitives (I-T1 through I-T8) land early
  inside `sigil-ide-core` so that FE1/FE2/FE3 editor scaffolding sits on real
  primitives, not stubs.
- **Phase AI** — AI1 through AI6 above.
- **Phase FX** — the host engine-specific game-editor plugins (3D viewport with gizmos FX1,
  scene hierarchy + inspector FX2, asset browser FX3, play mode + live reload
  FX4, animation timeline FX5, AI-driven prefab/scene/asset gen FX6).
- **Phase X** — R&D track parallel to G: GPU-driven layout extension (X1, builds
  on P0), HDR / wide-gamut rendering (X2), zero-copy storage-buffer scene data
  (X3), variable-font animation primitive (X4), GPU-resident scene graph R&D (X5).

## Phase ordering locked

The strategic plan sequences foundational work serially; the dispatch plan parallelises
where dependencies allow. The lock:

1. **Phase F + P0 + I-track skeleton** run first. F1/F4/F6/F7 are paint-only and
   parallelise with P0. The I-T1 skeleton lands alongside this DR (W2.9 in the
   dispatch plan) so every later I-track task has a real home.
2. **G-substrate G0→G5** runs serially as the strategic plan dictates. G0 IS this
   DR. G1–G5 cannot be reordered without rewriting downstream work.
3. **G-surface G6–G11** runs partially parallel to the G-substrate tail.
4. **G-catalog G12** + **Phase I post-G2** + **Phase AI** run partially in parallel
   once G-substrate locks. Phase FX consumes both I and AI.
5. **Phase X** runs as a parallel R&D track that does **not** block any phase
   above. X items productionize only when proven on real workloads.

## Architectural commitments locked

These are derived from `strategic-plan.md` and codified here so the
dispatch plan does not need to relitigate them.

### Own the foundation

| Surface | Tenure | Rationale |
|---|---|---|
| Slug glyph rasterizer | **Owned** | Lengyel band-rasterization implementation. Lives in `crates/renderer/src/slug/`. Our text-rendering differentiator. Patent expired 2026-03-17. See DR-014 + DR-032. |
| Layout engine (P0) | **Owned** | Replaces taffy at the foundation. Layout↔paint↔scene-graph seam is where uber performance lives. Single-threaded taffy is the bottleneck at scale that no rented library can fix. |
| Text run pipeline (G1) | **Owned** | Built directly on skrifa + Slug + unicode-segmentation. ~95% of UI text content in pure Rust we control. Complex-script shaping behind opt-in `shaping-complex` Cargo feature. |
| ide-core primitives (I-T2 rope, I-T3 history, I-T4 multicursor, I-T5 fuzzy, I-T6 workspace, I-T7 settings, I-T8 keymap) | **Owned** | Library-quality, reusable beyond sigil's widget surface. Crate must NOT depend on `sigil`. |
| Entity state model (G2) | **Owned** | `Entity<T>` + `Subscription` + `Context::observe` + `notify()`. Pattern-matches GPUI's validated shape. |
| Action dispatch (G3) | **Owned** | Plugin extensibility hinges on this. Cannot be a rental. |
| DrawPhase state machine (G4) | **Owned** | Frame invariants are uber-performance territory. |
| AI agent loop (AI2) | **Owned** | Cursor + Zed-AI rent the agent layer; we own it because the AI surface is where the "general IDE library" pitch gets sharpest. |

### Forever-rental boundary

| Crate | Why rented forever |
|---|---|
| `wgpu` | IS the cross-platform GPU layer in Rust. No alternative we'd build. |
| `winit` | IS the cross-platform windowing layer. Per-OS rewrite is a 12+ month yak that doesn't differentiate. |
| `skrifa` | Font parsing is not where we differentiate. Lean, pure-Rust, already transitive via Slug. |
| `lyon_geom` | Tiny, math-only; geometry primitives we won't re-derive. |
| `unicode-segmentation` | ~10 KB, zero transitive deps, Unicode-spec work we won't redo. |
| `accesskit_winit` | Cross-platform a11y abstraction; we contribute upstream rather than fork. |
| `smol` | Matches winit's preference; lighter than tokio for UI. |
| `notify` | Cross-platform fs-watcher. Not a yak we want. (I-T6.) |
| `ignore` | BurntSushi's gitignore parser. Forever rental. (I-T6.) |
| `toml_edit` | Comment + ordering preserving TOML round-trip. (I-T7.) |
| `arboard`, `bytemuck`, `half`, `smallvec`, `ahash`, `smol_str` | OS / utility boilerplate; not differentiating. |

### Never-adopt list

These were considered and rejected. The dispatch plan must not propose them.

- **`cosmic-text`** — Bundles a full text engine when we need a shaper. Lapce
  ships it because Lapce is a code editor with rich text; we are a UI library
  whose 95%-of-text use case is Latin + CJK ideographic + punctuation.
- **`parley`** — Linebender stack alignment isn't worth the dep weight when we're
  not on Vello.
- **`swash` as a runtime dep** — Slug already rasterizes; swash overlaps. Acceptable
  as a transitive build-time dep only.
- **Runtime `usvg`** — Currently sits in the icon baker (`crates/icons/`). Must
  move to a build-script tool so SVGs are baked at build time into a static glyph
  atlas. `usvg` does not reach the consumer binary.
- **`taffy`** — Removed at P0. The "design for replacement" half-step had us
  writing the same trait wrapper twice. Owned layout from the start.
- **`ropey` as a runtime dep** — Acceptable as a dev-dependency oracle for I-T2
  property tests, NEVER as a runtime dep. Rope is small enough to own (~1.5 weeks
  budget) and IDE perf demands force a custom rope at G12-scale anyway.
- **`nucleo` as a runtime dep** — I-T5 fuzzy-search is small (~500 LoC); owning
  it is cheaper than locking into its scoring algorithm.

## Yaks explicitly skipped

These were considered during planning and explicitly out of scope. Future-Thomas
cites this list when a teammate proposes one of them.

- **VSCode extension compat layer.** Running unmodified VSCode extensions is a
  massive surface with low strategic value. Cursor and Zed both eventually shipped
  some compat shim and it's the most-cursed code in their repos. Out of scope.
  Ship Sigil-native plugins (I5) instead, port the high-value VSCode extensions
  one at a time.
- **Visual UI designer / DSL.** Slint has one. We bet on Rust syntax + ergonomic
  traits (G6) being enough. Not a 2026-2027 yak.
- **Mobile (iOS / Android) support.** Plausible via wgpu, but not the bet.
  winit's mobile backends are alpha. Out of scope.
- **Web target (wasm + WebGPU).** Plausible but not strategic right now. Out
  of scope.
- **VR/AR support / foveated rendering hooks.** Dropped from the X-track per
  the May 2026 adversarial review. Revisit only if a real consumer asks.
- **Variable rate shading for UI.** UI is the *worst* candidate for VRS. Text
  needs sharp edges everywhere. Skipped from X-track. Permanent no.
- **GPU work graphs.** DX12-only, vendor-fragile; cross-platform availability is
  2027+. Watch and revisit; do not invest in 2026.
- **i18n proper.** Font-fallback chains across CJK + RTL + Cyrillic + Indic
  scripts. G1 + `shaping-complex` feature gives a baseline; full Lokalise-grade
  i18n is its own crate, year 2+.

## Strategic non-negotiables

- **Dependency philosophy: own what we'd ever replace; no "planned replacement"
  tenure.** A "planned replacement" is a half-decision that bakes in future churn.
  Either commit to owning a dep, or commit to renting it forever. The double-work
  cost of "ship on rented X, swap to owned X later" exceeds the cost of owning
  from the start.
- **Performance bias: end-to-end control beats best-in-class composition.** The
  GPUI playbook, not the Xilem playbook. GPUI hits 120 FPS on full-screen text
  editing because they own every layer and optimize across seams nobody else
  can touch. The Xilem/Linebender bet ("compose excellent libraries: Vello +
  parley + cosmic-text + taffy + winit + AccessKit") stops scaling past the
  first bottleneck because they cannot reach across seams.
- **License posture.** Sigil stays **MIT / Apache-2.0** dual-licensed. the host engine stays
  **PROPRIETARY**. The IDE-core primitives ship under sigil's MIT/Apache; the
  the host engine editor + the host engine game runtime stay proprietary. No proprietary content
  enters the sigil repo. No GPL/AGPL dep enters the sigil repo.
- **Determinism + verification-honesty contract.** No "verified" claim without
  comparing against real captured data. No values baked from a single capture
  into source. The repo's existing the contributor guide verification-honesty rule applies
  unchanged.

## Consequences

- **`sigil-ide-core` crate gets first-class status.** Library-quality, no `sigil`
  widget dep, MIT/Apache-2.0, consumable by non-sigil IDE consumers. Workspace
  manifest forbids the `sigil` dependency direction at CI level (W2.9 establishes
  this). Future third-party Rust IDE forks consume `sigil-ide-core` directly
  without taking on the sigil widget surface.
- **the host engine editor is a Zed-class IDE, not "editor with IDE widgets."** FE1/FE2/FE3
  scaffolding is provisional; the underlying primitives (I-T2 rope, I-T3 history,
  I-T4 multicursor, I-T5 fuzzy, I-T6 workspace, I-T7 settings, I-T8 keymap) are
  production-grade. When G3 actions + G12 catalog land, the editor widget surface
  is rebuilt on production substrate without throwing the primitives away.
- **AI surface is sigil-native, not webview-shoved-into-a-pane.** AI3 chat panel,
  AI4 inline completion, AI5 context inspector, AI6 palette routing all render
  through the same scene-command pipeline as every other widget. No web embed.
  This is what lets us bench the AI surface against the rest of the frame budget
  rather than treating it as a black-box subprocess.
- **Phase F + P0 + I-T1 skeleton run first.** the host engine integration (F1, F4, F6, F7)
  is paint-only and parallel-safe with P0. Setting these expectations now keeps
  Wave 3 dispatches from waiting on Wave 2 P0 layout work that does not block them.
- **G-substrate ordering is non-negotiable: G0 → G1 → G2 → G3 → G4 → G5.** IDE
  needs raise the bar on each substrate piece; they do not let us skip any.
  The mistake to avoid is "the IDE is urgent so let's hack quick widgets on the
  current foundation" — that produces widgets thrown away when G1/G2/G3/G4 land.
- **Phase FX is gated on Phase I + Phase AI.** the host engine game-editor plugins (3D
  viewport with gizmos, scene hierarchy, inspector, asset browser, play mode,
  animation timeline, AI content generation) consume the plugin host (I5),
  agent loop (AI2), and chat panel (AI3). Sequence FX after I + AI; do not
  attempt to ship FX1 as a standalone before I5 lands.
- **The dispatch plan is allowed to schedule things in parallel, but never
  out of dependency order.** This DR is the dependency-order authority.

## Kill criteria

Revisit this DR if any of:

- **GPUI ships a clean public extraction within 12 months.** Substrate
  differentiation alone evaporates. We lean further into the AI-first IDE +
  game-editor combo (which neither GPUI nor Zed cover) and consider sunsetting
  the general-desktop pitch.
- **Xilem 1.0 ships with Vello text quality matching Slug.** Slug moat shrinks.
  Re-evaluate whether owned text pipeline (G1) remains a differentiator or
  whether we lean entirely on IDE-density + game-engine integration.
- **A the host engine slip removes sigil's primary consumer for 6+ months.** Sigil
  becomes library-without-users. Re-evaluate whether the IDE pitch alone
  carries the project or whether scope narrows to "Rust IDE library, fork the
  editor."
- **A Unity rebound captures the indie engine-tools market.** Phase FX value
  decreases. Re-prioritize Phase I + Phase AI as the load-bearing pitch.
- **AI provider economics shift such that local-model is the only viable path.**
  AI1 model-provider abstraction already handles this; verify the local-model
  path is real before AI3 ships.

## Open questions

These are explicitly **not** resolved in this DR. They will land in subsequent DRs.

- **DR-041 (proposed): Cross-platform validation strategy.** What is the
  pixel-tolerance policy per platform? How do Mac / Linux / Windows golden
  divergences get bucketed? This blocks G11 dispatch.
- **DR-042 (proposed): IME backend per OS.** Win32 TSF vs IMM32; macOS
  NSTextInputClient; Linux IBus vs Fcitx5. Which surface does sigil expose
  through G7 and how do per-OS backends plug in? Blocks G7 dispatch.
- **DR-043 (proposed): Plugin host transport — in-process vs wasm.** I5 ships
  in-process. I9 prototypes wasm sandboxing. The question of whether wasm
  becomes the default or stays an opt-in deferred until I5 is real and we
  observe in-process plugin abuse patterns. This DR locks the I5-before-I9
  ordering but defers the transport-default decision.
- **DR-044+ (TBD):** AI provider abstraction shape (AI1); agent loop tool-use
  schema (AI2); inline-completion debounce + cancellation contract (AI4);
  context-window inspector token-accounting accuracy (AI5). Each gets its
  own DR when the corresponding Phase AI milestone is dispatched.

## References

- **Strategic plan:** a local plan document
  — the source of truth for files, deliverables, test gates, and acceptance
  criteria for every phase referenced in this DR (F0–F8, FE1–FE3, P0, G0–G12,
  I1–I9, AI1–AI6, FX1–FX6, X1–X5, I-T1–I-T8).
- **Dispatch plan:** a local plan document
  — the "who and when" companion to the strategic plan. This DR is W2.0 in
  that plan.
- [DR-014](./DR-014-renderer-slug.md) — Slug renderer decision (the rasterizer
  on which G1 + G12 build).
- [DR-031](./DR-031-vector-primitive-set.md) — Vector primitive set R1–R13
  (the renderer foundation this DR builds Phase G atop).
- [DR-032](./DR-032-slug-as-universal-vector-engine.md) — Slug as universal
  vector engine (locks the one-pipeline approach for text + icons).
- [DR-033](./DR-033-three-way-sprite-split.md) — Three-way sprite split
  (`Mono` / `Subpixel` / `Poly`, the delivery model G1 text consumes).
- [DR-034](./DR-034-layer-pool-two-phase-frame.md) — Layer pool + two-phase
  frame walker (the rendering substrate Phase G4 DrawPhase will gate against).
