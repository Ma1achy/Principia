#!/usr/bin/env bash
# Prints which GPU shards this run needs: `run`, whether any; `kernel`, whether the kernel's build reads what changed;
# and `shards`, gpu-shard's matrix entries as JSON. Without BASE_REF (push, schedule, dispatch) it's every shard. On a
# pull request it's every shard when its diff against origin/$BASE_REF touches one of `paths`, git pathspecs, else the
# render controls alone when it touches crates/render, else none.
# `paths` covers every entry of kernel_cache::INPUTS (xtask/tests/ci_gpu_kernel.rs).
set -euo pipefail

paths=(
  .cargo/config.toml
  Cargo.lock
  Cargo.toml
  crates/kernel
  crates/ledger/Cargo.toml
  crates/ledger/build.rs
  crates/ledger/data
  crates/ledger/src
  crates/ledger/wgsl
  crates/render/wgsl
  rust-toolchain.toml
  xtask/rust-gpu-backend.lock
  'xtask/src/build_kernel*'
  xtask/src/kernel_cache.rs
  .github/scripts/gpu_touch.sh
  .github/workflows/ci.yml
)

render='{"name": "controls render", "controls": "render"}'
shards=(
  '{"name": "suites 1/2, controls kernel", "slice": 1, "controls": "kernel", "save": true}'
  '{"name": "suites 2/2, golden, controls ledger, prin", "slice": 2, "controls": "ledger,prin", "golden": true}'
  '{"name": "controls xtask", "controls": "xtask"}'
  '{"name": "controls validation", "controls": "validation"}'
  '{"name": "controls engine", "controls": "engine"}'
  "$render"
)

# answer <kernel> [<shard>...]
answer() {
  local kernel=$1 run=true IFS=,
  shift
  if [ $# -eq 0 ]; then run=false; fi
  echo "run=$run"
  echo "kernel=$kernel"
  echo "shards=[$*]"
}

if [ -z "${BASE_REF:-}" ]; then
  answer true "${shards[@]}"
  exit
fi
# A rename counts as its old path too.
changed() {
  git diff --name-only --no-renames "origin/$BASE_REF...HEAD" -- "$@"
}
# Taken before they are tested, so a failing diff fails the job.
kernel_changed=$(changed "${paths[@]}")
render_changed=$(changed crates/render)
if [ -n "$kernel_changed" ]; then
  answer true "${shards[@]}"
elif [ -z "$render_changed" ]; then
  answer false
elif [ "$BASE_REF" = main ]; then
  answer false "$render"
else
  # Into another branch, whose own kernel changes this diff leaves out: changing the base to main starts no new run.
  answer false "${shards[@]}"
fi
