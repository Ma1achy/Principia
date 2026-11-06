# TASK-M9-42 — GG-1: the eight neighbours of (3, 43), a peek (R-701)

- **Milestone:** M9
- **Closes:** REQ-VAL-350
- **Depends on:** none
- **Needs (earlier milestones):** none
- **Reviewers:** code, qa, physics
- **Pitfalls:** none
- **Size:** ~120 lines, a script and its output

## Goal
R-701 takes RQ-612's option (a): before any definition is ruled, run all eight neighbours of (3, 43), to tell a shifted short cell from a long-lived cell one step over. An offset and a definition difference may both be present; one does not rule out the other.

## References
- `decisions.md` § "R-701 — GG-1 and BR-1: the eight neighbours of (3, 43) are run, ours stays labelled ours meanwhile, `p_max` is 4, no ε = 10⁻⁷⁰ cost is quoted, the authors' trajectories stay local, both BR-1 readings stay reported, and the open questions go to the authors in the owner's pending emails *(closes RQ-612, RQ-613, RQ-615 and RQ-616)*"
- `docs/design/principia_dd_replication.md` § "5.10 Converged-code references"
- `docs/design/principia_dd_replication.md` § "12.5 Peeks"
- `docs/design/principia_dd_replication.md` § "12.3 Outcomes, the like-for-like check and the forbidden moves"

## Deliverables
- A driver beside `docs/incoming/2026-10-28_rq598_data/brutus_run_here/hm_units.py`, running (2, 42), (2, 43), (2, 44), (3, 42), (3, 44), (4, 42), (4, 43) and (4, 44) at ε = 10⁻⁶, `L_w` set, `p_max` = 4, with the escape radius and cap converted to HM units point by point; its output committed beside the pilot's.
- Eight rows in `docs/verification/peek_log.md`, each stating that the run is Brutus, not Principia, and labelled ours.
- No Brutus source is committed beyond what the directory already holds.

## Acceptance tests
- Review checklist (physics reviewer) — REQ-VAL-350's verify: the eight rows, the HM conversion per point, and each row's like-for-like checks (`docs/design/principia_dd_replication.md` § 12.3).

## Notes
- Added by R-701 (4 Nov 2026, RQ-612 (a)). A peek, run when idle and after the backlog (§ 12.5), so it blocks nothing and TASK-M9-37 does not wait on it.
- R-638's gate stands: the measured cost per point is reported first, and ε stays at 10⁻⁶.
- A build task in `plan/tools/milestones.py`, not a target (applied per R-369): it is a like-for-like check of GG-1, whose outcome stays GG-1's.
