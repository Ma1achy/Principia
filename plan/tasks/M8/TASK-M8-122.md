# TASK-M8-122 — The fixed-k chart: rigid rotation at a chosen virial coefficient over D₁₁ ∪ D₁₂, a chart and builder axes (R-847)

- **Milestone:** M8
- **Closes:** REQ-CHART-143
- **Depends on:** TASK-M4-37, TASK-M8-109, TASK-M8-102, TASK-M8-117
- **Needs (earlier milestones):** none
- **Reviewers:** code, qa, physics
- **Pitfalls:** none
- **Size:** ~350 lines

## Goal
The owner's reversal of R-669 (R-847, RQ-725 (b)): fixed-`k` becomes a chart and builder axes like any other. Each pixel's start is the rigid rotation `v = ωJr`, `ω² = 2k|U|/I`, over Tanikawa & Mikkola 2008's `D₁₁ ∪ D₁₂`. The rotation sense, which the source doesn't print, is defined here under physics review (R-72), never guessed. TS-2 scores on it in M9 (TASK-M9-28).

## References
- `decisions.md` § "R-847 — R-669 is reversed: fixed-`k` and binary-plus-single (scattering) become charts and builder axes, built when their turn comes, with Heggie & Sweatman 1991 alongside and f_col built with the scattering chart; their targets score in M9; orbital elements are axes at a labelled pair, plus overlays; MK-1 stays, labelled a planar re-run *(closes RQ-723, RQ-725 and RQ-726)*"
- `docs/design/principia_dd_replication.md` § "5.12 Targets that wait for a chart, a diagnostic or data"
- `docs/design/principia_chart_composition.md` § "3.2 The primitives of the full order"

## Deliverables
- The fixed-`k` primitive and its chart: the axis `k`, the rigid-rotation construction, `D₁₁ ∪ D₁₂` as its domain, and the rotation sense, defined first and physics-reviewed.
- The builder's offer of `k` as an axis.

## Acceptance tests
- `cargo test -p engine fixed_k_chart` — REQ-CHART-143's arms and controls.

## Notes
- Nothing here is built until TASK-M4-37 has carried the rulings into the design (its Depends on). REQ-VAL-326's pointer to the 2008 paper is RQ-731's, the record PR's.
- *8 Nov 2026 (R-873):* **Build order:** stage 3 of R-863's order, after stage 2 (TASK-M8-117), placed by R-873 (RQ-744 (A) (a) and (C)); TASK-M8-106 and TASK-M8-108, stage 4, depend on it. The note above that no stage edge is made is overtaken. *Was:* "- **Depends on:** TASK-M4-37, TASK-M8-109, TASK-M8-102"
- *8 Nov 2026 (R-891):* RQ-731 is ruled (a): REQ-VAL-326's pointer is Tanikawa & Mikkola 2008, § 3.1 and § 4.2, its virial ratio k = T/|U|, Q = 2k.
