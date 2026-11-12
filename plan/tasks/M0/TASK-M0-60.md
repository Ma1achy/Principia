# TASK-M0-60 — A `plan/requirements.yaml`-only change runs on the pull request only the tests that read it (R-980)

- **Milestone:** M0
- **Closes:** REQ-TOOL-229
- **Depends on:** TASK-M0-54
- **Needs (earlier milestones):** none
- **Reviewers:** code, qa
- **Pitfalls:** none
- **Size:** ~150 lines, the tests included

## Goal
The owner ruled RQ-790 (a) (R-980): a pull request whose only code-tier path is `plan/requirements.yaml` runs on the
pull request only the tests that read that file, the tooling's and the product crates' alike, and the per-merge run
still runs the rest on every merge. Which of the sixteen files under `crates/` that name the file actually read it is
established here, by a scan, and every one that reads it stays on the pull request.

## References
- `decisions.md` § "R-980 — RQ-790: option (a), a `plan/requirements.yaml`-only change runs on the pull request only the tests that read that file, the tooling's and the product crates' alike; the per-merge run still runs the rest on every merge *(closes RQ-790; refines R-677)*"
- `decisions.md` § "R-677 — CI runs in three tiers: a change to Principia's code runs full CI as before; one to the plan's task data or the GUI's artboards and references runs only the cheap checks and the tests that read what it changed, with no build, test shard, GPU suite or mutants; one to the docs runs only the cheap checks; a change takes its highest tier, and an unknown path runs full CI; mutants run only when `crates/` source changes, on the merge-ready head; every required check still reports under its own name *(amends R-656, R-662, R-110, R-177, R-235, R-305 and R-676)*"
- `decisions.md` § "R-909 — The CI model is approved: a required pull-request tier under 30 minutes, an advisory tier on each merge to `main`, and a nightly and Windows background; the golden runner over every suite and the screenshot runner's self-test are product correctness, run per merge and never retired; the 17 retirement candidates are retired, one commit each; branch protection stays the owner's, who changes it from an exact old-to-new list *(amends R-677, R-683, R-684, R-904, R-905, R-907 and R-908)*"

## Deliverables
- `scripts/ci_changes.sh` and `.github/scripts/ci_quick_crates.py`: the `plan/requirements.yaml`-only class and its
  selection, the readers listed by a scan of the test sources, never by hand.
- `xtask/tests/ci_requirements_only.rs`, with its controls (R-176); `xtask/tests/ci_changes.rs` follows. Any qa file
  that models the old selection changes in this task's qa commit (R-290), each change named on the pull request.

## Acceptance tests
- `cargo test -p xtask --test ci_requirements_only` — REQ-TOOL-229's verify (REQ-TOOL-229).
- `cargo test -p xtask --features controls --test ci_requirements_only` — the negative controls (R-176).
- A pull-request run on a head that changes only `plan/requirements.yaml`, its selected tests and its time shown on
  the pull request, and the per-merge run on its merge commit passing.

## Notes
- New (12 Nov 2026), from R-980 (applied per R-369: task scoping, apart from TASK-M0-59's verdict, which changes a
  different script). This changes CI's behaviour, so the rulings pull request that records it changes no workflow.
- RQ-790's evidence names sixteen files under `crates/` that name `plan/requirements.yaml`; a file that only names it
  is not a reader, and the scan says which.
