# TASK-M9-14 — Period against word length: Li & Liao's period per letter (LL-4) and Hudomal's seven sequence fits (HU-4)

- **Milestone:** M9
- **Closes:** REQ-VAL-225, REQ-VAL-226, REQ-VAL-236, REQ-VAL-237
- **Depends on:** TASK-M9-06, TASK-M9-13, TASK-M3-30
- **Needs (earlier milestones):** none
- **Reviewers:** code, qa, physics
- **Pitfalls:** PIT-3
- **Size:** ~300 lines

## Goal
The replication suite's targets LL-4, HU-4 run as engine tests (`principia_dd_replication.md`; R-594, R-629). For Li & Liao's orbits, the scale-invariant period per letter T̄* = (T/L_f)·|E|^{3/2}, L_f the length of the orbit's free-group word in Tables A6–A10, must be computed per orbit from our runs and compared with the paper's ≈ 2.363 at equal masses, 1.663, 1.191, 0.776 and 0.527 at m₃ = 0.8, 0.6, 0.4 and 0.2, and its fit 1.912 m₃ + 0.084 (LL-4). Within each of Hudomal's seven sequences, T·|E|^{3/2} against free-group word length must be computed from our runs and compared with the seven published linear fits, per orbit exactly and the fits as distributions (HU-4). Each tolerance is a calibration, proposed here with its evidence and confirmed by the human with the M9 report (R-71).

## References
- `docs/design/principia_dd_replication.md` § "5.3 Free-fall periodic orbits: Li & Liao 2019"
- `docs/design/principia_dd_replication.md` § "5.4 The Belgrade orbits"
- `docs/design/principia_dd_replication.md` § "8.1 Carried over from the 25 Oct instruction"
- `docs/design/principia_dd_replication.md` § "8.2 Learnt since"
- `docs/design/principia_dd_replication.md` § "8.3 How the tasks apply them"
- `decisions.md` § "R-594 — The replication suite is adopted: published three-body results reproduced as engine tests in tiers A–D, ported as a design doc that extends the validation-orbits doc; each target a VAL requirement with its tolerance a calibration; every published table transcribed twice and never tuned to; sequenced by need; the questions it lists queued *(changes no ruling or requirement in force; recorded with R-629's port)*"
- `decisions.md` § "R-629 — PLAN.md is adopted as the single plan for verification and replication, superseding as the working plan the 25 Oct instruction's tier tables, sequencing and fetch list; its seven draft requirements are ported as M3 requirements with their tolerances as calibrations; R-594's port follows it, with the orbit catalogue's six tests as tasks; the plan's other questions stay queued *(refines R-594; answers PLAN.md § 9 questions 1 and 2)*"
- `decisions.md` § "R-71 — A missing value becomes a calibration requirement *(closes RQ-46 to RQ-55, values)*"
- `docs/design/principia_dd_validation_orbits.md` § "1.4 Šuvakov–Dmitrašinović (13 families, 2013) and Broucke–Hénon–Hadjidemetriou"
- `docs/notes/principia_validation_ground_truth_note.md` § "Four tiers of ground truth (increasing in what they prove)"
- `docs/notes/principia_validation_ground_truth_note.md` § "Chaos forbids pointwise comparison (exact vs structural)"

## Deliverables
- `fixtures/li_liao_2019/tables_a6_a10.csv` and `fixtures/belgrade/hudomal_sequences.csv`, transcribed twice, the copies and their diff kept.
- `crates/validation/src/gates/word_length.rs`: per-orbit T̄* and T|E|^{3/2}, and the fits recomputed.
- The calibration proposals.

## Acceptance tests
- `cargo xtask gate replication --target LL-4` — Tables A6–A10 (the words) transcribed twice (rule 2); per orbit T̄* from our T and E against the tables' within REQ-VAL-226; the fit recomputed and compared as a distribution; the words compared with the word reader's where its convention allows (RQ-576 governs any word comparison); control: T̄* computed with L_f off by one must move every mass set's mean (REQ-VAL-225).
- `cargo xtask gate replication --target LL-4 --propose` — the proposal measures what sets the tolerance and states it, with its evidence; reviewer-checked, confirmed by the human with the M9 report and recorded in `decisions.md` (REQ-VAL-226).
- `cargo xtask gate replication --target HU-4` — the sequence tables transcribed twice (rule 2); per orbit T|E|^{3/2} within REQ-VAL-237; each fit's slope and intercept recomputed and compared as a distribution; words compared only under RQ-576's equivalence once ruled; control: one sequence's word lengths shuffled must break its fit (REQ-VAL-236).
- `cargo xtask gate replication --target HU-4 --propose` — the proposal measures what sets the tolerance and states it, with its evidence; reviewer-checked, confirmed by the human with the M9 report and recorded in `decisions.md` (REQ-VAL-237).
- `git diff --no-index` of each fixture table's two transcriptions — empty, or each difference settled against the page and recorded beside the fixture, before the fixture is committed (R-594 item 4).

## Notes
- Calibrations proposed here (R-71; human confirmation with the M9 report, then recorded in decisions.md): REQ-VAL-226, REQ-VAL-237.
- R-594 item 4: every published table this task commits as a fixture is transcribed twice, independently, and the two copies are diffed before the commit, each row recording its page; both copies and the diff stay beside the fixture. A table a record has already transcribed twice and diffed is copied with its sha256 and the record's diff cited. Never tune to match: a disagreement with the paper goes to `REVIEW_QUEUE.md` with both numbers, as RQ-570 did, and the fixture is not edited.
- Reference data are read from the records named in each requirement's verify; the records stay records (R-159), and the contract is `principia_dd_replication.md`.
- *30 Oct 2026 (R-649, R-650):* moved to M9, external replication, and renumbered: formerly TASK-M3-54 (milestone M3). Its targets are measurements, never gates: each is scored in M9, or early as a peek (information only, `docs/verification/peek_log.md`), and its outcome is PASS, EXPLAINED DIVERGENCE (with the measurement showing the mechanism) or UNEXPLAINED, never a failure that blocks; like-for-like is checked first, and nothing is tuned, loosened or special-cased to pass (`docs/design/principia_dd_replication.md` § "12.3 Outcomes, the like-for-like check and the forbidden moves"). The tier each target binds (§ "12.2 Tiers"; a calibration takes its target's): LL-4 (REQ-VAL-225): tier (b) per orbit, tier (a) for the fit; HU-4 (REQ-VAL-236): tier (b) per orbit, tier (a) for the fits. *Was:* "The tier each binds is § '12.2 Tiers''s: (b); its statistical part also (a)." (corrected 30 Oct 2026 to the owner's confirmation of the tier reading, R-649). Its calibrations are confirmed by the human with the M9 report (REQ-VAL-340), *was:* "at the M3 gate".
