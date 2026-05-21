# sigil-bench-parity

Cross-framework bench-parity harness for Sigil and the frameworks we
want to compare against (GPUI's `uniform_list`, TanStack-Virtual,
etc.). The crate exists so every "X faster than Y" claim is grounded
in identical measured work on the same hardware with a passing
output-equivalence check, per the the author verification-honesty rule.

This is the SCAFFOLD — only the Sigil runner ships today. GPUI and
TanStack runners land in follow-up dispatches.

## Why it exists separately from `cargo bench`

`crates/sigil/benches/virtual_list.rs` is a `criterion` bench against
the Sigil widget API; that is the right tool for tracking Sigil
regressions inside this repo. It cannot measure other frameworks.

`sigil-bench-parity` is the substrate that does cross-framework runs:

- One JSON workload spec, multiple framework runners.
- Each runner returns `RunResult` (samples + summary + equivalence
  check verdicts + host info) serialised to JSON.
- The aggregator turns a directory of `RunResult` JSON files into a
  markdown report with one row per framework per workload.

The crate is intentionally **not in `default-members`** of the
workspace — when GPUI and TanStack runners arrive they will drag
heavy dep trees we do not want polluting normal `cargo build` /
`cargo test` runs at the workspace root.

## Running the Sigil runner today

```powershell
$env:CARGO_TARGET_DIR='target\bench-parity'
cargo run -p sigil-bench-parity --release --bin bench-parity-runner -- `
  --framework sigil `
  --workload crates/sigil-bench-parity/workloads/virtual-list-100k-krausest.json
```

That writes a JSON file under
`crates/sigil-bench-parity/results/sigil-virtual-list-100k-krausest-<utc>.json`.

Then render the report:

```powershell
cargo run -p sigil-bench-parity --release --bin bench-parity-aggregator -- `
  --results-dir crates/sigil-bench-parity/results `
  --output crates/sigil-bench-parity/results/REPORT.md
```

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
claim REQUIRES a matching equivalence_check pass on the same
RunResult.** The aggregator refuses to emit comparative prose
across frameworks whose equivalence checks did not all pass. Today
the aggregator surfaces per-framework rows + equivalence verdicts
verbatim and leaves any "vs" prose to a human-curated long-form
analysis doc citing these rows.

## GPUI runner — shipped, with a methodology caveat

The first GPUI runner ships in `crates/sigil-bench-parity-gpui/`,
gated on GPUI 0.2.2 + `test-support` (a headless path published by
the Zed team for their own test suite). The crate is a workspace
member but NOT in `default-members`, so GPUI's heavy deps stay out
of normal Sigil builds.

**Surface caveat (important for honest reporting):** the two runners
currently time DIFFERENT boundaries.

| framework | what's actually inside the `Instant::now()` window |
|---|---|
| sigil  | `Cx::add(virtual_list) + LayoutEngine::compute` — pure layout pass. |
| gpui   | `TestAppContext::build + add_window + VisualTestContext::draw` — layout + initial paint attempt + headless-paint panic + `catch_unwind`. |

GPUI's `VisualTestContext::draw` is the smallest published surface that
exercises `uniform_list`'s layout from outside an event loop. It runs
layout, then attempts to descend into paint, then panics because
headless mode has no GPU surface. The runner catches the panic and
counts the time up to the abort. So the GPUI number is inflated by
test-harness setup + paint-attempt machinery + unwind cost — magnitude
unknown but non-zero.

Until this is reconciled, the aggregator table should be read as
"both frameworks complete the same workload in the same order of
magnitude on this hardware" — NOT as a clean "X% faster than Y" claim.

**Follow-ups to close the methodology gap:**

1. Find a GPUI hook that runs layout without descending into paint.
   If `gpui::Element::layout` is accessible from `TestAppContext`,
   route the runner through it directly.
2. If GPUI doesn't expose layout-only, instead **widen Sigil's
   measurement** to match (include the equivalent paint-setup work)
   so both runners time the same surface.
3. Tighten equivalence: current `visible_window_indices` accepts ±1
   row count because Sigil renders with a 1-row over-scroll buffer
   and GPUI does not. Either configure Sigil to drop the buffer for
   this workload, or assert exact materialised-index set on both
   sides.
4. Increase iterations to 500 + warmup to 50, add coefficient-of-
   variation to the summary so tail jitter is legible.

## Hardware capture

Every `RunResult` carries `HostInfo` (OS + CPU brand + core count +
total memory + rustc version). The aggregator emits one
`## Hardware` block from the first result loaded. Cross-host result
mixing is a category error — keep separate `results/` dirs per host.
