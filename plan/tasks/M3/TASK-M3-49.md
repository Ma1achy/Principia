# TASK-M3-49 — Tanikawa & Mikkola 2021's triple-collision points on the free-fall plane (TM-1)

- **Milestone:** M3
- **Closes:** REQ-VAL-240, REQ-VAL-241
- **Depends on:** TASK-M3-48
- **Needs (earlier milestones):** none
- **Reviewers:** code, qa, physics
- **Pitfalls:** PIT-3
- **Size:** ~300 lines

## Goal
The replication suite's targets TM-1 run as engine tests (`principia_dd_replication.md`; R-594, R-629). On the free-fall plane, the binary-collision curves must converge on each of Tanikawa & Mikkola 2021's eleven triple-collision points (Table 2), and `d_min` must scale toward each along Table 4's approach initial conditions (radii 1e-3 to 1e-5), after the D11 mirror remap (rule 6) (TM-1). Each tolerance is a calibration, proposed here with its evidence and confirmed by the human at the M3 gate (R-71).

## References
- `docs/design/principia_dd_replication.md` § "5.5 Regularisation"
- `docs/design/principia_dd_replication.md` § "8.1 Carried over from the 25 Oct instruction"
- `docs/design/principia_dd_replication.md` § "8.2 Learnt since"
- `docs/design/principia_dd_replication.md` § "8.3 How the tasks apply them"
- `decisions.md` § "R-594 — The replication suite is adopted: published three-body results reproduced as engine tests in tiers A–D, ported as a design doc that extends the validation-orbits doc; each target a VAL requirement with its tolerance a calibration; every published table transcribed twice and never tuned to; sequenced by need; the questions it lists queued *(changes no ruling or requirement in force; recorded with R-629's port)*"
- `decisions.md` § "R-629 — PLAN.md is adopted as the single plan for verification and replication, superseding as the working plan the 25 Oct instruction's tier tables, sequencing and fetch list; its seven draft requirements are ported as M3 requirements with their tolerances as calibrations; R-594's port follows it, with the orbit catalogue's six tests as tasks; the plan's other questions stay queued *(refines R-594; answers PLAN.md § 9 questions 1 and 2)*"
- `decisions.md` § "R-71 — A missing value becomes a calibration requirement *(closes RQ-46 to RQ-55, values)*"
- `docs/notes/principia_validation_ground_truth_note.md` § "Four tiers of ground truth (increasing in what they prove)"
- `docs/notes/principia_validation_ground_truth_note.md` § "Chaos forbids pointwise comparison (exact vs structural)"

## Deliverables
- Tables 1, 2 and 4 transcribed twice once the paper is in the repo (RQ-586).
- The D11 mirror remap as a tested function; `crates/validation/src/gates/triple_collision.rs`; the calibration proposal.

## Acceptance tests
- `cargo xtask gate replication --target TM-1` — Tables 1, 2 and 4 transcribed twice once the paper is in the repo (rule 1, 2); the D11 remap a tested function; the measured d_min slope per point within REQ-VAL-241; control: an approach line through a point off the table must not converge (REQ-VAL-240).
- `cargo xtask gate replication --target TM-1 --propose` — the proposal measures what sets the tolerance and states it, with its evidence; reviewer-checked, confirmed by the human at the M3 gate and recorded in `decisions.md` (REQ-VAL-241).
- `git diff --no-index` of each fixture table's two transcriptions — empty, or each difference settled against the page and recorded beside the fixture, before the fixture is committed (R-594 item 4).

## Notes
- Calibrations proposed here (R-71; human confirmation at the M3 gate, then recorded in decisions.md): REQ-VAL-241.
- Waits on Tanikawa & Mikkola 2021 (doi:10.1007/s10569-021-10050-8): RQ-586 is closed by R-630 item 9, which has it fetched; it is paywalled, so it is listed for the human to fetch, and the task starts only once it is in the repo (R-630 item 9; R-594 item 5).
- R-594 item 4: every published table this task commits as a fixture is transcribed twice, independently, and the two copies are diffed before the commit, each row recording its page; both copies and the diff stay beside the fixture. A table a record has already transcribed twice and diffed is copied with its sha256 and the record's diff cited. Never tune to match: a disagreement with the paper goes to `REVIEW_QUEUE.md` with both numbers, as RQ-570 did, and the fixture is not edited.
- Reference data are read from the records named in each requirement's verify; the records stay records (R-159), and the contract is `principia_dd_replication.md`.
