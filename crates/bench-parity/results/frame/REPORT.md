# Bench-parity report

Per-framework measurements on locked workloads. A numeric `vs gpui` cell requires matching reuse/tick, host, and verdicts. A numeric `vs frame` cell is retained compute versus reconstruct — not the same mode.

Host setup is untimed. `vs gpui` is a number only when both sides stamp the same reuse/tick mode, the same host (OS/CPU/rustc), the same verdicts (Pass vs N/A is not a pair), and both CVs stay at or under 0.25. `vs frame` is a number when both sides pass `ratio_ok`, share that host, both CVs stay at or under 0.25, and reuse is complementary (1 vs 0) with the same tick flag. Do not quote `vs frame` as 1:1 remount. Krausest rows carry id/label/remove text. Tick requires last_done_did_move. An empty check map is not a pass. VOID if construct recorded all layout hits. NOISY if either CV is above the ceiling.

## Hardware

- OS: Darwin 26.5.1
- CPU: Apple M2 Max (12 cores)
- Memory: 32.00 GiB
- rustc: rustc 1.97.1 (8bab26f4f 2026-07-14)

## Workload — `data-table-10k-remount`

| framework | iterations | median (µs) | p95 (µs) | cv | hits | misses | first | visible | book | rss (MiB) | vs gpui | vs frame | equivalence |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---|
| gpui | 40 | 3215.71 | 3795.08 | 0.092 | 0 | 0 | 46 | 38 | 10000 | 474.9 | — mode | 1.00× | book_held=pass, column_count=pass, layout_did_work=pass, scene_did_work=n/a, text_did_work=pass, visible_window_indices=pass |
| sigil | 40 | 730.58 | 848.42 | 0.096 | 0 | 40 | 0 | 38 | 10000 | 30.5 | — mode | 0.23× | book_held=pass, column_count=pass, layout_did_work=pass, scene_did_work=n/a, text_did_work=pass, visible_window_indices=pass |
## Workload — `data-table-10k-tick`

| framework | iterations | median (µs) | p95 (µs) | cv | hits | misses | first | visible | book | rss (MiB) | vs gpui | vs frame | equivalence |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---|
| sigil | 80 | 9.38 | 10.96 | 0.067 | 80 | 0 | 0 | 38 | 10000 | 26.2 | — mode | 0.003× | book_held=pass, column_count=pass, identity_did_hold=pass, last_done_did_move=pass, layout_did_work=pass, scene_did_work=n/a, text_did_work=pass, visible_window_indices=pass |
| gpui | 80 | 3373.71 | 4054.21 | 0.099 | 0 | 0 | 0 | 38 | 10000 | 1019.1 | — mode | 1.00× | book_held=pass, column_count=pass, identity_did_hold=n/a, last_done_did_move=pass, layout_did_work=pass, scene_did_work=n/a, text_did_work=pass, visible_window_indices=pass |
## Workload — `virtual-list-100k-krausest-scroll`

| framework | iterations | median (µs) | p95 (µs) | cv | hits | misses | first | visible | book | rss (MiB) | vs gpui | vs frame | equivalence |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---|
| gpui | 80 | 582.00 | 769.75 | 0.146 | 0 | 0 | 90 | 40 | 100000 | 165.0 | — mode | 1.00× | book_held=pass, layout_did_work=pass, text_did_work=pass, visible_window_indices=pass |
| sigil | 80 | 92.88 | 110.00 | 0.134 | 0 | 80 | 90 | 40 | 100000 | 15.6 | — mode | 0.16× | book_held=pass, layout_did_work=pass, text_did_work=pass, visible_window_indices=pass |
## Workload — `virtual-list-10m-krausest-scroll`

| framework | iterations | median (µs) | p95 (µs) | cv | hits | misses | first | visible | book | rss (MiB) | vs gpui | vs frame | equivalence |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---|
| gpui | 40 | 607.62 | 732.96 | 0.173 | 0 | 0 | 50 | 40 | 10000000 | 135.2 | — mode | 1.00× | book_held=pass, layout_did_work=pass, text_did_work=pass, visible_window_indices=pass |
| sigil | 40 | 97.04 | 104.00 | 0.027 | 0 | 40 | 50 | 40 | 10000000 | 53.0 | — mode | 0.16× | book_held=pass, layout_did_work=pass, text_did_work=pass, visible_window_indices=pass |
## Workload — `virtual-list-1m-krausest-scroll`

| framework | iterations | median (µs) | p95 (µs) | cv | hits | misses | first | visible | book | rss (MiB) | vs gpui | vs frame | equivalence |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---|
| sigil | 80 | 86.08 | 103.04 | 0.184 | 0 | 80 | 90 | 40 | 1000000 | 19.1 | — mode | 0.15× | book_held=pass, layout_did_work=pass, text_did_work=pass, visible_window_indices=pass |
| gpui | 80 | 589.38 | 735.17 | 0.131 | 0 | 0 | 90 | 40 | 1000000 | 169.0 | — mode | 1.00× | book_held=pass, layout_did_work=pass, text_did_work=pass, visible_window_indices=pass |
