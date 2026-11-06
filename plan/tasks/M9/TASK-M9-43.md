# TASK-M9-43 — Read Paper I's normalisation for Tanikawa 2000's time columns (R-702)

- **Milestone:** M9
- **Closes:** REQ-VAL-351
- **Depends on:** none
- **Needs (earlier milestones):** none
- **Reviewers:** code, qa, physics
- **Pitfalls:** none
- **Size:** ~40 lines of record

## Goal
R-702 takes RQ-640's option (a): Paper I (Tanikawa, Umehara & Abe 1995) is in the library, so its normalisation is read, and whether Tanikawa 2000 keeps it. A reading task, with no code.

## References
- `decisions.md` § "R-702 — The library's open readings: Paper I's normalisation is read for Tanikawa 2000's times; Broucke 2006 stays context; Martynova & Orlov's fractions are compared with the 56 stated; the two unverified citations drop; the trust line's marks gain a dated "chosen by analogy" line; Tanikawa's free-fall papers are named by the frame each uses, no mirror for the ones read; AN-2 is shape-only *(closes RQ-640, RQ-641, RQ-642, RQ-645, RQ-648, RQ-650 and RQ-651; refines R-574)*"
- `decisions.md` § "R-680 — Standing rule: the source page is re-read before any triage item becomes a target; the spot-check's six fixes to the library triage are PR #385's work, done after the CI fixes land"

## Deliverables
- The pages of `docs/incoming/_local/external_sources_2026-10-28/springer/Tanikawa_Umehara_Abe_1995_Collision_orbits_I.pdf` read, not the record's summary (R-680), and Tanikawa 2000's own pages for what it keeps.
- A dated note in `docs/incoming/2026-10-29_research_batches/README.md` § 5.6: the quoted units, what "time" is in each paper, and the decoded unit, or the statement that it can't be decoded; the note's quotes only, since both papers are held locally.

## Acceptance tests
- Review checklist (physics reviewer) — REQ-VAL-351's verify: each quote checked against the page.

## Notes
- Added by R-702 (4 Nov 2026, RQ-640 (a)). A definition (R-72): no target is added here. If the times decode, T2000-1's row may propose them; § 9's short times (0.39 to 0.49 against Li 2021's 6.9 to 11.3) are the check.
- A build task in `plan/tools/milestones.py`, not a target (applied per R-369).
