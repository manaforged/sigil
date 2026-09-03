# Benchmarks

Every published Sigil number comes from a real window on named hardware,
with the exact command that produced it. Nothing in this document is a
headless measurement, and nothing compares frameworks under different
conditions.

## What is measured

The **Ledger** scenario (`bench-spec::ledger`) is a 10 000 × 24 grid with
a live price stream. A driver fires every 16 ms. Each pulse ticks the
stream (every 40th row moves) and, on scheduled pulses, applies one
interaction from a fixed script:

| Pulses | Interaction |
| --- | --- |
| 0–299 | Scroll 2 rows per pulse |
| 300, 360 | Sort by symbol, then by last done |
| 400–549 | Scroll 8 rows per pulse |
| 560, 640 | Quick filter `AB`, then clear |
| 660–859 | Five select → edit → commit → undo cycles |
| 860, 890 | Theme toggle, twice |

The metric is the interval between consecutive pulses. A frame that
overruns delays the next pulse, so the interval distribution shows what
a user sees: p50, p90, p95, p99, max, and the count of intervals above
16.67 ms (60 Hz) and 8.33 ms (120 Hz).

Every run ends with functional checks against the expected end state:
pulse count, tick count, cells moved, edits committed, undo depth, rows
visible, filter hits, scroll distance. A record with a failed check is
not comparable and the report marks it `FAIL`.

## Runners

| Framework | Command | Notes |
| --- | --- | --- |
| Sigil | `cargo run --release -p ledger -- --bench out.json` | `examples/ledger`, the product app in bench mode |
| GPUI 0.2.2 | `cargo run --release -p bench-gpui --bin ledger-gpui -- out.json` | Row chrome from GPUI's own `examples/data_table.rs`; inline edit is a modeled cell because GPUI ships no text input widget |

Each runner uses its framework's production constructors, its own text
stack, and its default GPU backend on the host (wgpu for Sigil; Metal on
macOS for GPUI). Those differences are part of the product and are
recorded in every record (`backend` field).

## Reproduce

```sh
scripts/bench-ledger.sh 5
```

This builds both runners, alternates framework order for five rounds,
and writes `target/bench-records/ledger/REPORT.md`. Run it on an idle
machine with the window unobstructed. Records are `record-v1` JSON
(`crates/bench-spec/src/record.rs`) with host, commit, backend, raw
intervals, summary, and checks.

## What is not claimed

- No number from a headless or test-support build. Sigil's headless
  harnesses (`crates/bench-parity`, `examples/ledger` tests) are
  regression tools for Sigil only and never appear in a comparison.
- No cross-framework pixel equivalence. Text stacks differ; the checks
  are functional.
- No aggregate "N× faster". Quote the percentile and the overrun count
  from a report you can rerun.
