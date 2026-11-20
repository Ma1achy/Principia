#!/usr/bin/env bash
# What every CI job that runs cargo builds with beyond the workflows' top-level env (the owner's CI list, 4 Nov 2026,
# item 4; `xtask/tests/ci_debug_info.rs`): every dependency without debug info, in both profiles a job's cargo runs
# build in, `dev` (`cargo build`, `cargo run`, the `cargo xtask` alias) and `test` (`cargo test`, nextest). The
# workspace's own crates keep R-374's line tables: `package."*"` never matches a workspace member, so a panic still
# names file and line and a backtrace still names every frame, with file and line for the workspace's own.
#
# A cargo config in the runner's CARGO_HOME, so it reaches every cargo the job runs, the tests' child runs on fixtures
# and on the workspace among them. Never in the repository, whose profiles govern local builds (R-233), and outside
# rust-cache's key, which hashes the CARGO_* environment (R-337). `ci.yml`'s `ci` and `xtask-ci` hash this file into
# the fixture pool's key: the pool's builds are these settings' (`cachekey`).
set -eu

home="${CARGO_HOME:-$HOME/.cargo}"
mkdir -p "$home"
printf '[profile.dev.package."*"]\ndebug = false\n[profile.test.package."*"]\ndebug = false\n' >> "$home/config.toml"
