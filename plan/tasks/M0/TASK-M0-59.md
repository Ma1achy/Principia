# TASK-M0-59 — The per-merge verdict covers every job, any red job a red `main`, an install-step outage "no result" (R-982, R-979)

- **Milestone:** M0
- **Closes:** REQ-TOOL-228
- **Depends on:** TASK-M0-54
- **Needs (earlier milestones):** REQ-TOOL-223
- **Reviewers:** code, qa
- **Pitfalls:** none
- **Size:** ~200 lines, the tests included

## Goal
The owner vetoed D-2 of the report of 12 Nov 2026: "Any red job on main is red; make the per-merge verdict cover every
job so the two can't disagree" (R-982). And they ruled RQ-789 (a): only the install step's network and archive-download
failures count as infrastructure, their verdict "no result", never green (R-979). The per-merge verdict
(`.github/scripts/ci_per_merge.py`) reads every job of `ci.yml`'s per-merge run: any failed job makes it red, but for a
failure confined to an install step's network or archive fetch, which reads "no result" in any job; that failure beside
any other reads red.

## References
- `decisions.md` § "R-982 — Any red job on `main` is a red `main`; the per-merge verdict covers every job of the per-merge run, so the verdict and the run's jobs cannot disagree *(vetoes D-2 of the report of 12 Nov 2026; amends R-907 and R-908; refines R-681)*"
- `decisions.md` § "R-979 — RQ-789: option (a), REQ-TOOL-223 amended: only the release-build install step's network and archive-download failures count as infrastructure, and their verdict is "no result", never green; any other release-build failure stays red *(closes RQ-789)*"
- `decisions.md` § "R-681 — Standing rules: while `main` is red, or a required check is stuck, agents open and update no other pull request, and the owner is told in one message; a CI-tooling pull request shows its own `xtask-ci` passing before it merges; a stuck check is cleared by re-running only the job that reports it, never the whole pipeline *(refines R-369)*"
- `decisions.md` § "R-907 — CI restructure: pull-request CI is a sanity check (build, fmt, Clippy, the normal suite, a lavapipe GPU smoke set); process tests run on a pull request only when it changes `.github/`, `xtask/`, `scripts/` or plan files, and nightly on `main`; each merge to `main` is tested on its own, advisory, a correctness failure making `main` red and any other filing a top-priority task; a 36-hour no-run alert, with lavapipe's full GPU suite as the fallback; no lavapipe-versus-RTX week *(amends R-305, R-376, R-678, R-681, R-684, R-904 and R-905)*"
- `decisions.md` § "R-908 — CI: decide and act: pull-request feedback in under 30 minutes and merge-ready to merge in under an hour; the required checks become build, fmt, Clippy, the product test suite, a lavapipe GPU smoke set and reviews; process tests move off the pull-request path, or are retired where history shows high cost and no product catch, each in its own commit; mutation, in-depth GPU tests and the slow tier leave the pull-request path; product-correctness tests are never retired *(amends R-196, R-202, R-302, R-305, R-376, R-678, R-683, R-684, R-904, R-905 and R-907)*"
- `decisions.md` § "R-968 — A job whose only failure is the package-install step is re-run once, automatically, on GitHub-hosted runners, marked as a re-run and never turning a real failure green; the per-merge verdict is unchanged, RQ-789 open *(extends R-714)*"

## Deliverables
- `.github/scripts/ci_per_merge.py`: the verdict over every job of the run, `red|no-result|green`, the process,
  lint, `cargo xtask ci` and web-skeleton jobs no longer read as "task"; the install-step class narrow, its log
  evidence (the step's name and a network or archive-download error) stated in the summary; its docstring citing R-982
  and R-979 in place of "RELEASE_JOBS fail as correctness, whatever failed".
- `.github/workflows/ci.yml`: the `per-merge` job's conclusion follows the verdict: red fails, "no result" fails with
  its own summary line, never passes, green passes.
- `xtask/tests/ci_per_merge_verdict.rs`, with its controls (R-176). Any qa file that models the old verdict changes in
  this task's qa commit (R-290), each change named on the pull request.
- `plan/OPERATIONS.md` § "Merging": the paragraph on what is red `main` names the built verdict.

## Acceptance tests
- `cargo test -p xtask --test ci_per_merge_verdict` — REQ-TOOL-228's verify: a failed process, lint, `cargo xtask ci`
  or web-skeleton job reads red; an install step's network or archive-download failure reads "no result" in any job;
  beside any other failure it reads red; all passing reads green (REQ-TOOL-228).
- `cargo test -p xtask --features controls --test ci_per_merge_verdict` — the negative controls (R-176).
- The verdict run on the saved logs of the three per-merge runs of 11 Nov 2026 (RQ-789's evidence) and of `main` at
  12 Nov 2026 04:00 (D-2's run), each result shown on the pull request.

## Notes
- New (12 Nov 2026), from R-982 and R-979, one task for both (applied per R-369: task scoping), since both change the
  one verdict script. This changes CI's behaviour, so the rulings pull request that records them changes no workflow.
- Until this merges, the orchestrator reads every job of `main`'s run itself and holds as R-681 says on any red one.
- "No result" holds as red does under R-681 (R-979's reading, flagged for the owner's confirmation); the merge watcher
  refuses a merge on it as on red (R-966).
- Applied per R-369 (R-982): the run is `ci.yml`'s on `main`; the Windows runner's named measurements in their own
  workflows are not jobs of it (R-711, R-859).
