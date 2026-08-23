#!/bin/sh
# Defendable 1:1 construct vs crates.io GPUI 0.2.2.
# Both sides build the tree and layout it every sample.
# Fails if any pair has no numeric vs gpui (dirty tree, NOISY, output mismatch).
set -eu
root="$(CDPATH= cd -- "$(dirname "$0")/.." && pwd)"
cd "$root"
out="$root/crates/bench-parity/results/one-to-one"
mkdir -p "$out"
workloads="data-table-10k text-labels-2k flex-tree-2k virtual-list-100k-krausest wrap-text-200 kv-form-500 nested-flex-64 overlay-stack-100"

if [ -n "$(git -C "$root" status --porcelain -- . ':!crates/bench-parity/results')" ]; then
  echo "worktree is dirty — vs gpui will be — source. commit first." >&2
  exit 1
fi

for id in $workloads; do
  spec="$root/crates/bench-parity/workloads/${id}.json"
  cargo run --release --offline --locked -p bench-parity --bin bench-parity-runner -- \
    --framework sigil --workload "$spec" --output "$out/sigil-${id}.json"
  cargo run --release --offline --locked -p bench-parity-gpui --bin bench-parity-runner-gpui -- \
    --framework gpui --workload "$spec" --output "$out/gpui-${id}.json"
done

cargo run --release --offline --locked -p bench-parity --bin bench-parity-aggregator -- \
  --one-to-one --results-dir "$out" --output "$out/REPORT.md"

echo
echo "report: $out/REPORT.md"
echo "Quote only a numeric vs-gpui cell from this file."
