# TASK-M3-61 — Region-D statistics: the lifetime tail and lifetime against Lyapunov time (V-9, MT-1)

- **Milestone:** M3
- **Closes:** REQ-VAL-286, REQ-VAL-287, REQ-VAL-288, REQ-VAL-289
- **Depends on:** TASK-M3-34, TASK-M3-14
- **Needs (earlier milestones):** none
- **Reviewers:** code, qa, physics
- **Pitfalls:** PIT-2, PIT-3
- **Size:** ~250 lines

## Goal
The replication suite's targets V-9, MT-1 run as engine tests (`principia_dd_replication.md`; R-594, R-629). The t_end histogram's tail on the free-fall surface must be classed as a power law (differential index about −1.4 to −1.7) or as exponential, with the criterion stated (V-9). Lifetime against Lyapunov time on the free-fall plane must follow t_d ≈ 10 t_e^1.8, with Mikkola & Tanikawa 2007's two-segment fit (MT-1). Each tolerance is a calibration, proposed here with its evidence and confirmed by the human at the M3 gate (R-71).

## References
- `docs/design/principia_dd_replication.md` § "5.9 Region D: maps and statistics"
- `docs/design/principia_dd_replication.md` § "8.1 Carried over from the 25 Oct instruction"
- `docs/design/principia_dd_replication.md` § "8.2 Learnt since"
- `docs/design/principia_dd_replication.md` § "8.3 How the tasks apply them"
- `decisions.md` § "R-594 — The replication suite is adopted: published three-body results reproduced as engine tests in tiers A–D, ported as a design doc that extends the validation-orbits doc; each target a VAL requirement with its tolerance a calibration; every published table transcribed twice and never tuned to; sequenced by need; the questions it lists queued *(changes no ruling or requirement in force; recorded with R-629's port)*"
- `decisions.md` § "R-629 — PLAN.md is adopted as the single plan for verification and replication, superseding as the working plan the 25 Oct instruction's tier tables, sequencing and fetch list; its seven draft requirements are ported as M3 requirements with their tolerances as calibrations; R-594's port follows it, with the orbit catalogue's six tests as tasks; the plan's other questions stay queued *(refines R-594; answers PLAN.md § 9 questions 1 and 2)*"
- `decisions.md` § "R-71 — A missing value becomes a calibration requirement *(closes RQ-46 to RQ-55, values)*"
- `docs/notes/principia_validation_ground_truth_note.md` § "Four tiers of ground truth (increasing in what they prove)"
- `docs/notes/principia_validation_ground_truth_note.md` § "Chaos forbids pointwise comparison (exact vs structural)"

## Deliverables
- `crates/validation/src/gates/region_d_stats.rs`; the calibration proposals.

## Acceptance tests
- `cargo xtask gate replication --target V-9` — the fit and its criterion at REQ-VAL-287's level; systems still bound at 200 counted as censored, not dropped (rule 18); control: an exponential sample must not be classed as a power law (REQ-VAL-286).
- `cargo xtask gate replication --target V-9 --propose` — the proposal measures what sets the tolerance and states it, with its evidence; reviewer-checked, confirmed by the human at the M3 gate and recorded in `decisions.md` (REQ-VAL-287).
- `cargo xtask gate replication --target MT-1` — our lifetime and FTLE on the same starts; the two-segment fit at REQ-VAL-289's level; control: lifetimes paired with shuffled FTLEs must lose the relation (REQ-VAL-288).
- `cargo xtask gate replication --target MT-1 --propose` — the proposal measures what sets the tolerance and states it, with its evidence; reviewer-checked, confirmed by the human at the M3 gate and recorded in `decisions.md` (REQ-VAL-289).

## Notes
- Calibrations proposed here (R-71; human confirmation at the M3 gate, then recorded in decisions.md): REQ-VAL-287, REQ-VAL-289.
- Reference data are read from the records named in each requirement's verify; the records stay records (R-159), and the contract is `principia_dd_replication.md`.
