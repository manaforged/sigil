# DR-009 — Performance Discipline

**Status:** Accepted
**Date:** 2026-04-14

## Context

This library targets a specific product shape: **high-complexity
desktop UIs with real-time live data.** Concrete workloads:

- Dashboards with 50-500 reactive signals, each updating 10-100 Hz
  from WebSocket/IPC/polling sources → 500-50,000 writes per second.
- Virtualized data grids over 10,000-100,000 rows, where every row's
  cell may be signal-driven but only 30-50 rows are rendered at any
  moment.
- Deep component trees (1000+ nodes, 20+ nesting levels) whose
  disposal must cascade cleanly.
- Broadcast events (theme switch, locale change, window resize) that
  fan out to thousands of dependent effects in a single frame.
- Long-running sessions (10+ minutes of streaming) without memory
  growth.
- Concurrent data sources: multiple background tasks each writing
  signals at different rates into the main-thread reactive graph.

If any of these scenarios blows the 16.67 ms frame budget at 60 Hz
or drifts upward over a long session, the library fails its target
market. "Competitive with Leptos on microbenches" is not sufficient.

Prior to this DR, performance was addressed reactively: benches
existed but covered only hot-path microbenches (single-signal
read/write). Fan-out at scale, cross-thread round-trip latency,
sustained write throughput, memory-over-time, and frame-drain under
load were all unmeasured. Adversarial reviews surfaced perf-adjacent
issues (wake coalescing, drain starvation, fake task handles) that
correct design would have caught earlier.

## Decision

**Every crate that touches a hot path ships with benches and a
budget.** Every change to a hot path re-runs the benches. Every PR
that regresses >20% on any tracked bench fails CI.

### 1. Bench tiers

Every perf-relevant crate organises benches into five tiers:

| Tier | Purpose | Example |
|------|---------|---------|
| **1. Micro** | Single-op hot-path latency | `signal_set_one_subscriber` |
| **2. Fan-out** | One write → N dependents | `signal_set_1000_subscribers`, `derived_chain_depth_20` |
| **3. Real-time** | Sustained / burst write throughput | `sustained_1k_writes_per_sec_10s`, `burst_10k_writes_one_frame` |
| **4. Tree complexity** | Structural stress | `deep_tree_disposal_1k`, `wide_tree_build_10k_components` |
| **5. Allocation / memory** | Steady-state footprint + leak detection | `mem_per_signal`, `streaming_session_memory_delta_10min` |

A change that touches only its own tier need re-run only that tier.
A change to the core runtime re-runs all tiers.

### 2. Budgets (provisional — measure before the first real primitive)

| Operation | Target | Rationale |
|-----------|--------|-----------|
| `signal.get()` | ≤10 ns | Called 1000s of times per frame in any real UI. Must approach memory-access floor. |
| `signal.set()` with 1 sub | ≤120 ns | Common case; dictates reactive fan-out cost. |
| `signal.set()` with 100 subs | ≤20 µs | Fan-out under a theme switch. |
| `signal.set()` with 1000 subs | ≤200 µs | Broadcast events. 5 at ≤200 µs fit in one frame. |
| Cross-thread dispatch round-trip | ≤500 µs p99 | Background task write → effect fires on next frame. |
| Drain with 100 pending callbacks | ≤2 ms | Fits in frame budget alongside layout+paint. |
| Sustained 1k writes/sec, frame latency p99 | ≤16 ms | 60 Hz maintained under live-data load. |
| Burst 10k writes batched | ≤5 ms | One frame's worth of coalesced work. |
| Memory per signal (steady state) | ≤128 B | 10k signals = 1.3 MB — fits comfortably. |
| 10-minute streaming session memory growth | ≤10 MB | Bounded heap churn; any growth beyond this is a leak. |
| Deep owner disposal, 1000 nodes | ≤100 µs | Closing a complex dialog. |

**These numbers are provisional.** The DR commits to *measuring
against them*, not *hitting them on day one*. Current bench reality
will inform which are reached, which need refactors, and which need
their targets adjusted with a justification commit.

### 3. CI regression detection

