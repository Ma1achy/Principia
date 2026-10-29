# TASK-M3-62 — Converged-code references: Boekholt et al. 2020's lifetime grid and Breen et al. 2020's Brutus runs (GG-1, BR-1)

- **Milestone:** M3
- **Closes:** REQ-VAL-295, REQ-VAL-296, REQ-VAL-297, REQ-VAL-298
- **Depends on:** TASK-M3-25, TASK-M3-34
- **Needs (earlier milestones):** none
- **Reviewers:** code, qa, physics
- **Pitfalls:** PIT-2, PIT-3
- **Size:** ~300 lines

## Goal
The replication suite's targets GG-1, BR-1 run as engine tests (`principia_dd_replication.md`; R-594, R-629). Boekholt, Portegies Zwart & Valtonen 2020's converged region-D lifetimes must be compared with ours, to the last close approach, per point where our run is reliable and by a KS test otherwise, over the systems that end before 200 (rule 18) (GG-1). Our trajectories must agree per trajectory with Breen et al. 2020's converged Brutus runs to t = 10, in their frame remapped by a tested function (BR-1). Each tolerance is a calibration, proposed here with its evidence and confirmed by the human at the M3 gate (R-71).

## References
- `docs/design/principia_dd_replication.md` § "5.10 Converged-code references"
- `docs/design/principia_dd_replication.md` § "8.1 Carried over from the 25 Oct instruction"
- `docs/design/principia_dd_replication.md` § "8.2 Learnt since"
- `docs/design/principia_dd_replication.md` § "8.3 How the tasks apply them"
- `decisions.md` § "R-594 — The replication suite is adopted: published three-body results reproduced as engine tests in tiers A–D, ported as a design doc that extends the validation-orbits doc; each target a VAL requirement with its tolerance a calibration; every published table transcribed twice and never tuned to; sequenced by need; the questions it lists queued *(changes no ruling or requirement in force; recorded with R-629's port)*"
- `decisions.md` § "R-629 — PLAN.md is adopted as the single plan for verification and replication, superseding as the working plan the 25 Oct instruction's tier tables, sequencing and fetch list; its seven draft requirements are ported as M3 requirements with their tolerances as calibrations; R-594's port follows it, with the orbit catalogue's six tests as tasks; the plan's other questions stay queued *(refines R-594; answers PLAN.md § 9 questions 1 and 2)*"
- `decisions.md` § "R-71 — A missing value becomes a calibration requirement *(closes RQ-46 to RQ-55, values)*"
- `decisions.md` § "R-33 — The independent convergence reference is Brutus-style *(IE-5, amended)*"
- `docs/notes/principia_validation_ground_truth_note.md` § "Four tiers of ground truth (increasing in what they prove)"
- `docs/notes/principia_validation_ground_truth_note.md` § "Chaos forbids pointwise comparison (exact vs structural)"

## Deliverables
- The two data sets, once obtained (RQ-598), committed only under stated terms (rule 17), else fetched and hash-checked.
- `crates/validation/src/gates/converged.rs`; the calibration proposals.

## Acceptance tests
- `cargo xtask gate replication --target GG-1` — per-point lifetimes as they publish them, if obtained (RQ-598); the reliability of our run read from the convergence reference (R-33); agreement within REQ-VAL-296; control: lifetimes to the criterion firing rather than to the last close approach must disagree (REQ-VAL-295).
- `cargo xtask gate replication --target GG-1 --propose` — the proposal measures what sets the tolerance and states it, with its evidence; reviewer-checked, confirmed by the human at the M3 gate and recorded in `decisions.md` (REQ-VAL-296).
- `cargo xtask gate replication --target BR-1` — the data, if obtained (RQ-598); per trajectory chord against theirs within REQ-VAL-298; control: a run with one coordinate perturbed by 1e-8 must part from theirs before t = 10 (REQ-VAL-297).
- `cargo xtask gate replication --target BR-1 --propose` — the proposal measures what sets the tolerance and states it, with its evidence; reviewer-checked, confirmed by the human at the M3 gate and recorded in `decisions.md` (REQ-VAL-298).
- `git diff --no-index` of each fixture table's two transcriptions — empty, or each difference settled against the page and recorded beside the fixture, before the fixture is committed (R-594 item 4).

## Notes
- Calibrations proposed here (R-71; human confirmation at the M3 gate, then recorded in decisions.md): REQ-VAL-296, REQ-VAL-298.
- Waits on RQ-592, RQ-598: the task starts only once they are ruled, and builds no chart or diagnostic ahead of the ruling (R-594 item 5).
- R-594 item 4: every published table this task commits as a fixture is transcribed twice, independently, and the two copies are diffed before the commit, each row recording its page; both copies and the diff stay beside the fixture. A table a record has already transcribed twice and diffed is copied with its sha256 and the record's diff cited. Never tune to match: a disagreement with the paper goes to `REVIEW_QUEUE.md` with both numbers, as RQ-570 did, and the fixture is not edited.
- Reference data are read from the records named in each requirement's verify; the records stay records (R-159), and the contract is `principia_dd_replication.md`.
- *28 Oct 2026 (R-636, closing RQ-592):* GG-1 compares only the systems that end before 200 (rule 18), the rest recorded as "not evaluated within cap", never as failures; the task now waits on RQ-598 alone.
