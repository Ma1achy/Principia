# TASK-M9-44 — LL-6: all 316 free-fall orbits against the authors' table, a peek target (R-704)

- **Milestone:** M9
- **Closes:** REQ-VAL-352, REQ-VAL-353
- **Depends on:** TASK-M9-06
- **Needs (earlier milestones):** none
- **Reviewers:** code, qa, physics
- **Pitfalls:** PIT-3
- **Size:** ~200 lines

## Goal
R-704 takes RQ-661's option (b). LL-5 stays closure on the atlas's rows, as ruled. LL-6 runs what LL-5 can't: the authors' own starts, for all 316 orbits, the 82 website-only ones included, against the authors' own table.

## References
- `decisions.md` § "R-704 — PLAN.md § 2.1 gains a dated note pointing at the authors' GitHub table, LL-5 staying closure only, and a new peek target, LL-6, compares all 316 orbits with that table *(closes RQ-661)*"
- `docs/design/principia_dd_replication.md` § "5.3 Free-fall periodic orbits: Li & Liao 2019"
- `docs/design/principia_dd_replication.md` § "12.5 Peeks"
- `docs/design/principia_dd_replication.md` § "12.3 Outcomes, the like-for-like check and the forbidden moves"

## Deliverables
- A parser for `docs/incoming/2026-10-27_hp_ics_freefall/data/liliao_github/free-fall-3b-pictures.md`, with a sha256 check of the file.
- The target in `crates/validation`, through the replication gate: the frame conversion a tested function, each row's closure at its T, and the table against the atlas's `pub_*`.
- The tolerance's proposal (REQ-VAL-353); the result a row of `docs/verification/peek_log.md`, stating its tier, per-orbit spread and conventions.

## Acceptance tests
- `cargo xtask gate replication --target LL-6` — REQ-VAL-352's verify, with its control; the outcome recorded with its tier: PASS, EXPLAINED DIVERGENCE or UNEXPLAINED, never a merge condition (R-649); the control still has to fire.
- `cargo xtask gate replication --target LL-6 --propose` — the proposal measures what sets the tolerance; reviewer-checked, confirmed by the human with the M9 report and recorded in `decisions.md` (REQ-VAL-353).

## Notes
- Added by R-704 (4 Nov 2026, RQ-661 (b)).
- Li & Liao's 2017 tables were found not correct to the last printed digit on 5 orbits, so a similar limit is plausible here, unmeasured: a row failing by its last digit is a finding about the table's rounding, stated with its measurement, and nothing is tuned.
- The 82 are cited to the table, never as Li & Liao 2019's published results (rule 13).
