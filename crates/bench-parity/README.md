# sigil-bench-parity

Cross-framework bench-parity harness for Sigil and the frameworks we
want to compare against (GPUI's `uniform_list`, TanStack-Virtual,
etc.). The crate exists so every "X faster than Y" claim is grounded
in identical measured work on the same hardware with a passing
output-equivalence check, per the the author verification-honesty rule.

GPUI and Sigil both ship. Timed surfaces match per workload (`layout`
or `layout_paint`). TanStack is not in this crate.

## Why it exists separately from `cargo bench`

`crates/sigil/benches/virtual_list.rs` is a `criterion` bench against
the Sigil widget API; that is the right tool for tracking Sigil
regressions inside this repo. It cannot measure other frameworks.

`sigil-bench-parity` is the substrate that does cross-framework runs:

- One JSON workload spec, multiple framework runners.
- Each runner returns `RunResult` (samples, summary, canonical output,
  source revision, check verdicts, and host info) serialised to JSON.
- The aggregator turns a directory of `RunResult` JSON files into a
  markdown report with one row per framework per workload. `vs gpui`
  is a number only when [`comparable`](src/compare.rs) holds: same
  clean source revision, canonical output, reuse/tick mode, host, and
  verdicts (Pass vs N/A is not a pair), with both CVs ≤ 0.25. Empty
  `equivalence_checks` is not a pass.

The crate is intentionally **not in `default-members`** of the
workspace — when GPUI and TanStack runners arrive they will drag
heavy dep trees we do not want polluting normal `cargo build` /
`cargo test` runs at the workspace root.

## Running the Sigil runner today

```text
cargo run --locked --offline -p bench-parity --release --bin bench-parity-runner -- \
  --framework sigil \
  --workload crates/bench-parity/workloads/virtual-list-100k-krausest.json

cargo run --locked --offline -p bench-parity-gpui --release --bin bench-parity-runner-gpui -- \
  --framework gpui \
  --workload crates/bench-parity/workloads/virtual-list-100k-krausest.json

cargo run --locked --offline -p bench-parity --release --bin bench-parity-aggregator -- \
  --results-dir crates/bench-parity/results \
  --output crates/bench-parity/results/REPORT.md
```

Workloads: `virtual-list-100k-krausest`, `virtual-list-1m-krausest`
(1M book, top window), `virtual-list-1m-krausest-tail` (last page),
`virtual-list-10m-krausest` / `-tail` (10M book),
`data-table-10k` (construct
+ layout + shape), `data-table-10k-remount` (keyed row reuse + incoming
row shape), `data-table-10k-paint` (reuse + CPU emit),
`data-table-10k-tick` (in-place last-done updates),
`data-table-10k-tick-paint` (tick + CPU emit), `flex-tree-2k`,
`text-labels-2k`, `wrap-text-200`, `kv-form-500`, `nested-flex-64`,
`overlay-stack-100`.

Quote `results/data-table/REPORT.md` for the 24-field GPUI
`data_table` clone. `results/REPORT.md` is list / flex / labels
only. A `data-table-*` row without `column_count=pass` cannot
print `vs gpui`. `results/stale-construct/` is the old window-only
table JSON — do not aggregate it.

Quote only a numeric `vs gpui` or `vs frame` cell. `vs gpui` is
1:1 same mode. `vs frame` is Sigil retained `compute` versus GPUI
per-sample reconstruct — a 0.2.2 app frame, not 1:1 remount.
`VOID`, `NOISY`, `— source`, `— output`, `— mode`, `— checks`, and
`— host` are not speedups.

The 1M set is `./scripts/bench-1m.sh` → `results/1m/REPORT.md`.
A 1M row without `book_held=pass` is a 40-row window, not a million-row
comparison. Tail also needs `tail_window=pass` (last source row is
on screen).

The data_table set is the claim people can rerun:

```text
./scripts/bench-data-table.sh
```

