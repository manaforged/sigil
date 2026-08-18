#!/bin/sh
# Print the 1:1 data_table commands and open the profiler compare page.
# Does not start two GPU processes — tile the native windows yourself.
set -eu
root="$(CDPATH= cd -- "$(dirname "$0")/.." && pwd)"
page="$root/showcase/compare/data-table.html"
gpui_toml="$(ls -d "${CARGO_HOME:-$HOME/.cargo}"/registry/src/index.crates.io-*/gpui-0.2.2/Cargo.toml 2>/dev/null | head -n 1 || true)"

echo "Sigil window:"
echo "  SIGIL_PROFILE=$root/sigil-data-table.json cargo run --profile profiling -p data-table"
echo
echo "GPUI window:"
if [ -n "$gpui_toml" ]; then
  echo "  cargo run --release --example data_table --manifest-path $gpui_toml"
else
  echo "  cargo run --release --example data_table --manifest-path \$CARGO_HOME/registry/src/index.crates.io-*/gpui-0.2.2/Cargo.toml"
fi
echo
echo "Compare page: $page"
echo "Drop traces into the two columns after you quit both apps."

if command -v open >/dev/null 2>&1; then
  open "$page"
elif command -v xdg-open >/dev/null 2>&1; then
  xdg-open "$page"
fi
