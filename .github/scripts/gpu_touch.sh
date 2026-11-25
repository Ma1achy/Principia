#!/usr/bin/env bash
# Prints `run=true` when this run needs the GPU suites, else `run=false`: always without BASE_REF (push, schedule,
# dispatch), and on a pull request when its diff against origin/$BASE_REF touches one of `paths`, git pathspecs.
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

if [ -z "${BASE_REF:-}" ]; then
  echo run=true
  exit
fi
# Taken before it is tested, so a failing diff fails the job; a rename counts as its old path too.
touched=$(git diff --name-only --no-renames "origin/$BASE_REF...HEAD" -- "${paths[@]}")
if [ -n "$touched" ]; then echo run=true; else echo run=false; fi
