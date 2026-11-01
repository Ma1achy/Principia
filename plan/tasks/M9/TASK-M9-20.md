# TASK-M9-20 — Lehto et al. 2008 at a small grid: local homogeneity, reversibility and the step-halving controls (L-3, L-4, L-5)

- **Milestone:** M9
- **Closes:** REQ-VAL-281, REQ-VAL-282, REQ-VAL-283, REQ-VAL-284, REQ-VAL-285
- **Depends on:** TASK-M3-34, TASK-M3-08
- **Needs (earlier milestones):** none
- **Reviewers:** code, qa, physics
- **Pitfalls:** PIT-2, PIT-3
- **Size:** ~400 lines

## Goal
The replication suite's targets L-3, L-4, L-5 run as engine tests (`principia_dd_replication.md`; R-594, R-629). Lehto et al. 2008's local-homogeneity statistic, computed with their 8-orbit stencil on our lifetime map through the tested lens remap — 6,212 of 21,699 (28.6 %) at ε = 1.25×10⁻³, falling from 28.6 to 8.6 % over six bin sizes, p_ε ∝ ε^−0.5 — must be reproduced as a distribution at a small grid (L-3). Lehto et al.'s reversibility map ξ and its populations I and II (the Fig. 9 fits) must be reproduced on the reversible occupant, logH's TTL mode, with Lehto's ξ transcribed as theirs (L-4). Halving the step must only sharpen our lifetime map's boundaries, and an unregularised run (the `none` occupant) must reproduce its main structures, as Lehto et al. report (L-5). Each tolerance is a calibration, proposed here with its evidence and confirmed by the human with the M9 report (R-71).

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
- Lehto's lens remap and their 8-orbit stencil as tested functions; `crates/validation/src/gates/lehto.rs`.
- The calibration proposals.

## Acceptance tests
- `cargo xtask gate replication --target L-3` — their escape criterion (ratio 100 or 2000) and ours recorded (rule 9); the statistic per bin size at REQ-VAL-282's level; control: a map with its pixels shuffled must lose the homogeneity; the outcome recorded with its tier: PASS where the comparison above holds, otherwise EXPLAINED DIVERGENCE (with the measurement showing the mechanism) or UNEXPLAINED, never a merge condition (R-649); the control still has to fire (REQ-VAL-281).
- `cargo xtask gate replication --target L-3 --propose` — the proposal measures what sets the tolerance and states it, with its evidence; reviewer-checked, confirmed by the human with the M9 report and recorded in `decisions.md` (REQ-VAL-282).
- `cargo xtask gate replication --target L-4` — runs on logH-TTL selected by run_reversible (R-500); the two populations' fits at REQ-VAL-284's level; Lehto's ξ is theirs, not REQ-VAL-134's definition, which it may inform; control: the same map on a non-reversible occupant must be refused; the outcome recorded with its tier: PASS where the comparison above holds, otherwise EXPLAINED DIVERGENCE (with the measurement showing the mechanism) or UNEXPLAINED, never a merge condition (R-649); the control still has to fire (REQ-VAL-283).
- `cargo xtask gate replication --target L-4 --propose` — the proposal measures what sets the tolerance and states it, with its evidence; reviewer-checked, confirmed by the human with the M9 report and recorded in `decisions.md` (REQ-VAL-284).
- `cargo xtask gate replication --target L-5 --render` — the figure beside the paper's; the three maps rendered at a small grid and reviewed by the physics reviewer; control: a map whose structure moves under step halving must be reported as failing; the physics reviewer compares them; the outcome recorded with its tier: PASS where the comparison above holds, otherwise EXPLAINED DIVERGENCE (with the measurement showing the mechanism) or UNEXPLAINED, never a merge condition (R-649); the control still has to fire (REQ-VAL-285).

## Notes
- Calibrations proposed here (R-71; human confirmation with the M9 report, then recorded in decisions.md): REQ-VAL-282, REQ-VAL-284.
- Reference data are read from the records named in each requirement's verify; the records stay records (R-159), and the contract is `principia_dd_replication.md`.
- *30 Oct 2026 (R-649, R-650):* moved to M9, external replication, and renumbered: formerly TASK-M3-60 (milestone M3). Its targets are measurements, never gates: each is scored in M9, or early as a peek (information only, `docs/verification/peek_log.md`), and its outcome is PASS, EXPLAINED DIVERGENCE (with the measurement showing the mechanism) or UNEXPLAINED, never a failure that blocks; like-for-like is checked first, and nothing is tuned, loosened or special-cased to pass (`docs/design/principia_dd_replication.md` § "12.3 Outcomes, the like-for-like check and the forbidden moves"). The tier each target binds (§ "12.2 Tiers"; a calibration takes its target's): L-3 (REQ-VAL-281): tier (a); L-4 (REQ-VAL-283): tier (a); L-5 (REQ-VAL-285): tier (a). *Was:* "The tier each binds is § '12.2 Tiers''s: (b); (a) statistically or qualitatively." (corrected 30 Oct 2026 to the owner's confirmation of the tier reading, R-649). Its calibrations are confirmed by the human with the M9 report (REQ-VAL-340), *was:* "at the M3 gate".
