# Bench-parity report

Per-framework measurements on locked workloads. A numeric `vs gpui` cell requires matching reuse/tick, host, and verdicts. A numeric `vs frame` cell is retained compute versus reconstruct — not the same mode.

Host setup is untimed. `vs gpui` is a number only when both sides stamp the same reuse/tick mode, the same host (OS/CPU/rustc), the same verdicts (Pass vs N/A is not a pair), and both CVs stay at or under 0.25. `vs frame` is a number when both sides pass `ratio_ok`, share that host, both CVs stay at or under 0.25, and reuse is complementary (1 vs 0) with the same tick flag. Do not quote `vs frame` as 1:1 remount. Krausest rows carry id/label/remove text. Tick requires last_done_did_move. An empty check map is not a pass. VOID if construct recorded all layout hits. NOISY if either CV is above the ceiling.

## Hardware

- OS: Darwin 26.5.1
- CPU: Apple M2 Max (12 cores)
- Memory: 32.00 GiB
- rustc: rustc 1.97.1 (8bab26f4f 2026-07-14)

## Workload — `virtual-list-100k-krausest`

| framework | iterations | median (µs) | p95 (µs) | cv | hits | misses | first | visible | book | rss (MiB) | vs gpui | vs frame | equivalence |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---|
| gpui | 80 | 463.83 | 601.21 | 0.149 | 0 | 0 | 100 | 40 | 100000 | 14.7 | 1.00× | — | book_held=pass, layout_did_work=pass, text_did_work=pass, visible_window_indices=pass |
| sigil | 80 | 214.79 | 302.54 | 0.174 | 0 | 80 | 100 | 40 | 100000 | 16.2 | 0.46× | — | book_held=pass, layout_did_work=pass, text_did_work=pass, visible_window_indices=pass |
## Workload — `virtual-list-100k-krausest-scroll`

| framework | iterations | median (µs) | p95 (µs) | cv | hits | misses | first | visible | book | rss (MiB) | vs gpui | vs frame | equivalence |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---|
| gpui | 80 | 294.75 | 437.38 | 0.210 | 0 | 0 | 90 | 40 | 100000 | 95.2 | — mode | — checks | book_held=pass, layout_did_work=n/a, visible_window_indices=pass |
| sigil | 80 | 18.54 | 20.25 | 0.053 | 0 | 80 | 90 | 40 | 100000 | 11.2 | — mode | — checks | book_held=pass, layout_did_work=pass, visible_window_indices=pass |
## Workload — `virtual-list-10m-krausest`

| framework | iterations | median (µs) | p95 (µs) | cv | hits | misses | first | visible | book | rss (MiB) | vs gpui | vs frame | equivalence |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---|
| gpui | 40 | 209.29 | 236.12 | 0.173 | 0 | 0 | 46 | 40 | 10000000 | 51.2 | — checks | — | book_held=pass, visible_window_indices=pass |
| sigil | 40 | 99.67 | 110.88 | 0.069 | 0 | 40 | 46 | 40 | 10000000 | 49.4 | — checks | — | book_held=pass, visible_window_indices=pass |
## Workload — `virtual-list-10m-krausest-scroll`

| framework | iterations | median (µs) | p95 (µs) | cv | hits | misses | first | visible | book | rss (MiB) | vs gpui | vs frame | equivalence |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---|
| gpui | 40 | 237.25 | 312.54 | 0.121 | 0 | 0 | 50 | 40 | 10000000 | 95.0 | — mode | — checks | book_held=pass, layout_did_work=n/a, visible_window_indices=pass |
| sigil | 40 | 20.83 | 23.00 | 0.041 | 0 | 40 | 50 | 40 | 10000000 | 49.0 | — mode | — checks | book_held=pass, layout_did_work=pass, visible_window_indices=pass |
## Workload — `virtual-list-10m-krausest-tail`

| framework | iterations | median (µs) | p95 (µs) | cv | hits | misses | first | visible | book | rss (MiB) | vs gpui | vs frame | equivalence |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---|
| sigil | 40 | 145.38 | 152.62 | 0.206 | 0 | 40 | 9999960 | 40 | 10000000 | 49.3 | — checks | — | book_held=pass, tail_window=pass, visible_window_indices=pass |
| gpui | 40 | 152.62 | 172.62 | 0.056 | 0 | 0 | 9999960 | 40 | 10000000 | 51.0 | — checks | — | book_held=pass, tail_window=pass, visible_window_indices=pass |
## Workload — `virtual-list-1m-krausest`

| framework | iterations | median (µs) | p95 (µs) | cv | hits | misses | first | visible | book | rss (MiB) | vs gpui | vs frame | equivalence |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---|
| sigil | 40 | 215.42 | 254.33 | 0.076 | 0 | 40 | 46 | 40 | 1000000 | 18.8 | 0.49× | — | book_held=pass, layout_did_work=pass, text_did_work=pass, visible_window_indices=pass |
| gpui | 40 | 438.83 | 476.75 | 0.044 | 0 | 0 | 46 | 40 | 1000000 | 17.7 | 1.00× | — | book_held=pass, layout_did_work=pass, text_did_work=pass, visible_window_indices=pass |
## Workload — `virtual-list-1m-krausest-scroll`

| framework | iterations | median (µs) | p95 (µs) | cv | hits | misses | first | visible | book | rss (MiB) | vs gpui | vs frame | equivalence |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---|
| sigil | 40 | 19.12 | 23.46 | 0.104 | 0 | 40 | 46 | 40 | 1000000 | 14.6 | — mode | — checks | book_held=pass, layout_did_work=pass, visible_window_indices=pass |
| gpui | 40 | 240.50 | 329.96 | 0.160 | 0 | 0 | 46 | 40 | 1000000 | 58.2 | — mode | — checks | book_held=pass, layout_did_work=n/a, visible_window_indices=pass |
## Workload — `virtual-list-1m-krausest-tail`

| framework | iterations | median (µs) | p95 (µs) | cv | hits | misses | first | visible | book | rss (MiB) | vs gpui | vs frame | equivalence |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---|
| gpui | 40 | 148.67 | 167.88 | 0.058 | 0 | 0 | 999960 | 40 | 1000000 | 16.6 | — checks | — | book_held=pass, tail_window=pass, visible_window_indices=pass |
| sigil | 40 | 100.54 | 110.33 | 0.088 | 0 | 40 | 999960 | 40 | 1000000 | 15.0 | — checks | — | book_held=pass, tail_window=pass, visible_window_indices=pass |
