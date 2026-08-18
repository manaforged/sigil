#!/bin/sh
# 100k / 1M / 10M krausest (+ last-page + retained scroll). Writes JSON + REPORT
# under results/1m/. Quote only a numeric vs-gpui or vs-frame cell. book_held
# must pass on the large books. Do not quote vs frame as 1:1 remount.
set -eu
root="$(CDPATH= cd -- "$(dirname "$0")/.." && pwd)"
cd "$root"
out="$root/crates/bench-parity/results/1m"
mkdir -p "$out"
workloads="virtual-list-100k-krausest virtual-list-100k-krausest-scroll virtual-list-1m-krausest virtual-list-1m-krausest-tail virtual-list-1m-krausest-scroll virtual-list-10m-krausest virtual-list-10m-krausest-tail virtual-list-10m-krausest-scroll"

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
echo "Quote only a numeric vs-gpui or vs-frame cell. 1M/10M without book_held=pass is not a speedup."
