# Bench-parity report

Per-framework measurements on locked workloads. A numeric `vs gpui` cell requires matching reuse/tick, host, and verdicts. A numeric `vs frame` cell is retained compute versus reconstruct — not the same mode.

Host setup is untimed. `vs gpui` is a number only when both sides stamp the same reuse/tick mode, the same host (OS/CPU/rustc), the same verdicts (Pass vs N/A is not a pair), and both CVs stay at or under 0.25. `vs frame` is a number when both sides pass `ratio_ok`, share that host, both CVs stay at or under 0.25, and reuse is complementary (1 vs 0) with the same tick flag. Do not quote `vs frame` as 1:1 remount. An empty check map is not a pass. VOID if construct recorded all layout hits. NOISY if either CV is above the ceiling.

## Hardware

- OS: Darwin 26.5.1
- CPU: Apple M2 Max (12 cores)
- Memory: 32.00 GiB
- rustc: rustc 1.97.1 (8bab26f4f 2026-07-14)

## Workload — `data-table-10k`

| framework | iterations | median (µs) | p95 (µs) | cv | hits | misses | first | visible | book | rss (MiB) | vs gpui | vs frame | equivalence |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---|
| gpui | 40 | 2705.88 | 3258.79 | 0.095 | 0 | 0 | 46 | 38 | 10000 | 29.4 | 1.00× | — | book_held=pass, column_count=pass, layout_did_work=pass, scene_did_work=n/a, text_did_work=pass, visible_window_indices=pass |
| sigil | 40 | 2220.83 | 2620.54 | 0.094 | 0 | 40 | 0 | 38 | 10000 | 57.2 | 0.82× | — | book_held=pass, column_count=pass, layout_did_work=pass, scene_did_work=n/a, text_did_work=pass, visible_window_indices=pass |
## Workload — `data-table-10k-paint`

| framework | iterations | median (µs) | p95 (µs) | cv | hits | misses | first | visible | book | rss (MiB) | vs gpui | vs frame | equivalence |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---|
| sigil | 40 | 1239.04 | 1413.96 | 0.083 | 0 | 40 | 0 | 38 | 0 | 35.1 | — mode | — checks | column_count=pass, layout_did_work=pass, scene_did_work=pass, text_did_work=pass, visible_window_indices=pass |
| gpui | 40 | 3437.21 | 3822.04 | 0.095 | 0 | 0 | 46 | 38 | 10000 | 474.7 | — mode | — checks | book_held=pass, column_count=pass, layout_did_work=pass, scene_did_work=fail, text_did_work=pass, visible_window_indices=pass |
## Workload — `data-table-10k-remount`

| framework | iterations | median (µs) | p95 (µs) | cv | hits | misses | first | visible | book | rss (MiB) | vs gpui | vs frame | equivalence |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---|
| gpui | 40 | 3498.33 | 3971.83 | 0.065 | 0 | 0 | 46 | 38 | 10000 | 475.0 | — mode | 1.00× | book_held=pass, column_count=pass, layout_did_work=pass, scene_did_work=n/a, text_did_work=pass, visible_window_indices=pass |
| sigil | 40 | 712.79 | 772.46 | 0.074 | 0 | 40 | 0 | 38 | 10000 | 30.7 | — mode | 0.20× | book_held=pass, column_count=pass, layout_did_work=pass, scene_did_work=n/a, text_did_work=pass, visible_window_indices=pass |
## Workload — `data-table-10k-tick`

| framework | iterations | median (µs) | p95 (µs) | cv | hits | misses | first | visible | book | rss (MiB) | vs gpui | vs frame | equivalence |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---|
| sigil | 80 | 7.17 | 8.79 | 0.093 | 80 | 0 | 0 | 38 | 10000 | 26.1 | — mode | 0.002× | book_held=pass, column_count=pass, identity_did_hold=pass, layout_did_work=pass, scene_did_work=n/a, text_did_work=pass, visible_window_indices=pass |
| gpui | 80 | 3529.79 | 4419.12 | 0.101 | 0 | 0 | 0 | 38 | 10000 | 1020.1 | — mode | 1.00× | book_held=pass, column_count=pass, identity_did_hold=n/a, layout_did_work=pass, scene_did_work=n/a, text_did_work=pass, visible_window_indices=pass |
## Workload — `data-table-10k-tick-paint`

| framework | iterations | median (µs) | p95 (µs) | cv | hits | misses | first | visible | book | rss (MiB) | vs gpui | vs frame | equivalence |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---|
| gpui | 40 | 3416.12 | 3994.92 | 0.085 | 0 | 0 | 0 | 38 | 10000 | 478.9 | — mode | — checks | book_held=pass, column_count=pass, identity_did_hold=n/a, layout_did_work=pass, scene_did_work=fail, text_did_work=pass, visible_window_indices=pass |
| sigil | 40 | 816.04 | 955.92 | 0.070 | 40 | 0 | 0 | 38 | 0 | 33.9 | — mode | — checks | column_count=pass, identity_did_hold=pass, layout_did_work=pass, scene_did_work=pass, text_did_work=pass, visible_window_indices=pass |
