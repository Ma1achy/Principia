# TASK-M0-49 — Mutants runs get a per-mutant timeout and a per-process memory cap on test processes

- **Milestone:** M0
- **Closes:** REQ-VAL-179, REQ-VAL-180, REQ-VAL-181
- **Depends on:** TASK-M0-14, TASK-M0-23
- **Needs (earlier milestones):** none
- **Reviewers:** code, qa
- **Pitfalls:** none
- **Size:** ~200 lines

## Goal
A `cargo mutants` shard has one limit today, R-302's per-shard time limit (REQ-VAL-149). A mutant whose tests hang
holds its shard until that limit cuts it off, and the shard's remaining mutants go untested; a mutant that allocates
without bound can take the runner down with it. Under R-348 every `cargo mutants` run, in CI or local, gets two caps: a
per-mutant timeout through cargo-mutants' timeout setting, so a hang is recorded as a timeout and the shard finishes,
and a per-process memory cap on the test processes (`ulimit -v` or `prlimit`), so a runaway allocation kills one test,
not the runner. CI's Linux runners enforce both caps; on macOS a local run gets the timeout only (R-352). Both values
are calibration requirements (R-71): this task proposes them with their evidence, and the human confirms them at the
M0 gate.

## References
- `decisions.md` § "R-348 — Mutants runs get a per-mutant timeout and a per-process memory cap on test processes; both values are calibrated"
- `decisions.md` § "R-352 — RQ-191's thirteen items stand; the fragment-stage lint also fails on a float compared with itself and on comparisons against finite-max stand-ins *(closes RQ-191; amends R-348 and R-351)*"
- `decisions.md` § "R-302 — Per-PR mutation runs are sharded across parallel CI jobs; the nightly full run is the backstop *(closes RQ-176; amends R-196)*"
- `decisions.md` § "R-202 — A surviving mutant fails the per-PR job unless it is a listed, justified equivalent *(closes RQ-139)*"
- `decisions.md` § "R-196 — Mutation testing joins the QA gate"
- `decisions.md` § "R-71 — A missing value becomes a calibration requirement *(closes RQ-46 to RQ-55, values)*"
- `decisions.md` § "R-182 — Escape fixtures are defined; proposed tolerances are provisional in CI *(closes T4, T5)*"
- `decisions.md` § "R-176 — Controls come before the tests that need them *(closes G3, S2)*"
- `decisions.md` § "R-272 — Throwaway `measure/` branches are allowed; the ubuntu mutants timing runs on one *(closes RQ-164)*"

## Deliverables
- The per-mutant timeout, set through cargo-mutants' timeout setting at the proposed value (REQ-VAL-180, marked
  provisional, R-182), in one place both runs read: `.cargo/mutants.toml` if the setting can live there, which the
  per-PR shards and TASK-M0-19's nightly run already share, or else one place the workflows read.
- The per-process memory cap on the test processes, set with `ulimit -v` or `prlimit` at the proposed value
  (REQ-VAL-181, marked provisional), applied to each test process of the baseline and of every mutant, so that the
  cap kills the test, not cargo-mutants or the runner step. The PR says which of the two sets it and why.
- `.github/workflows/mutants.yml`: the `mutants` shards run under both caps; each cap's comment cites R-348 and its
  requirement.
- `fixtures/mutants/caps/`: a fixture crate outside the workspace, beside `fixtures/mutants/untested/`, whose diff
  yields a mutant that makes its test hang and one that makes its test allocate without bound; `mutants-fixture` runs
  it under the caps.
- The proposal, in the PR description: each value with its evidence (REQ-VAL-180's and REQ-VAL-181's verify lines),
  taken on `ubuntu-latest` (a `measure/` branch may take it, R-272).

## Acceptance tests
- CI log on the PR head, `mutants-fixture` job — `cargo mutants` over `fixtures/mutants/caps/` under the caps
  finishes; its `outcomes.json` records the hanging mutant as a timeout and the allocating one as caught, its test
  process killed by the cap, and the run goes on to the next mutant; `cargo xtask mutants-check` lists the timeout as
  not a survivor (REQ-VAL-179, R-348, R-202). Each with its negative control (R-176): the fixture's hanging mutant
  with a timeout longer than its hang is not recorded as a timeout, and its allocating mutant under a cap larger than
  its allocation is not killed by it.
- Review checklist (code) — `mutants.yml`'s shards pass the timeout at REQ-VAL-180's value and run every test process
  under REQ-VAL-181's cap, both read from one place, marked provisional; the unmutated baseline passes under both
  (REQ-VAL-179, R-182, R-348).
- Proposal: the per-mutant timeout with its evidence: the baseline's test time in each shard of the PRs merged so
  far, the slowest caught mutant's test time, and the headroom left within the per-shard limit; provisional in CI
  until the human confirms it at the M0 gate (REQ-VAL-180, R-71).
- Proposal: the per-process memory cap with its evidence: the largest peak, in the measure the cap limits, over every
  baseline test process, lavapipe's GPU tests included, and the runner's memory; provisional in CI until the human
  confirms it at the M0 gate (REQ-VAL-181, R-71).

## Notes
- Applied per R-204, accepted by R-352 (R-348): this task waits for TASK-M0-14 (PR #96), which rewrites `mutants.yml`'s
  toolchain, cache and kernel-build steps, so the caps are written and measured on the workflow as #96 leaves it.
- The nightly full run is TASK-M0-19's, which depends on this task and applies the same two caps from the same place
  (R-348).
- `ulimit -v` caps virtual memory, which the Rust test harness's threads and lavapipe reserve far beyond what they
  use; the proposal measures that, not resident memory, if `ulimit -v` or `prlimit --as` sets the cap. A cap set in
  the step's shell also binds cargo and rustc; the deliverable applies it to the test processes, and the PR shows how.
- Reviewers as TASK-M0-23's, which wrote the mutants gate.
