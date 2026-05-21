# Bench-parity report

Per-framework measurements on locked workloads. Every "X vs Y" comparison must verify that BOTH frameworks report `pass` for every equivalence check on the same workload before any speedup claim is made.

> **Methodology note.** Different runners may currently time different surfaces — e.g. the Sigil runner times pure layout (`LayoutEngine::compute`); the GPUI runner times `VisualTestContext::draw` which includes paint-attempt setup and a `catch_unwind` of the headless-paint panic. Until those boundaries are reconciled, treat the rows below as single-framework measurements on a shared workload, not as a clean side-by-side comparison. See `crates/sigil-bench-parity/README.md` § "GPUI runner — shipped, with a methodology caveat".

## Hardware

- OS: Windows 11 (26200)
- CPU: AMD Ryzen 7 9800X3D 8-Core Processor            (16 cores)
- Memory: 61.65 GiB
- rustc: rustc 1.95.0 (59807616e 2026-04-14)

## Workload — `virtual-list-100k-krausest`

| framework | iterations | median (µs) | p95 (µs) | p99 (µs) | equivalence |
|---|---:|---:|---:|---:|---|
| gpui | 100 | 299.10 | 419.90 | 475.30 | visible_window_indices=pass |
| sigil | 100 | 263.80 | 510.00 | 641.20 | visible_window_indices=pass |

