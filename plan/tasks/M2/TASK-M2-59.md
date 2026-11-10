# TASK-M2-59 — The chart composition in the contracts: registry, refusals, lowerings, region D

- **Milestone:** M2
- **Closes:** REQ-CHART-094, REQ-CHART-095, REQ-CHART-108, REQ-CHART-109, REQ-CHART-110
- **Depends on:** TASK-M2-58, TASK-M2-37, TASK-M2-52
- **Needs (earlier milestones):** none
- **Reviewers:** code, qa, physics
- **Pitfalls:** none
- **Size:** ~450 lines of docs

## Goal
Port principia_chart_composition.md into the contracts, as the rulings on RQ-712 to RQ-716 and RQ-719 give it, so the build tasks cite contracts and not a design note. Region D's chart gets its section in the chart reference (R-72).

## References
- `docs/design/principia_chart_composition.md` § "2. The two layers"
- `docs/design/principia_chart_composition.md` § "3. The primitive registry"
- `docs/design/principia_chart_composition.md` § "6.4 Measure: a statistic says what it counts"
- `docs/design/principia_chart_composition.md` § "7. Lowering: three ways a pair runs"
- `docs/design/principia_chart_composition.md` § "7.4 Seams: jitter, spread and refinement"
- `docs/design/principia_chart_composition.md` § "8.1 The definition"
- `docs/design/principia_chart_composition.md` § "8.2 The closed form"
- `docs/design/principia_chart_composition.md` § "8.4 What the chart is: the domain"
- `docs/design/principia_chart_composition.md` § "8.6 The momentum, and the rescale per pixel"
- `docs/contracts/principia_chart_decoder_contract.md` § "The four axis kinds — a closed set"
- `docs/contracts/principia_lowering_contract.md` § "Appendix — worked enumeration of the current chart set"
- `docs/design/principia_dd_generation_root.md` § "3.7 `QuadReduction` — completed ledger"

## Deliverables
- `docs/contracts/principia_chart_decoder_contract.md`: the registry beside the kinds; Part 5's refusals and labels.
- `docs/design/principia_dd_decoder.md` §3.7 and §2 and `docs/contracts/principia_inverse_encode_contract.md` Part 6: § 5.3's definitions, as RQ-715 rules.
- `docs/design/principia_chart_reference.md`: the third-body pair, the primitives' formulas, the warps and pair maps; § 5.4's rows.
- `docs/contracts/principia_lowering_contract.md`: the Anosova row conformed; CPU batch; the cap.
- `docs/design/principia_core_design.md`: seam 3's exception, as RQ-719 rules.
- `docs/gui/principia_render_gui_spec.md` § G7 and `docs/design/principia_lines.md` § 4.1, as ruled.
- `docs/design/principia_dd_refinement_policy.md` §§ 1 and 6, and `docs/design/principia_dd_generation_root.md`: § 7.4's seam clause on `unresolved(f)`, the decision `Seam`, `n_seam`, and § 3.7's "every footprint resolved" reading a flagged footprint as not resolved; the chart contract: § 6.4's statistic rule.

## Acceptance tests
- Physics reviewer: dd_decoder §3.7 and §2 carry both-held's computed form and its outcomes in order, and inverse_encode_contract Part 6 the held decode (REQ-CHART-108, REQ-CHART-109, REQ-CHART-110).
- Physics reviewer: each section written and approved, each ruling carried as ruled (REQ-CHART-094).
- Physics reviewer: the refinement policy and generation root carry § 7.4's clause, `Seam` and `n_seam`, generation root § 3.7's "every footprint resolved" reads a flagged footprint as not resolved, and the chart contract § 6.4's rule (REQ-CHART-094).
- Physics reviewer: region D's section, each formula checked against `principia_dd_validation_orbits.md` § 1.6 and the import's rescale (REQ-CHART-095).
- `python3 plan/check_plan.py` — green.

## Notes
- **Build order (R-863):** no stage of the owner's order names it, so no stage edge is made; where it goes is RQ-744 (C).
- The virial kind's answer (RQ-717) lands in this task if it is ruled by then, else in TASK-M2-70.
- Waits on its rulings item: RQ-712, RQ-713, RQ-714, RQ-715, RQ-716, RQ-719. Nothing here is implemented until ruled (the order's A6, "Nothing implemented until ruled"); TASK-M2-58 in Depends on holds it.
- *8 Nov 2026 (R-873):* **Build order:** before stage 3 (RQ-744 (C)); TASK-M2-60, which depends on it, is before stage 1.
- *9 Nov 2026 (R-916):* moved from M4 to M2 and renumbered: formerly TASK-M4-32; the header follows plan/tasks.yaml. *Was:* "Milestone: M4".
