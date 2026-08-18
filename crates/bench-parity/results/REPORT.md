# Bench-parity report

Per-framework measurements on locked workloads. Every "X vs Y" comparison must verify that BOTH frameworks report `pass` for every equivalence check on the same workload before any speedup claim is made.

Host setup is untimed. `vs gpui` is a number only when both sides stamp the same reuse/tick mode, the same host (OS/CPU/rustc), the same verdicts (Pass vs N/A is not a pair), and both CVs stay at or under 0.25. An empty check map is not a pass. VOID if construct recorded all layout hits. NOISY if either CV is above the ceiling. `— mode` means one side retained the tree and the other reconstructed.

## Hardware

- OS: Darwin 26.5.1
- CPU: Apple M2 Max (12 cores)
- Memory: 32.00 GiB
- rustc: rustc 1.97.1 (8bab26f4f 2026-07-14)

## Workload — `flex-tree-2k`

| framework | iterations | median (µs) | p95 (µs) | cv | hits | misses | first | visible | book | rss (MiB) | vs gpui | equivalence |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---|
| gpui | 20 | 22920.62 | 23540.42 | 0.025 | 0 | 0 | 0 | 0 | 0 | 566.0 | 1.00× | layout_did_work=pass |
| sigil | 20 | 16034.79 | 17022.04 | 0.092 | 0 | 20 | 0 | 0 | 0 | 223.4 | 0.70× | layout_did_work=pass |
## Workload — `text-labels-2k`

| framework | iterations | median (µs) | p95 (µs) | cv | hits | misses | first | visible | book | rss (MiB) | vs gpui | equivalence |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---|
| gpui | 20 | 6765.88 | 7480.46 | 0.119 | 0 | 0 | 0 | 0 | 0 | 39.1 | 1.00× | layout_did_work=pass, text_did_work=pass |
| sigil | 20 | 979.21 | 1249.04 | 0.131 | 0 | 20 | 0 | 0 | 0 | 40.1 | 0.14× | layout_did_work=pass, text_did_work=pass |
## Workload — `virtual-list-100k-krausest`

| framework | iterations | median (µs) | p95 (µs) | cv | hits | misses | first | visible | book | rss (MiB) | vs gpui | equivalence |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---|
| gpui | 80 | 160.00 | 293.71 | 0.319 | 0 | 0 | 90 | 40 | 100000 | 13.5 | NOISY | book_held=pass, visible_window_indices=pass |
| sigil | 80 | 120.25 | 146.96 | 0.133 | 0 | 80 | 90 | 40 | 100000 | 11.8 | NOISY | book_held=pass, visible_window_indices=pass |
