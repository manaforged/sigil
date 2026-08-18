#!/bin/sh
# 1:1 data_table benches. Writes JSON + REPORT under results/data-table/.
# VOID if Sigil layout_hits == samples. Re-run after any table change.
set -eu
root="$(CDPATH= cd -- "$(dirname "$0")/.." && pwd)"
cd "$root"
out="$root/crates/bench-parity/results/data-table"
mkdir -p "$out"
workloads="data-table-10k data-table-10k-remount data-table-10k-paint data-table-10k-tick data-table-10k-tick-paint"

for id in $workloads; do
  spec="$root/crates/bench-parity/workloads/${id}.json"
  cargo run --release --offline --locked -p bench-parity --bin bench-parity-runner -- \
    --framework sigil --workload "$spec" --output "$out/sigil-${id}.json"
  cargo run --release --offline --locked -p bench-parity-gpui --bin bench-parity-runner-gpui -- \
    --framework gpui --workload "$spec" --output "$out/gpui-${id}.json"
done

cargo run --release --offline --locked -p bench-parity --bin bench-parity-aggregator -- \
  --results-dir "$out" --output "$out/REPORT.md"

echo
echo "report: $out/REPORT.md"
echo "Quote only a numeric vs-gpui cell. VOID / NOISY / — mode / — checks / — host are not speedups."
