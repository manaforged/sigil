#!/bin/sh
# flex / labels / 100k list. Writes JSON + REPORT under results/current/.
set -eu
root="$(CDPATH= cd -- "$(dirname "$0")/.." && pwd)"
cd "$root"
out="$root/crates/bench-parity/results/current"
mkdir -p "$out"
workloads="flex-tree-2k text-labels-2k virtual-list-100k-krausest"

for id in $workloads; do
  spec="$root/crates/bench-parity/workloads/${id}.json"
  cargo run --release --offline --locked -p bench-parity --bin bench-parity-runner -- \
    --framework sigil --workload "$spec" --output "$out/sigil-${id}.json"
  cargo run --release --offline --locked -p bench-parity-gpui --bin bench-parity-runner-gpui -- \
    --framework gpui --workload "$spec" --output "$out/gpui-${id}.json"
done

cargo run --release --offline --locked -p bench-parity --bin bench-parity-aggregator -- \
  --results-dir "$out" --output "$root/crates/bench-parity/results/REPORT.md"

echo
echo "report: $root/crates/bench-parity/results/REPORT.md"
echo "Quote only a numeric vs-gpui cell. VOID / NOISY / — mode / — checks / — host are not speedups."
