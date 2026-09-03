# sigil-bench-parity

Sigil-only headless regression harness. Locked workloads
(`workloads/*.json`), one runner (`SigilRunner`), a `RunResult` JSON
per run, and an aggregator that renders a directory of results as a
markdown table.

This crate does not compare frameworks. Cross-framework numbers come
only from real windows: see `docs/benchmarks.md` and
`scripts/bench-ledger.sh` (`examples/ledger --bench` and
`crates/bench-gpui`). The former GPUI test-support runner was removed
because a headless GPUI build is not the product GPUI ships, so it could
not be a fair comparison.

## Running

```text
cargo run --locked --offline -p bench-parity --release --bin bench-parity-runner -- \
  --framework sigil \
  --workload crates/bench-parity/workloads/data-table-10k-tick.json \
  --output target/bench-parity/sigil-data-table-10k-tick.json

cargo run --locked --offline -p bench-parity --release --bin bench-parity-aggregator -- \
  --results-dir target/bench-parity \
  --output target/bench-parity/REPORT.md
```

Workloads: `virtual-list-100k-krausest`, `virtual-list-1m-krausest`
(1M book, top window), `virtual-list-1m-krausest-tail` (last page),
`virtual-list-10m-krausest` / `-tail` (10M book),
`data-table-10k` (construct + layout + shape), `data-table-10k-remount`
(keyed row reuse + incoming row shape), `data-table-10k-paint` (reuse +
CPU emit), `data-table-10k-tick` (in-place last-done updates),
`data-table-10k-tick-paint` (tick + CPU emit), `flex-tree-2k`,
`text-labels-2k`, `wrap-text-200`, `kv-form-500`, `nested-flex-64`,
`overlay-stack-100`.

A `VOID` cell, or a failed `text_did_work` / `scene_did_work` check,
means the timer did not do the named work — do not quote that row.
Sigil `TEXT_MEMO` is cleared on a new engine so construct cannot skip
shaping.

## Adding a new workload


1. Drop a JSON file under `workloads/`.
2. The schema is the `Workload` struct in `src/lib.rs` — `workload_id`,
   `description`, `category`, typed `params`, and a list of
   `equivalence_checks`.
3. If you introduce a new `category`, every existing
   `FrameworkRunner` must return either a real `RunResult` or an
   explicit error for the new category — silently returning empty
   samples would muddy the comparison.

## Output-equivalence rule

**Every "X faster than Y" claim requires matching canonical output and
passing checks.** The
canonical output records root geometry, workload-specific counts,
and visible text from an untimed verification frame. Old result JSON
without output or source provenance cannot produce a ratio.
