#!/usr/bin/env bash
set -euo pipefail

root=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
cd "$root"

jobs=(lint features wasm test perf msrv fuzz publish)
usage="usage: scripts/gate.sh [--gpu] [--list] [JOB...] (jobs: ${jobs[*]})"

gpu=0
picked=()
for arg in "$@"; do
  case "$arg" in
    --gpu) gpu=1 ;;
    --list) printf '["%s"]\n' "$(IFS=,; echo "${jobs[*]}" | sed 's/,/","/g')"; exit 0 ;;
    -*) echo "gate: unknown argument $arg ($usage)" >&2; exit 2 ;;
    *) [[ " ${jobs[*]} " == *" $arg "* ]] || { echo "gate: unknown job $arg ($usage)" >&2; exit 2; }
       picked+=("$arg") ;;
  esac
done
[ ${#picked[@]} -gt 0 ] || picked=("${jobs[@]}")

outside=(benches/rivals/Cargo.toml fuzz/Cargo.toml)
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

job_lint() {
  need mdbook "cargo install --locked mdbook@0.4.40"
  need cargo-deny "cargo install --locked cargo-deny@0.20.2"
  step source node scripts/check-source.mjs
  step fmt cargo fmt --all --check
  local manifest
  for manifest in "${outside[@]}"; do
    step "fmt $manifest" cargo fmt --manifest-path "$manifest" --check
  done
  step "clippy benches/rivals" cargo clippy --manifest-path benches/rivals/Cargo.toml --locked -- -D warnings
  step deny cargo deny check
  step clippy cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
  step rustdoc env RUSTDOCFLAGS="-D warnings" cargo doc --workspace --all-features --no-deps --locked
  step book mdbook build docs --dest-dir "$root/target/book"
}

job_features() {
  need cargo-hack "cargo install --locked cargo-hack"
  step features cargo hack clippy -p sigil-ui --lib --each-feature --keep-going --locked -- -D warnings
}

job_wasm() {
  need wasm-bindgen-test-runner "cargo install --locked wasm-bindgen-cli@0.2.127"
  step wasm-clippy cargo clippy -p sigil-ui --lib --target wasm32-unknown-unknown --locked -- -D warnings
  step wasm-test cargo test -p sigil-ui --target wasm32-unknown-unknown --locked --features kittest --test tasks_wasm \
    --config 'target.wasm32-unknown-unknown.runner="wasm-bindgen-test-runner"'
}

job_test() {
  step test cargo test --workspace --all-features --locked --no-fail-fast
  step terminal-only cargo test --workspace --locked --features sigil-ui/terminal
  step dialog cargo test -p sigil-ui --locked --features dialog,kittest --test integration two_hosts::a_dialog
  if [ "$gpu" = 1 ]; then
    local skips=() name
    while read -r name; do skips+=(--skip "$name"); done < <(node scripts/check-source.mjs --manual-tests)
    step gpu cargo test --workspace --all-features --locked --no-fail-fast --lib --tests -- --ignored --test-threads=1 "${skips[@]}"
  fi
}

job_perf() {
  step perf cargo test -p sigil-ui --release --features terminal --locked --test perf_gate
}

job_msrv() {
  step msrv-toolchain rustup toolchain install "$msrv" --profile minimal --no-self-update
  step msrv cargo "+$msrv" check --workspace --all-targets --all-features --locked
}

job_fuzz() {
  need cargo-fuzz "cargo install --locked cargo-fuzz@0.13.2"
  step fuzz-toolchain rustup toolchain install nightly --profile minimal --no-self-update
  local fuzz_target
  fuzz_target=$(rustc +nightly -vV | sed -n 's/^host: //p')
  case "$fuzz_target" in
    *-apple-darwin) export CFLAGS="${CFLAGS:-} -mllvm -asan-guard-against-version-mismatch=0" ;;
  esac
  step fuzz cargo +nightly fuzz build --dev --target "$fuzz_target"
}

job_publish() {
  step publish-clean cargo clean -p sigil-ui -p sigil-fonts -p sigil-kittest -p sigil-sign
  step publish cargo publish --workspace --locked --dry-run
}

need node "https://nodejs.org"
step env refuse_env
for job in "${picked[@]}"; do
  "job_$job"
done

echo "gate: passed ${picked[*]} ($((SECONDS - started))s)" >&2
