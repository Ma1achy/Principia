# TASK-M0-33 — Test-suite speed: nextest, stable fixtures, injectable spawn timings

- **Milestone:** M0
- **Closes:** REQ-VAL-165
- **Depends on:** TASK-M0-22
- **Needs (earlier milestones):** none
- **Reviewers:** code, qa
- **Pitfalls:** PIT-3
- **Size:** per R-223/R-225, est. ~250 counted lines

## Goal
The suite runs much faster without losing a test (R-231). `cargo test` runs test binaries one after another, so each binary's slowest test holds the rest back. Synthetic fixture workspaces are rebuilt from scratch every run because their sources are rewritten. The spawn helper's tests wait out real grace and timeout periods. The 27 Sep survey measured these as the largest costs after R-226's double control run.

## References
- `docs/read_first/principia_00_philosophy.md` § "4.4 A test that cannot fail is not a test"
- `decisions.md` § "R-214 — Children are spawned through one helper with a timeout *(closes RQ-147)*"
- `decisions.md` § "R-217 — TASK-M0-26's size accepted; a timed-out child's whole process group dies; the timeout is 300 s provisional *(amends R-214)*"
- `decisions.md` § "R-224 — The two flaky tests are fixed before TASK-M0-29"
- `decisions.md` § "R-231 — After TASK-M0-22, one task speeds up the suite: nextest, stable fixtures, injectable spawn timings"
- `decisions.md` § "R-270 — TASK-M0-33: qa's one-round exception is granted; the fixture-pool cost is sent back *(amends R-231)*"

## Deliverables
- CI's test steps (`.github/workflows/`) and the documented local run use `cargo nextest run` (pinned version), with doctests through `cargo test --doc`, for both feature sets the jobs run today. `cargo xtask controls` keeps its own cargo invocations.
- Fixture workspaces (`xtask/tests/deps.rs`, `qa_TASK-M0-01_r191/_r193/_r194` and their support modules, and the others the survey lists) are written only when their content changes, so mtimes hold, each with its own target directory kept across runs, never shared between a test and its control (R-224).
- The spawn helper (`crates/validation/src/spawn.rs`) takes an injectable grace and timeout; its own tests and qa's `qa_TASK-M0-26_r217` use short ones. The calibrated values (REQ-VAL-156) are unchanged.
- Edits to qa's merged files are limited to what these need, under a one-round exception, with qa reviewing (R-231).

## Acceptance tests
- A check that the CI test steps together list every test `cargo test --workspace -- --list` lists, in both feature sets, plus the doctests (REQ-VAL-165).
- The whole suite's wall time before and after, on the same machine, shown in the PR (REQ-VAL-165).
- The `ci` job's wall time is no slower than before (~10.5 min), and the local fixture pool is ~5 GB, both shown in the PR (REQ-VAL-165, R-270).
- Review checklist (code and qa): a warm second run of each fixture test rebuilds nothing for its fixture type; no spawn-helper test waits the calibrated values; every control still trips its test (`cargo xtask controls`) (REQ-VAL-165).

## Notes
- R-231: the human's list of 28 Sep. After TASK-M0-22, since both change how the suite and its controls run.
- The survey (27 Sep) measured about 250 s of fixture rebuilds and about 55 s of spawn-timing waits, serial, under load.
- R-270 (amends R-231): fixture copies share one build directory per fixture type, not per copy, and CI caches the pool between runs; targets are the `ci` job no slower than ~10.5 min and a local pool of ~5 GB. The "its own target directory" wording above is read per fixture type. qa gets a one-round exception to modify `crates/validation/tests/qa_TASK-M0-33.rs`.
