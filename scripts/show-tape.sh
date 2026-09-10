#!/bin/sh
set -eu
root="$(CDPATH= cd -- "$(dirname "$0")/.." && pwd)"
cd "$root"
rows="${ACCESS_LOG_ROWS:-1000000}"
export ACCESS_LOG_ROWS="$rows"
echo "book $rows lines"
cargo build --release --offline -p access-log --bin access-log-sigil
cargo build --release --offline -p access-log --bin access-log-gpui --features gpui-window
cargo run --release --offline -p access-log --bin access-log-sigil &
cargo run --release --offline -p access-log --bin access-log-gpui --features gpui-window &
echo "two windows. compare the big µs line. Tail jumps the million."
wait