That script writes `crates/bench-parity/results/data-table/REPORT.md`.
A `VOID` cell, or a failed `text_did_work` / `scene_did_work` check,
means the timer did not do the named work — do not quote the ratio.
Both runners use `data_table::{generate, FIELDS, cell_text}`. GPUI
rows match `examples/data_table.rs` TableRow chrome (zebra, tint,
border, nowrap). Sigil `TEXT_MEMO` is cleared on a new engine so
construct cannot skip shaping.

## Adding a new workload

1. Drop a JSON file under `workloads/`.
2. The schema is the `Workload` struct in `src/lib.rs` — `workload_id`,
   `description`, `category`, typed `params`, and a list of
   `equivalence_checks`.
3. If you introduce a new `category`, every existing
   `FrameworkRunner` must return either a real `RunResult` or an
   explicit error for the new category — silently returning empty
   samples would muddy the comparison.

## Adding a new framework runner

1. Create `src/runners/<name>_runner.rs` and `impl FrameworkRunner`
   for a fresh stateless type. The `run` method must:
   - dispatch on `workload.category` (error for unsupported);
   - run the warmup + measured iterations per the workload params;
   - populate every declared equivalence check (Pass / Fail /
     NotApplicable);
   - return a fully-populated `RunResult`.
2. Re-export the type from `src/runners/mod.rs`.
3. Register it in `src/bin/bench-parity-runner.rs`'s `dispatch` match.
4. If the framework's dependency tree is heavy (GPUI), keep it gated
   behind a cargo feature so the default build of this crate stays
   light.

## Output-equivalence rule

Per global the contributor guide verification honesty: **every "X faster than Y"
claim requires matching canonical output and passing checks.** The
canonical output records root geometry, workload-specific counts,
and visible text from an untimed verification frame. Old result JSON
without output or source provenance cannot produce a ratio.

## Defendable 1:1 (quote this)

GPUI 0.2.2 reconstructs the element tree every sample. The only 1:1
pair is **both sides construct**: Sigil `new` engine + `add` +
`compute` per sample, GPUI `uniform_list` / `div` + `layout_as_root` +
`prepaint` per sample. Retain and tick are not 1:1 with GPUI 0.2.2.

```text
./scripts/bench-1to1.sh
```

Writes `crates/bench-parity/results/one-to-one/REPORT.md`. The
worktree must be clean. The aggregator `--one-to-one` flag drops
retain/tick rows, omits `vs frame`, and exits 1 unless every pair
prints a numeric `vs gpui`. Quote only that file. Name crates.io
GPUI 0.2.2, CPU layout, and `.overscan(0)` in the same sentence.

## 1:1 frame (binding)

Host setup is never timed (`TestAppContext`, `Cx`, text engine).

Each sample clocks **construct the frame tree + layout it**:

| | GPUI 0.2.2 | Sigil |
|---|---|---|
| construct | `uniform_list` / `div` tree | `cx.add(virtual_list \| container)` |
| layout | `VisualTestContext::draw` through prepaint | `LayoutEngine::compute` |

The GPUI runner uses a timing element inside the supported draw path.
It starts the clock before element construction and stops after
prepaint. It does not call GPUI layout methods outside a draw phase.

`uniform_list` materialises visible rows in `prepaint`. Sigil
materialises them in `add` + `compute` (`items_bind`). Same viewport,
row height, cell sizes (`ID_CELL_W` / `TABLE_CELL_W`), and label
strings. 1:1 construct benches call `.overscan(0)` so Sigil mounts
the geometric page only — the same window as `uniform_list`. Product
lists keep the default of one page of overscan each side. Construct
pairs require equal `visible_last`. A 76-vs-38 pair prints `— visible`.
Retain still uses the overscan band. A ratio also requires the exact
same viewport rows and text.

`vs gpui` is `sigil_median / gpui_median`. Printed only when
`comparable` holds. A Sigil construct run with
`layout_hits == samples` is void — that sample did not remasure.
GPUI stamps `reuse=0` on every run (fresh element tree). A Sigil
`reuse=1` remount or tick cannot pair with it.

## Hardware capture

Every `RunResult` carries `HostInfo` (OS + CPU brand + core count +
total memory + rustc version). The aggregator emits one
`## Hardware` block from the first result loaded. Cross-host result
mixing is a category error — keep separate `results/` dirs per host.