Every PR runs `cargo bench -p <crate>` for each affected crate and
compares against a committed baseline (`docs/benchmarks/baseline-<crate>.json`,
criterion's stable JSON output).

Regression thresholds:

- **≤ +10 %**: pass silently.
- **+10 % to +20 %**: warn in CI output; PR can merge, but review
  comment auto-posted with the per-bench delta.
- **> +20 %**: fail CI. Merge blocked until either the regression is
  fixed or the baseline is explicitly updated with a justification in
  the PR body.

Baselines are committed artifacts. Updating a baseline requires a
one-line rationale in the commit message (e.g. `baseline: accept +15%
on signal_set_ten_subscribers — SmallVec inline cap raised to 8`).
This trades some merge-conflict friction for visible, git-blameable
perf trajectory.

### 4. Per-phase bench requirement

Every phase that lands new crates or primitives lands a bench suite
covering its relevant tiers:

- Foundation crates (`ui-reactive`, `ui-async`, `ui-renderer`,
  `ui-shell`) — Tiers 1-5.
- Layout / text / focus — Tiers 1, 2, 4.
- Primitives (Button, Input, Table, etc.) — Tier 4 (one Button is
  cheap; 1000 Buttons in a grid is what matters).
- Patterns (list page, data grid) — Tier 3 + Tier 4 together.

A phase may ship without hitting the budget for a given bench, but
it must at minimum measure and document. "We didn't measure" is the
failure mode this DR exists to prevent.

### 5. Measurement methodology

- **Criterion** with `--warm-up-time 1 --measurement-time 3
  --sample-size 30` for dev runs; CI smoke uses shorter windows.
- **Release profile** (`--release`) for every number reported as a
  perf result. Debug numbers are not cited.
- **Environment recording.** Every committed baseline includes
  machine info (OS, CPU, Rust version). Cross-machine comparisons
  are ratio-based, not absolute.
- **Statistical regressions.** Criterion's p-value < 0.05 is the
  threshold for "statistically significant change." Noise inside
  that band is ignored.
- **No inline micro-optimisation without a bench.** Every `#[inline]`,
  every `SmallVec` size tweak, every atomic ordering choice comes
  with a bench number showing the delta.

### 6. Allocation discipline

On render, layout, and event-routing hot paths: **zero heap
allocations**. Verified by bench + optional `cargo-bloat` / allocation
profiler runs. Enforced by code review; PRs that add a `Vec::new()`
or `Box::new()` to a hot path require a bench justifying the cost.

On cold paths (setup, disposal, one-time construction): allocation is
fine, measured where interesting.

## Consequences

- Every contributor writes benches. Pay once, win on every future
  regression.
- Baselines are git-tracked artifacts. Occasional merge conflicts on
  bench JSON, accepted cost.
- Some refactors need a baseline update even if they're neutral on
  behaviour — an explicit "no perf change" commit bumping the baseline
  is fine.
- CI bench-smoke gets slower. Current 10-sample check stays in
  bench-smoke; new bench-regression job runs in parallel with test and
  can be cached between pushes on the same branch.
- Skipping this discipline when "we'll fix perf later" is no longer
  an option. "Later" never comes in frameworks. Every un-benched
  commit is a potential silent regression compounding in the tree.

## Kill criteria

This DR is revisited if:

- Bench maintenance consumes >15 % of contribution time over a
  quarter. At that point the methodology needs simplification, not
  abandonment.
- False-positive regression alarms (environmental noise, not real)
  exceed 5 % of PRs. Increase sample size or move to a SaaS
  (Bencher.dev or similar).
- A hot-path budget is missed by >2x on measured hardware and no
  refactor closes the gap without compromising correctness. Either
  the target is wrong, or the architecture is — forces a conversation.

## References

- [DR-005 addendum — debuggability of thread-local ownership](./DR-005-reactivity-owner-tree.md)
- [criterion.rs documentation](https://bheisler.github.io/criterion.rs/book/)
- [Leptos reactivity benchmarks](https://github.com/leptos-rs/leptos/tree/main/benchmarks)
- [Floem benchmarks](https://github.com/lapce/floem/tree/main/benchmarks)
