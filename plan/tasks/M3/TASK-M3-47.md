# TASK-M3-47 — Li & Liao's known orbits against Standish 1970 and Yasko & Orlov 2015 (LL-3)

- **Milestone:** M3
- **Closes:** REQ-VAL-223, REQ-VAL-224
- **Depends on:** TASK-M3-46
- **Needs (earlier milestones):** none
- **Reviewers:** code, qa, physics
- **Pitfalls:** PIT-3
- **Size:** ~200 lines

## Goal
The replication suite's targets LL-3 run as engine tests (`principia_dd_replication.md`; R-594, R-629). Li & Liao's F1 at masses (0.6, 0.8, 1) must be shown to be Standish's 1970 orbit, and F1 and F2 at (1, 1, 1) Yasko & Orlov's 2015 Orbits 16 and 17, by integrating each from both sources' initial conditions and comparing the orbits after the remap (rule 11); F1 and F2 at (1, 1, 1) must also be shown to be one orbit (§ 4.1) (LL-3). Each tolerance is a calibration, proposed here with its evidence and confirmed by the human at the M3 gate (R-71).

## References
- `docs/design/principia_dd_replication.md` § "5.3 Free-fall periodic orbits: Li & Liao 2019"
- `docs/design/principia_dd_replication.md` § "4.1 The atlas's 311 free-fall orbits against Li & Liao's 234"
- `docs/design/principia_dd_replication.md` § "8.1 Carried over from the 25 Oct instruction"
- `docs/design/principia_dd_replication.md` § "8.2 Learnt since"
- `docs/design/principia_dd_replication.md` § "8.3 How the tasks apply them"
- `decisions.md` § "R-594 — The replication suite is adopted: published three-body results reproduced as engine tests in tiers A–D, ported as a design doc that extends the validation-orbits doc; each target a VAL requirement with its tolerance a calibration; every published table transcribed twice and never tuned to; sequenced by need; the questions it lists queued *(changes no ruling or requirement in force; recorded with R-629's port)*"
- `decisions.md` § "R-629 — PLAN.md is adopted as the single plan for verification and replication, superseding as the working plan the 25 Oct instruction's tier tables, sequencing and fetch list; its seven draft requirements are ported as M3 requirements with their tolerances as calibrations; R-594's port follows it, with the orbit catalogue's six tests as tasks; the plan's other questions stay queued *(refines R-594; answers PLAN.md § 9 questions 1 and 2)*"
- `decisions.md` § "R-71 — A missing value becomes a calibration requirement *(closes RQ-46 to RQ-55, values)*"
- `docs/notes/principia_validation_ground_truth_note.md` § "Four tiers of ground truth (increasing in what they prove)"
- `docs/notes/principia_validation_ground_truth_note.md` § "Chaos forbids pointwise comparison (exact vs structural)"

## Deliverables
- The two papers' initial conditions transcribed twice once they are in the repo (RQ-586).
- `crates/validation/src/gates/free_fall_known.rs` and the calibration proposal.

## Acceptance tests
- `cargo xtask gate replication --target LL-3` — both sources' ICs, remapped by a tested function, integrated one period under the default occupant; agreement of the shape and of T·|E|^{3/2} within REQ-VAL-224's tolerance; control: F1 at (0.6, 0.8, 1) against Yasko & Orlov's Orbit 16 must differ (REQ-VAL-223).
- `cargo xtask gate replication --target LL-3 --propose` — the proposal measures what sets the tolerance and states it, with its evidence; reviewer-checked, confirmed by the human at the M3 gate and recorded in `decisions.md` (REQ-VAL-224).
- `git diff --no-index` of each fixture table's two transcriptions — empty, or each difference settled against the page and recorded beside the fixture, before the fixture is committed (R-594 item 4).

## Notes
- Calibrations proposed here (R-71; human confirmation at the M3 gate, then recorded in decisions.md): REQ-VAL-224.
- Waits on RQ-586: the task starts only once it is ruled, and builds no chart or diagnostic ahead of the ruling (R-594 item 5).
- R-594 item 4: every published table this task commits as a fixture is transcribed twice, independently, and the two copies are diffed before the commit, each row recording its page; both copies and the diff stay beside the fixture. A table a record has already transcribed twice and diffed is copied with its sha256 and the record's diff cited. Never tune to match: a disagreement with the paper goes to `REVIEW_QUEUE.md` with both numbers, as RQ-570 did, and the fixture is not edited.
- Reference data are read from the records named in each requirement's verify; the records stay records (R-159), and the contract is `principia_dd_replication.md`.
