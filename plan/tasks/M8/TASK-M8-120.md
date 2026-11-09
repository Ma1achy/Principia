# TASK-M8-120 — The symmetry filter: a display option after rendering, GUI and export

- **Milestone:** M8
- **Closes:** REQ-CHART-136, REQ-CHART-138, REQ-GUI-424
- **Depends on:** TASK-M2-84, TASK-M8-103, TASK-M8-32, TASK-M8-108
- **Needs (earlier milestones):** none
- **Reviewers:** code, qa, physics, gui
- **Pitfalls:** none
- **Size:** ~400 lines

## Goal
The owner's order of 7 Nov: an optional toggle that draws as transparent every pixel whose starting state is the symmetry image of another start present in the same chart, keeping one canonical representative of each pair. A display option applied after rendering, never a change to what's computed.

## References
- `docs/design/principia_chart_composition.md` § "6.6 The symmetry rule"
- `docs/design/principia_chart_composition.md` § "10.4 The symmetry filter"
- `docs/design/principia_chart_composition.md` § "8.8 Region D must match the papers (order item A5)"
- `docs/design/principia_chart_composition.md` § "6.2 Labels: reduced capability, never a refusal"
- `docs/design/principia_chart_composition.md` § "12. Tests that can fail"
- `decisions.md` § "R-871 — Region D's chart is the plane, unmasked: the full disc rendered literally, every mirror and body swap declared, so the symmetry filter leaves exactly region D with no mask, on by default for the paper-frame presets only; its check, the filtered area 0.307 at equal masses and rest *(closes RQ-714; on RQ-738, which stays open)*"
- `decisions.md` § "R-874 — The decoder's mirror step is removed, as R-865 has it; the full count is the default statistic, and "mirror pairs counted once" an optional statistic beside it, off by default, stated wherever used *(closes RQ-739)*"

## Deliverables
- `crates/engine/src/chart_authoring/symmetry.rs`: the per-chart declaration, extensible; the partner check through signed `C` and the chart's own axis inverses, never the contract's `C` or `Enc`; under a swap, body-indexed outcome fields permuted; the CPU mask, off the sim key.
- The display pass's and the export writer's read of that mask.
- `crates/gui/src/windows/chart_builder/`: the toggle, "no duplicates in this chart", and the export dialog's toggle.
- The legend's and the sidecar's statement of the canonical choice.
- Each paper-frame preset's default on.

## Acceptance tests
- `cargo test -p engine symmetry_filter_rest_reconstructs` — the owner's test: on a rest chart declaring the mirror only, one copy, filtered + its mirror reproduces the full chart bitwise, outcome fields equal, signed fields flipped; mutations: a signed field not flipped, the canonical side off by one row (REQ-CHART-136).
- `cargo test -p engine symmetry_filter_statement` — the legend and sidecar state the choice; a paper-frame preset opens on, any other chart off; a target's score is one with the filter on or off; controls: the statement dropped, the default on for every chart (REQ-CHART-138).
- `cargo test -p gui symmetry_filter_toggle` — the toggle in both places, its no-partners state, one click to the full disc with no new run; control: the toggle left live with no partners (REQ-GUI-424).

## Notes
- **Build order (R-863):** stage 5, time-axis charts and the symmetry filter. It depends on stage 4's task, TASK-M8-108, and merges only with its tests green.
- The optional later "skip computing duplicates" export setting, off by default and labelled, is not built here, and no task owes it yet.
- The test names are this design's; the two the order asks for are the owner's.
- The mirror-step proposal (design § 6.6) isn't applied: the filter reads the signed map and doesn't assume it.
- *8 Nov 2026 (R-865):* the mirror step's removal is ruled, conditional on the bitwise check, and TASK-M2-57 builds it. The filter still reads the signed map, so nothing here depends on it.
- Waits on its rulings item: RQ-736, whose conflicts include what "bitwise" means for its fixture and which preset the filter can't take to D. Nothing here is implemented until ruled (the order's A6, "Nothing implemented until ruled"); R-917's hold holds it, no longer an edge: R-916 took TASK-M2-58 (was TASK-M4-37), the rulings applied, out of Depends on, and no M3 to M9 task starts until M2's gate passes, which waits on TASK-M2-58 as an M2 task. *Was:* "TASK-M4-37 in Depends on holds it."
- *8 Nov 2026 (R-871, R-874, R-878):* RQ-714 is ruled (R-871): region D's preset is the full disc, unmasked, the filter giving exactly D; the 0.307 check is added. RQ-739 closes (R-874): the full count stays the default, "mirror pairs counted once" an optional statistic built here, off by default (REQ-CHART-150). RQ-736's conflicts keep their readings held (R-878); RQ-738 stays open on the model's items, none of which holds this task. TASK-M2-85 (the acute Burrau charts through the filter) depends on this task. *Was:* "- **Closes:** REQ-CHART-135, REQ-CHART-136, REQ-CHART-137, REQ-CHART-138, REQ-GUI-424"
- *9 Nov 2026 (R-916):* the chart core is M2's (R-916, R-917, R-919), so this header follows plan/tasks.yaml. *Was:* "Closes: REQ-CHART-135, REQ-CHART-136, REQ-CHART-137, REQ-CHART-138, REQ-GUI-424, REQ-CHART-150"; "Depends on: TASK-M4-37, TASK-M8-112, TASK-M8-103, TASK-M8-32, TASK-M8-108".
- *9 Nov 2026 (R-916):* split: the mask, the partner check, the statistics and the 0.307 area on a synthetic payload (REQ-CHART-135, REQ-CHART-137, REQ-CHART-150, REQ-CHART-175, REQ-CHART-176) are TASK-M2-84's; this task keeps the owner's rest test on real runs (REQ-CHART-136), the statement (REQ-CHART-138) and the toggle (REQ-GUI-424). *Was (Acceptance):* the `symmetry_filter_display_only`, `symmetry_filter_fixed_velocity`, `mirror_pairs_statistic` and region-D area lines, now at TASK-M2-84.
