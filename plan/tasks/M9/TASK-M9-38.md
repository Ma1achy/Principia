# TASK-M9-38 — The GPU-path tightening defined: what agreeing better means, while staying interactive

- **Milestone:** M9
- **Closes:** REQ-VAL-341
- **Depends on:** TASK-M9-37
- **Needs (earlier milestones):** none
- **Reviewers:** code, qa, physics
- **Pitfalls:** none
- **Size:** ~100 lines

## Goal
Verification deepening, the GPU path agreeing better while staying interactive, is post-build work (R-649 item 6). Before any of it is built, it is defined (R-72): what the agreement is measured against, on which targets (read from M9's report), and what staying interactive means. Each change the definition proposes is a design change with its own ruling, justified on Principia's goals, never on a target alone (R-649 item 4).

## References
- `docs/design/principia_dd_replication.md` § "12.6 Verification deepening (post-build)"
- `docs/design/principia_dd_replication.md` § "12.2 Tiers"
- `docs/notes/principia_validation_ground_truth_note.md` § "The correctness factoring: validate on CPU, inherit on GPU"
- `decisions.md` § "R-72 — A missing definition is written by the task that needs it *(closes RQ-46 to RQ-55, definitions)*"
- `decisions.md` § "R-649 — Replication and verification targets are measurements, not gates: failing one never blocks building; each declares the tier it binds, (a) the GPU explorer, agreeing statistically or qualitatively only, or (b) the CPU IC inspector, where exact targets bind; three outcomes, PASS, EXPLAINED DIVERGENCE (shown by a measurement) and UNEXPLAINED (open, labelled, non-blocking); no default, physics, criterion, architecture or tolerance is changed and nothing is tuned per target in pursuit of a pass; a comparison is checked like-for-like before anything is called a failure; verification deepening comes after the core build *(amends R-594, R-636, R-637 and R-639 for external targets; applies to RQ-598's targets and every target in force)*"
- `decisions.md` § "R-650 — External replication becomes its own final milestone, M9, after the core build: it scores every external target, writes up the explained divergences and tightens the GPU path, and exits on its report; the internal correctness tests and the cheap anchor smoke tests (figure-eight closure, SP-1 run to its horizon, SP-2) stay continuous and block a merge as now; source ingest and transcription go on whenever idle; early peeks score a target as information only, in a peek log *(refines R-649; amends R-594, R-629, R-630, R-640 and R-642: where and when the targets are scored)*"

## Deliverables
- Doc change: `docs/design/principia_dd_replication.md` § 12.6 — the definition (REQ-VAL-341).

## Acceptance tests
- The definition is in § 12.6; each change it proposes is queued for its own ruling; physics reviewer approved (REQ-VAL-341).

## Notes
- Added by R-650 (30 Oct 2026). Definitions written here (R-72; physics reviewer approves before merge): REQ-VAL-341.
