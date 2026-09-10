#!/bin/sh
set -eu
root="$(CDPATH= cd -- "$(dirname "$0")/.." && pwd)"
cd "$root"
rounds="${1:-5}"
out="${2:-$root/target/bench-records/ledger}"
mkdir -p "$out"

cargo build --release --locked -p ledger --bin ledger
cargo build --release --locked -p bench-gpui --bin ledger-gpui
cargo build --release --locked -p bench-spec --bin bench-report

sigil="$root/target/release/ledger"
gpui="$root/target/release/ledger-gpui"

round=1
while [ "$round" -le "$rounds" ]; do
  if [ $((round % 2)) -eq 1 ]; then
    "$sigil" --bench "$out/sigil-$round.json"
    "$gpui" "$out/gpui-$round.json"
  else
    "$gpui" "$out/gpui-$round.json"
    "$sigil" --bench "$out/sigil-$round.json"
  fi
  round=$((round + 1))
done

"$root/target/release/bench-report" "$out" "$out/REPORT.md"
echo "report: $out/REPORT.md"
