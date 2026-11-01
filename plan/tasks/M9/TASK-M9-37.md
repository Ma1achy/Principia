# TASK-M9-37 — M9's report: every external target with its tier, outcome and like-for-like checks

- **Milestone:** M9
- **Closes:** REQ-VAL-340
- **Depends on:** TASK-M9-01, TASK-M9-02, TASK-M9-03, TASK-M9-04, TASK-M9-05, TASK-M9-06, TASK-M9-08, TASK-M9-10, TASK-M9-11, TASK-M9-12, TASK-M9-13, TASK-M9-14, TASK-M9-15, TASK-M9-16, TASK-M9-17, TASK-M9-18, TASK-M9-19, TASK-M9-20, TASK-M9-21, TASK-M9-26, TASK-M9-30, TASK-M9-31, TASK-M9-32, TASK-M9-33, TASK-M9-34, TASK-M9-36
- **Needs (earlier milestones):** none
- **Reviewers:** code, qa, physics
- **Pitfalls:** none
- **Size:** ~200 lines

## Goal
M9 exits on this report, not a gate (R-650). It lists every M9 target with its tier, its outcome (PASS, EXPLAINED DIVERGENCE or UNEXPLAINED), its like-for-like checks and, for an explained divergence, the measurement showing the mechanism. A target not yet scored, waiting on a ruling or data, is listed "not scored" with what it waits on, never as a failure. Every M9 calibration is put to the human with it, or, where its proposal has not landed (REQ-SYS-095, TASK-M9-39's), listed with what it waits on, and the owner reviews it.

## References
- `docs/design/principia_dd_replication.md` § "12. Targets are measurements: tiers, outcomes and M9 (R-649, R-650)"
- `docs/design/principia_dd_replication.md` § "12.1 What stays continuous, and what moves to M9"
- `docs/design/principia_dd_replication.md` § "12.2 Tiers"
- `docs/design/principia_dd_replication.md` § "12.3 Outcomes, the like-for-like check and the forbidden moves"
- `docs/design/principia_dd_replication.md` § "12.4 The tasks in M9"
- `docs/design/principia_dd_replication.md` § "12.5 Peeks"
- `decisions.md` § "R-649 — Replication and verification targets are measurements, not gates: failing one never blocks building; each declares the tier it binds, (a) the GPU explorer, agreeing statistically or qualitatively only, or (b) the CPU IC inspector, where exact targets bind; three outcomes, PASS, EXPLAINED DIVERGENCE (shown by a measurement) and UNEXPLAINED (open, labelled, non-blocking); no default, physics, criterion, architecture or tolerance is changed and nothing is tuned per target in pursuit of a pass; a comparison is checked like-for-like before anything is called a failure; verification deepening comes after the core build *(amends R-594, R-636, R-637 and R-639 for external targets; applies to RQ-598's targets and every target in force)*"
- `decisions.md` § "R-650 — External replication becomes its own final milestone, M9, after the core build: it scores every external target, writes up the explained divergences and tightens the GPU path, and exits on its report; the internal correctness tests and the cheap anchor smoke tests (figure-eight closure, SP-1 run to its horizon, SP-2) stay continuous and block a merge as now; source ingest and transcription go on whenever idle; early peeks score a target as information only, in a peek log *(refines R-649; amends R-594, R-629, R-630, R-640 and R-642: where and when the targets are scored)*"

## Deliverables
- `docs/verification/m9_report.md` — the report, one row per M9 target, every live M9 requirement but the three of TASK-M9-39, a build task and not scored (REQ-INT-110, REQ-SYS-094, REQ-SYS-095; R-654), the report itself (REQ-VAL-340) and TASK-M9-38's definition (REQ-VAL-341), which follows the report and has no target, from the scoring commands' outputs where they exist.
- The list of M9 calibrations, each with its proposal or, for REQ-SYS-095 if TASK-M9-39 has not landed, what it waits on, for the human.

## Acceptance tests
- Review (physics): the report covers every M9 target, every live M9 requirement but REQ-INT-110, REQ-SYS-094 and REQ-SYS-095 (TASK-M9-39, a build task, not scored; R-654), REQ-VAL-340 itself and REQ-VAL-341 (TASK-M9-38's definition, which follows the report and has no target); each row has its tier, its outcome, its like-for-like checks and, for an explained divergence, the measurement; the peek log's rows are cited, never counted as scores (REQ-VAL-340).

## Notes
- Added by R-650 (30 Oct 2026). It depends on every M9 scoring task that waits on no ruling or data. Those that wait are listed "not scored" until they run, and the report is updated when they do: TASK-M9-22 to TASK-M9-25, TASK-M9-27 to TASK-M9-29 and TASK-M9-35, on RQ-593 to RQ-597, RQ-608, RQ-612, RQ-613, RQ-615, RQ-616, RQ-618 and RQ-620; TASK-M9-07, on Standish 1970 and Yasko & Orlov 2015 being in the repo (R-630 item 9); TASK-M9-09, on TM-1's data being accepted (Tanikawa & Mikkola 2021's tables landed on main in #338, under `docs/incoming/2026-10-29_research_batches/tanikawa_mikkola_2021/`, but their unblocking of the task is not yet accepted). Each rejoins this task's dependencies once it is unblocked. *31 Oct 2026 (R-653, RQ-622 (c)):* TASK-M9-33 and TASK-M9-34 rejoin, and B-2 is scored within TASK-M9-16; *was:* "TASK-M9-33 and TASK-M9-34, on RQ-622, which holds their only targets (REQ-VAL-309, REQ-VAL-308); and, within TASK-M9-16, B-2 (REQ-VAL-246, with its calibration REQ-VAL-247), on RQ-622". Applied per R-369. *Was:* "It depends on every M9 scoring task that waits on no ruling or data. Those that wait (TASK-M9-22 to TASK-M9-25, TASK-M9-27 to TASK-M9-29 and TASK-M9-35, on RQ-593 to RQ-597, RQ-608, RQ-612, RQ-613, RQ-615, RQ-616, RQ-618 and RQ-620) are listed "not scored" until they run…", and its Depends on listed TASK-M9-07, TASK-M9-09, TASK-M9-33 and TASK-M9-34, which wait (Codex review of PR #342).
- The file name `docs/verification/m9_report.md` is this port's, beside the peek log, applied per R-369.
