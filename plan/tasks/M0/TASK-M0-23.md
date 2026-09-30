# TASK-M0-23 — The per-PR mutation gate: `cargo mutants --in-diff` in CI

- **Milestone:** M0
- **Closes:** REQ-VAL-148, REQ-VAL-149
- **Depends on:** TASK-M0-22
- **Needs (earlier milestones):** none
- **Reviewers:** code, qa
- **Pitfalls:** none
- **Size:** ~290 lines

## Goal
Mutation testing joins the QA gate (R-196). On every pull request CI runs `cargo mutants --in-diff` on the changed code, under a per-PR time limit, and every mutant that survives in the PR's diff is a qa finding: killed with a test, or justified as equivalent. The job fails on any survivor not in a checked-in list of equivalent mutants, each entry with a one-line justification the code and qa reviewers approve (R-202). Generated code, GPU-only (spirv-gated) paths and xtask's own harness plumbing are excluded. The time limit is a value the corpus doesn't give, so this task proposes it with its evidence (R-71). If no practical limit covers per-PR runs, that goes to REVIEW_QUEUE rather than the run being dropped (R-196).

## References
- `docs/read_first/principia_00_philosophy.md` § "4.4 A test that cannot fail is not a test"
- `decisions.md` § "R-71 — A missing value becomes a calibration requirement *(closes RQ-46 to RQ-55, values)*"
- `decisions.md` § "R-177 — Cadence *(closes G4, C6)*"
- `decisions.md` § "R-182 — Escape fixtures are defined; proposed tolerances are provisional in CI *(closes T4, T5)*"
- `decisions.md` § "R-186 — GitHub-hosted runners first; no self-hosted runner *(amends R-110, R-169, R-174)*"
- `decisions.md` § "R-196 — Mutation testing joins the QA gate"
- `decisions.md` § "R-198 — TASK-M0-04 is split into M0-04, M0-21, M0-22 and M0-23 *(closes RQ-135)*"
- `decisions.md` § "R-202 — A surviving mutant fails the per-PR job unless it is a listed, justified equivalent *(closes RQ-139)*"
- `decisions.md` § "R-272 — Throwaway `measure/` branches are allowed; the ubuntu mutants timing runs on one *(closes RQ-164)*"

## Deliverables
- `.cargo/mutants.toml` — cargo-mutants' configuration, holding the one exclusion list the per-PR job and TASK-M0-19's nightly run share. Each exclusion is commented with which of R-196's three categories it falls in: generated code (the files `cargo xtask codegen` writes, matched so that a generated file added later is excluded too), GPU-only paths (items gated on `target_arch = "spirv"`) and xtask's own harness plumbing. Applied per R-204 (RQ-162) — veto?: cargo-mutants cannot match an attribute from configuration, so a spirv-gated item is excluded by also carrying `#[cfg_attr(test, mutants::skip)]`, and `.cargo/mutants.toml` names that marker in a comment under the GPU-only category (a missing marker fails closed: the item's mutants survive and the job names them). xtask's harness plumbing is `xtask/src/main.rs` and `xtask/src/codegen.rs` only, in `exclude_globs`; xtask's checks and `ci.rs` stay mutated. An exclusion whose category the code reviewer disputes goes to REVIEW_QUEUE, not into the list.
- `.github/workflows/ci.yml` — a `mutants` job on `pull_request` events, on `ubuntu-latest` (R-186): it installs cargo-mutants, writes the PR's diff against its base (`git diff origin/<base>...HEAD`), runs `cargo mutants --in-diff <diff>` under the proposed time limit (REQ-VAL-149, marked provisional, R-182), and uploads the mutants report as an artifact. The job lists each surviving mutant in its log, and fails on any survivor not in the equivalent-mutants list (R-202).
- The checked-in equivalent-mutants list, read by the `mutants` job: each entry names one mutant and carries a one-line justification of why it is equivalent; a change to the list is approved by the code and qa reviewers, as R-197 does for lint suppressions (R-202).
- `fixtures/mutants/untested/` — a fixture crate outside the workspace, with a diff that adds an untested branch, and the test that kills its mutants.
- The time-limit proposal, in the PR description: the wall-clock time of `cargo mutants --in-diff` on `ubuntu-latest` over the diffs of the PRs merged so far (each PR, its diff size and the mutants tested), the proposed limit and the headroom it leaves. The human confirms it at the M0 gate (R-71).
- `plan/reviewers/qa.md` carries R-196's line (every surviving mutant in a PR's diff is a finding, killed with a test or listed in the checked-in equivalent-mutants list with a one-line justification the code and qa reviewers approve, R-202); it was added with the plan change applying R-198 (7f90866) and applies to every qa review from then on. This task adds the CI job that makes the check mechanical.

## Acceptance tests
- `cargo mutants --dir fixtures/mutants/untested --in-diff fixtures/mutants/untested/change.diff` — without the killing test, the untested branch's mutant is reported as surviving (missed); with it, none survives: the gate can fire (REQ-VAL-148).
- The `mutants` job's check over the fixture: with the untested branch's surviving mutant not in the equivalent-mutants list the job fails naming it; with it listed (with a justification) the job passes (R-202) (REQ-VAL-148).
- CI log on the PR head: the `mutants` job ran `cargo mutants --in-diff` on this PR's diff under the proposed limit, with its report uploaded; review checklist (code): each exclusion in `.cargo/mutants.toml` names its R-196 category; review checklist (qa): R-196's line is in the qa checklist, and every mutant surviving in this PR's diff is killed or listed as equivalent; review checklist (code and qa): each entry in the equivalent-mutants list carries a one-line justification, and both approve it (R-202) (REQ-VAL-148).
- Proposal: the per-PR time limit with its evidence (measured in-diff run times on `ubuntu-latest`, and the headroom); provisional in CI until the human confirms it at the M0 gate (REQ-VAL-149).

## Notes
- R-202 (closes RQ-139): the per-PR job goes red on any surviving mutant not in the checked-in equivalent-mutants list; each entry carries a one-line justification, and the code and qa reviewers approve it, as R-197 does for lint suppressions.
- R-196's nightly full run is TASK-M0-19's (R-198), since that task creates `nightly.yml`; it uses this task's exclusion list.
- The per-PR job runs on `pull_request` events, since `--in-diff` needs a base to diff against; the per-push `ci` job is unchanged (R-177).
- This task's tests register their controls here (TASK-M0-22 is merged before it).
- R-272 (closes RQ-164): the ubuntu timings are taken on a throwaway `measure/<what>` branch, pushed so that CI runs the measurement and deleted straight after, with no PR opened from it.
