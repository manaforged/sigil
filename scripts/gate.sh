#!/usr/bin/env bash
set -euo pipefail

root=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
cd "$root"

gpu=0
for arg in "$@"; do
  case "$arg" in
    --gpu) gpu=1 ;;
    *) echo "gate: unknown argument $arg (usage: scripts/gate.sh [--gpu])" >&2; exit 2 ;;
  esac
done

msrv=$(sed -n 's/^rust-version = "\(.*\)"/\1/p' Cargo.toml)
started=$SECONDS

step() {
  local name=$1
  shift
  local at=$SECONDS
  echo "gate: $name: $*" >&2
  if ! "$@"; then
    echo "gate: FAILED at $name" >&2
    exit 1
  fi
  echo "gate: $name ok ($((SECONDS - at))s)" >&2
}

need() {
  command -v "$1" >/dev/null 2>&1 || { echo "gate: $1 is missing; install: $2" >&2; exit 1; }
}

refuse_env() {
  local name bad=0
  for name in $(node -e 'console.log(require("./scripts/gate-policy.json").refusedEnv.join(" "))'); do
    if [ -n "${!name+x}" ]; then
      echo "gate: $name is set; the gate never runs with a test escape" >&2
      bad=1
    fi
  done
  return "$bad"
}

need node "https://nodejs.org"
need cargo-hack "cargo install --locked cargo-hack"
need cargo-deny "cargo install --locked cargo-deny@0.20.2"
need cargo-fuzz "cargo install --locked cargo-fuzz@0.13.2"
need mdbook "cargo install --locked mdbook@0.4.40"
need wasm-bindgen-test-runner "cargo install --locked wasm-bindgen-cli@0.2.127"

step env refuse_env
step source node scripts/check-source.mjs
step api-doc node scripts/check-api-doc.mjs --modules
step fmt cargo fmt --all --check
step clippy cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
step features cargo hack clippy -p sigil-ui --lib --each-feature --keep-going --locked -- -D warnings
step wasm-clippy cargo clippy -p sigil-ui --lib --target wasm32-unknown-unknown --locked -- -D warnings
step wasm-test cargo test -p sigil-ui --target wasm32-unknown-unknown --locked --test tasks_wasm \
  --config 'target.wasm32-unknown-unknown.runner="wasm-bindgen-test-runner"'
step test cargo test --workspace --all-features --locked --no-fail-fast
step perf cargo test -p sigil-ui --release --features terminal --locked --test perf_gate
step rustdoc cargo doc --workspace --all-features --no-deps --locked --config 'build.rustdocflags=["-D","warnings"]'
step msrv-toolchain rustup toolchain install "$msrv" --profile minimal --no-self-update
step msrv cargo "+$msrv" check --workspace --all-targets --all-features --locked
step deny cargo deny check
step fuzz-toolchain rustup toolchain install nightly --profile minimal --no-self-update
step fuzz cargo +nightly fuzz build
step book mdbook build docs --dest-dir "$root/target/book"

if [ "$gpu" = 1 ]; then
  skips=()
  while read -r name; do skips+=(--skip "$name"); done < <(node scripts/check-source.mjs --manual-tests)
  step gpu cargo test --workspace --all-features --locked --no-fail-fast --lib --tests -- --ignored --test-threads=1 "${skips[@]}"
fi

echo "gate: passed ($((SECONDS - started))s)" >&2
