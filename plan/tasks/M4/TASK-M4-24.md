# TASK-M4-24 — Symbol-sequence maps: Tanikawa & Mikkola's free-fall and non-zero-L maps, and Valtonen et al.'s L/M/A/H partition (TS-1, TS-2, V-1/V-3/V-7)

- **Milestone:** M4
- **Closes:** REQ-VAL-325, REQ-VAL-326, REQ-VAL-327, REQ-VAL-328
- **Depends on:** TASK-M4-17, TASK-M3-16
- **Needs (earlier milestones):** none
- **Reviewers:** code, qa, physics
- **Pitfalls:** PIT-3
- **Size:** ~350 lines

## Goal
The replication suite's targets TS-1, TS-2, V-1/V-3/V-7 run as engine tests (`principia_dd_replication.md`; R-594, R-629). Maps of the first two to four syzygy symbols over the free-fall plane must reproduce Tanikawa & Mikkola's 2008 and 2015 maps in their convention and labelling, after the D11 mirror remap (TS-1). First-symbol maps at non-zero angular momentum, virial ratio 0 to 0.7 (Tanikawa & Mikkola 2008, Fig. 7), must be reproduced, the fixed-k velocity prescription read from the 2015 PASJ paper (TS-2). The L/M/A/H partition of the Agekyan–Anosova map as Valtonen et al. 2016 draw it, and a transition-entropy map, must be reproduced, with the high-entropy share (0.66–0.83 measured from the figure, against the text's 'roughly one half') compared as a distribution (V-1, V-3, V-7). Each tolerance is a calibration, proposed here with its evidence and confirmed by the human at the M4 gate (R-71).

## References
- `docs/design/principia_dd_replication.md` § "5.12 Targets that wait for a chart, a diagnostic or data"
- `docs/design/principia_dd_replication.md` § "8.1 Carried over from the 25 Oct instruction"
- `docs/design/principia_dd_replication.md` § "8.2 Learnt since"
- `docs/design/principia_dd_replication.md` § "8.3 How the tasks apply them"
- `decisions.md` § "R-594 — The replication suite is adopted: published three-body results reproduced as engine tests in tiers A–D, ported as a design doc that extends the validation-orbits doc; each target a VAL requirement with its tolerance a calibration; every published table transcribed twice and never tuned to; sequenced by need; the questions it lists queued *(changes no ruling or requirement in force; recorded with R-629's port)*"
- `decisions.md` § "R-629 — PLAN.md is adopted as the single plan for verification and replication, superseding as the working plan the 25 Oct instruction's tier tables, sequencing and fetch list; its seven draft requirements are ported as M3 requirements with their tolerances as calibrations; R-594's port follows it, with the orbit catalogue's six tests as tasks; the plan's other questions stay queued *(refines R-594; answers PLAN.md § 9 questions 1 and 2)*"
- `decisions.md` § "R-71 — A missing value becomes a calibration requirement *(closes RQ-46 to RQ-55, values)*"
- `docs/notes/principia_validation_ground_truth_note.md` § "Four tiers of ground truth (increasing in what they prove)"
- `docs/notes/principia_validation_ground_truth_note.md` § "Chaos forbids pointwise comparison (exact vs structural)"

## Deliverables
- The maps on the field and chart RQ-595 rules; the D11 mirror remap and the book's ×2 scale as tested functions.
- The calibration proposal.

## Acceptance tests
- `cargo xtask gate replication --target TS-1 --render` — the figure beside the paper's; the syzygy-symbol field once RQ-595 is ruled; reviewed by the physics reviewer; control: the map without the mirror remap must be mirrored; the physics reviewer compares them (REQ-VAL-325).
- `cargo xtask gate replication --target TS-2 --render` — the figure beside the paper's; as TS-1, on the chart RQ-595 rules; control: the k = 0 map must equal TS-1's first-symbol map; the physics reviewer compares them (REQ-VAL-326).
- `cargo xtask gate replication --target V-7` — the symbol-sequence field and Chernin et al.'s definitions once RQ-595 is ruled; the share at REQ-VAL-328's level; control: a partition at the wrong scale must misplace the boundaries (REQ-VAL-327).
- `cargo xtask gate replication --target V-7 --propose` — the proposal measures what sets the tolerance and states it, with its evidence; reviewer-checked, confirmed by the human at the M4 gate and recorded in `decisions.md` (REQ-VAL-328).

## Notes
- Calibrations proposed here (R-71; human confirmation at the M4 gate, then recorded in decisions.md): REQ-VAL-328.
- Waits on RQ-595: the task starts only once it is ruled, and builds no chart or diagnostic ahead of the ruling (R-594 item 5).
- Reference data are read from the records named in each requirement's verify; the records stay records (R-159), and the contract is `principia_dd_replication.md`.
