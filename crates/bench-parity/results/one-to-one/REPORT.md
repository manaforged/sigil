# Bench-parity report

1:1 construct versus crates.io GPUI 0.2.2. Both sides build the tree and layout it every sample. No retain, no tick. Quote only a numeric `vs gpui` cell. There is no `vs frame` column.

Host setup is untimed. A cell prints only when `one_to_one` holds: construct on both sides, same host, same clean revision, same mounted row count, same canonical output, both CVs at or under 0.25, layout and text actually ran. VOID / NOISY / — source / — visible / — output / — checks fail this report.

## Hardware

- OS: Darwin 26.5.1
- CPU: Apple M2 Max (12 cores)
- Memory: 32.00 GiB
- rustc: rustc 1.98.0 (88d9e12ae 2026-08-18)
- GPUI: crates.io 0.2.2

## Workload — `data-table-10k`

| framework | iterations | median (µs) | p95 (µs) | cv | hits | misses | first | visible | book | rss (MiB) | vs gpui | equivalence |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---|
| gpui | 40 | 2607.17 | 2841.12 | 0.051 | 0 | 0 | 0 | 38 | 10000 | 29.6 | 1.00× | book_held=pass, column_count=pass, layout_did_work=pass, scene_did_work=n/a, text_did_work=pass, visible_window_indices=pass |
| sigil | 40 | 1728.79 | 2004.71 | 0.064 | 0 | 40 | 0 | 38 | 10000 | 55.2 | 0.66× | book_held=pass, column_count=pass, layout_did_work=pass, scene_did_work=n/a, text_did_work=pass, visible_window_indices=pass |
## Workload — `flex-tree-2k`

| framework | iterations | median (µs) | p95 (µs) | cv | hits | misses | first | visible | book | rss (MiB) | vs gpui | equivalence |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---|
| gpui | 20 | 20309.08 | 21140.88 | 0.030 | 0 | 0 | 0 | 0 | 0 | 567.9 | 1.00× | layout_did_work=pass |
| sigil | 20 | 14224.75 | 14880.38 | 0.076 | 0 | 20 | 0 | 0 | 0 | 227.1 | 0.70× | layout_did_work=pass |
## Workload — `kv-form-500`

| framework | iterations | median (µs) | p95 (µs) | cv | hits | misses | first | visible | book | rss (MiB) | vs gpui | equivalence |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---|
| sigil | 16 | 1287.21 | 1505.50 | 0.073 | 0 | 16 | 0 | 0 | 0 | 42.8 | 0.31× | layout_did_work=pass, text_did_work=pass |
| gpui | 16 | 4205.88 | 4575.38 | 0.073 | 0 | 0 | 0 | 0 | 0 | 31.6 | 1.00× | layout_did_work=pass, text_did_work=pass |
## Workload — `nested-flex-64`

| framework | iterations | median (µs) | p95 (µs) | cv | hits | misses | first | visible | book | rss (MiB) | vs gpui | equivalence |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---|
| gpui | 16 | 873.50 | 981.38 | 0.061 | 0 | 0 | 0 | 0 | 0 | 16.6 | 1.00× | layout_did_work=pass, text_did_work=pass |
| sigil | 16 | 437.12 | 606.67 | 0.156 | 0 | 16 | 0 | 0 | 0 | 19.6 | 0.50× | layout_did_work=pass, text_did_work=pass |
## Workload — `overlay-stack-100`

| framework | iterations | median (µs) | p95 (µs) | cv | hits | misses | first | visible | book | rss (MiB) | vs gpui | equivalence |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---|
| sigil | 16 | 106.00 | 116.46 | 0.080 | 0 | 16 | 0 | 0 | 0 | 15.1 | 0.48× | layout_did_work=pass, text_did_work=pass |
| gpui | 16 | 219.04 | 234.46 | 0.041 | 0 | 0 | 0 | 0 | 0 | 13.5 | 1.00× | layout_did_work=pass, text_did_work=pass |
## Workload — `text-labels-2k`

| framework | iterations | median (µs) | p95 (µs) | cv | hits | misses | first | visible | book | rss (MiB) | vs gpui | equivalence |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---|
| gpui | 20 | 3919.17 | 4265.25 | 0.063 | 0 | 0 | 0 | 0 | 0 | 31.4 | 1.00× | layout_did_work=pass, text_did_work=pass |
| sigil | 20 | 1055.92 | 1260.79 | 0.089 | 0 | 20 | 0 | 0 | 0 | 39.5 | 0.27× | layout_did_work=pass, text_did_work=pass |
## Workload — `virtual-list-100k-krausest`

| framework | iterations | median (µs) | p95 (µs) | cv | hits | misses | first | visible | book | rss (MiB) | vs gpui | equivalence |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---|
| gpui | 80 | 449.62 | 588.29 | 0.106 | 0 | 0 | 100 | 40 | 100000 | 14.6 | 1.00× | book_held=pass, layout_did_work=pass, text_did_work=pass, visible_window_indices=pass |
| sigil | 80 | 218.29 | 275.96 | 0.095 | 0 | 80 | 100 | 40 | 100000 | 16.3 | 0.49× | book_held=pass, layout_did_work=pass, text_did_work=pass, visible_window_indices=pass |
## Workload — `wrap-text-200`

| framework | iterations | median (µs) | p95 (µs) | cv | hits | misses | first | visible | book | rss (MiB) | vs gpui | equivalence |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---|
| gpui | 16 | 649.12 | 753.38 | 0.116 | 0 | 0 | 0 | 0 | 0 | 16.4 | 1.00× | layout_did_work=pass, text_did_work=pass |
| sigil | 16 | 247.08 | 277.25 | 0.063 | 0 | 16 | 0 | 0 | 0 | 19.2 | 0.38× | layout_did_work=pass, text_did_work=pass |
