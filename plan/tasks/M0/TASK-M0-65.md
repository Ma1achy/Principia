# TASK-M0-65 — Paths on Windows in the tests and the lints: escaped in TOML and JSON, no verbatim prefix, `/` in the lints' output and either separator in comparisons

- **Milestone:** M0
- **Closes:** REQ-TOOL-237
- **Depends on:** none
- **Needs (earlier milestones):** none
- **Reviewers:** code, qa
- **Pitfalls:** none
- **Size:** ~250 lines, the helpers' tests and controls included

## Goal
The owner's goal of 12 Nov 2026 for the Windows runner: `win-tests` green. Win-gpu run 43431404746 (ab1dc06c5) on
the PC failed about 75 tests for one cause, paths on Windows, in three groups:

- **B1:** a Windows path spliced into a TOML or JSON string, where its `\` is read as an escape (`missing escaped
  value`, `invalid escape`): the controls fixtures' `validation` path, `expected_message.rs`, `validated_chart.rs`
  and `deps.rs`'s metadata.
- **B3:** `canonicalize` gives a verbatim path (`\\?\E:\…`) on Windows, which neither cargo's output nor a `/` joined
  after it matches.
- **B2:** a path compared with `/` where Windows writes `\`: the four xtask lints print a finding's path with the
  platform's separator, and tests compare a tool's output (rustc's, the render registry's) with a `/` path.

Each test is fixed where it writes or compares the path. The lints print a repo-relative path with `/` on every
platform. Nothing a lint refuses, no assertion's meaning, no tolerance and no product result changes.

## References
- `decisions.md` § "R-711 — One measurement-only self-hosted runner, `malachys-pc-rtx2080s`, on the owner's PC, with no pull-request trigger on any self-hosted job; DX12 may run on it as a measurement; PC benchmarks stay apart from the Mac's performance gates; a recorded run on it can serve Paper 2's non-Metal parity evidence; its jobs have their own workflow; the signing rules *(closes RQ-695; amends R-174, R-186 and R-656)*"
- `decisions.md` § "R-860 — R-859's list of what the Windows runner may run is not exhaustive: the nightly `win-tests` job and the Windows arms of the path-cover and time-column benchmarks stay as named measurements, never required checks *(closes RQ-742; refines R-859)*"
- `decisions.md` § "R-946 — "Windows once proven" counts the PC's CPU-only tiers alone: the slow tier, the mutation runs and the process tests may move to the PC's per-merge run without waiting for the card, containment first and unchanged, and only once the CPU tiers repeat, three runs on one commit of `main` with the same result for every test and every hosted-Linux pass passing on all three; the PC's GPU results stay measurements until each of the four failing steps is explained, sorted by running it under lavapipe as well as on the RTX *(no RQ; refines R-909; amends R-719)*"
- `decisions.md` § "R-176 — Controls come before the tests that need them *(closes G3, S2)*"
- `decisions.md` § "R-290 — qa may change test files that only qa has committed to *(closes RQ-172, amends R-237)*"

## Deliverables
- `crates/validation/src/paths.rs`, a module of the validation crate: `plain`, a path without Windows' verbatim prefix
  (`\\?\C:\a` as `C:\a`, `\\?\UNC\s\h` as `\\s\h`, any other path unchanged); `escaped`, a path as `plain` gives it,
  escaped for the inside of a TOML basic string or a JSON string; `slashed`, a tool's output with each `\` read as `/`,
  for comparing it with a `/` path.
- `xtask/src/lib.rs`: `repo_path`, a path with its separator written `/`, and the four lints' `Finding` display
  (`lint_no_alloc`, `lint_constants`, `lint_vocab`, `lint_compute`) printing the path through it.
- B1: `xtask/tests/controls.rs`, `crates/validation/tests/expected_message.rs`, `crates/engine/tests/validated_chart.rs`
  write the `validation` or `crates` path through `escaped`; `xtask/tests/deps.rs`'s metadata writes the workspace
  path through it.
- B3: `xtask/tests/build_kernel.rs`'s expected working directory through `plain`.
- B2: `crates/engine/tests/validated_chart.rs`, `crates/render/tests/registry.rs`, `xtask/tests/deps.rs` compare the
  tool's output through `slashed`; `crates/kernel/tests/links.rs` compares by `Path::ends_with`;
  `xtask/tests/lint_constants.rs` and `xtask/tests/ci_changes.rs` write the paths they compare through `repo_path`.
- No file named `qa_*` or `support/qa_*` edited (R-290): each such site the PR lists with its file, line and fix, for
  qa's round. No `cfg(windows)` or `cfg(unix)` added.

## Acceptance tests
- `cargo test -p validation --test paths` and `cargo test -p xtask --test paths` — the helpers on Windows' forms, fed
  as text on the Mac; controls: the prefix kept, a `\` unescaped, and a separator left as `\` each fail
  (REQ-TOOL-237).
- On the Mac, with and without `--features controls`: `cargo test -p xtask --test controls --test deps --test
  lint_no_alloc --test lint_constants --test lint_vocab --test lint_compute_pipelines --test build_kernel --test
  ci_changes --test paths`, `cargo test -p validation --test expected_message --test paths`, `cargo test -p engine
  --test validated_chart`, `cargo test -p kernel --test links`, `cargo test -p render --test registry`; `cargo clippy
  --workspace --all-targets -- -D warnings` with and without `--features controls`; `cargo fmt --check`
  (REQ-TOOL-237).
- On the PC, the tests of groups B1, B2 and B3 in run 43431404746 pass on the first `win-tests` run on `main` after
  merge, except the qa files' sites and the tests a group-A fix also holds back; the run id reported in the PR or a
  follow-up comment, a measurement (R-711) (REQ-TOOL-237).

## Notes
- Entered 13 Nov 2026 (applied per R-369: a test-only and tooling fix that changes no result, two-way).
- Out of scope: groups A1 to A4 (Unix tools, shell stand-ins, `sysctl`), P1 to P6 and E of the triage of run
  43431404746, and any platform gating, which waits on the owner.
- REQ-TOOL-237: REQ-TOOL-234 is TASK-M0-63's, and TASK-M0-64, open at the same time, renumbers its own to the next
  free id, likely 235; 237 leaves it room.
