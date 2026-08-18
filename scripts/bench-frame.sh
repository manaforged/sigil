#!/bin/sh
# Live-frame family. Sigil retained compute vs GPUI 0.2.2 reconstruct.
# Quote the vs-frame column. Do not quote vs frame as 1:1 remount.
# Writes JSON + REPORT under results/frame/.
set -eu
root="$(CDPATH= cd -- "$(dirname "$0")/.." && pwd)"
cd "$root"
out="$root/crates/bench-parity/results/frame"
mkdir -p "$out"
workloads="virtual-list-100k-krausest-scroll virtual-list-1m-krausest-scroll virtual-list-10m-krausest-scroll data-table-10k-remount data-table-10k-tick"

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
echo "Quote only a numeric vs-frame cell. vs gpui is — mode on this set."
