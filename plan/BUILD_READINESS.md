# Build readiness

*Checked 26 Sep 2026 against `main` at `86263a7` (after PR #11), with read-only `gh` calls; 5a amended by R-186. The build hasn't started:
there is no code, no crates, no CI workflow and no task branch.*

| # | Item | Status | What was found |
|---|---|---|---|
| 1 | Plan merged, `check_plan.py` green | **done** | PRs #9, #10, #11 merged. `python3 plan/check_plan.py`: **246 tasks, 1228 live requirements, 0 failures** (11 retired, 1239 ids). |
| 2 | Open REVIEW_QUEUE entries | **done (none)** | 128 entries, each with its `**Ruling:**`; no requirement carries `rq:`. |
| 3 | Pending rulings | **done (none)** | `DECISIONS_TO_MAKE.md`: "every item below is ruled by R-21 to R-59". The cold-read rulings R-168 to R-184 are applied (PR #10); R-185 is applied (PR #11). |
| 4 | Crate map confirmed | **done** | R-185: systems_architecture §7.1 confirmed; kernel → ledger is a build-dependency only; the kernel is `no_std`. |
| 5a | Self-hosted runner | **not needed** | R-186: CI runs on GitHub-hosted `ubuntu-latest` and `macos-15`; benchmarks on the human's Mac. The first Metal check (TASK-M0-04, TASK-M0-06) decides whether one is ever proposed. |
| 5b | Outside contributors need approval (R-174) | **waiting** | `…/actions/permissions/fork-pr-contributor-approval` → `approval_policy: first_time_contributors`. R-174 asks for approval of **all** outside collaborators (`all_external_contributors`). See HUMAN_SETUP §1. |
| 5c | `main` protected, requiring `ci` and `reviews-complete` (R-175, R-177) | **waiting** | `…/branches/main/protection` → 404 "Branch not protected"; `…/rulesets` → `[]`. The two checks can only be required once their workflows have run (after TASK-M0-01 and TASK-M0-03). See HUMAN_SETUP §2. |
| 6 | Labels present | **done** | `design`, `investigation`, `validation`, `gui`. |
| 7 | First task ready | **done** | TASK-M0-01 ("The cargo workspace, the crates and CI"). Its only dependency, TASK-M0-00, is done (R-185, PR #11). Not started. |

**Waiting on the human:** 5b (5a is no longer needed, R-186). For 5c, protection can be turned on now, and the two required checks added after
TASK-M0-01 and TASK-M0-03 have run them. The build starts when the human says so.
