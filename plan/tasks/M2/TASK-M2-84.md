# TASK-M2-84 — The symmetry filter's maths: the mask, the partner check and the statistics, on a synthetic payload (R-916)

- **Milestone:** M2
- **Closes:** REQ-CHART-135, REQ-CHART-137, REQ-CHART-150, REQ-CHART-175, REQ-CHART-176, REQ-CHART-209
- **Depends on:** TASK-M2-77, TASK-M2-81, TASK-M2-99
- **Needs (earlier milestones):** none
- **Reviewers:** code, qa, physics
- **Pitfalls:** none
- **Size:** ~300 lines, the tests included

## Goal
The symmetry filter's maths on a synthetic payload: the per-chart declaration, the partner check through signed `C`, the mask off the sim key, the optional "mirror pairs counted once" statistic, and the owner's two checks read on a synthetic payload and on region D's area. The toggle, the export and the owner's test on real runs stay TASK-M8-120's.

## References
- `docs/design/principia_chart_composition.md` § "10.4 The symmetry filter"
- `docs/design/principia_chart_composition.md` § "6.6 The symmetry rule"
- `docs/design/principia_chart_composition.md` § "8.8 Region D must match the papers (order item A5)"
- `decisions.md` § "R-874 — The decoder's mirror step is removed, as R-865 has it; the full count is the default statistic, and "mirror pairs counted once" an optional statistic beside it, off by default, stated wherever used *(closes RQ-739)*"
- `decisions.md` § "R-916 — M2 is the whole non-visual chart system: the primitive registry, axes from primitives and warps, charts as a pair of axes with their dependence rules, refusals and labels, curved slices, CPU decode and encode through the composition, and the non-visual path maths, each an engine function with its tests, landing before any further downstream chart work, today's charts and view operations re-expressed in it bit for bit first; GPU lowering, contours and display, GUI routes, panels, the Sequence's UI, time-axis mode's UI and overlays keep their milestones as thin adapters onto it; a task mixing maths and visuals is split *(amends R-863; supersedes R-873 (A) (b) and (E) (i); refines R-864 and R-894)*"
- `docs/design/principia_chart_composition.md` § "8.1 The definition"
- `docs/design/principia_chart_composition.md` § "6.2 Labels: reduced capability, never a refusal"
- `decisions.md` § "R-871 — Region D's chart is the plane, unmasked: the full disc rendered literally, every mirror and body swap declared, so the symmetry filter leaves exactly region D with no mask, on by default for the paper-frame presets only; its check, the filtered area 0.307 at equal masses and rest *(closes RQ-714; on RQ-738, which stays open)*"

## Deliverables
- `crates/engine/src/chart_authoring/symmetry.rs`: the per-chart declaration, extensible; the partner check through signed `C` and the chart's own axis inverses; the CPU mask, off the sim key.
- The mirror-pairs statistic, off by default (R-874).

## Acceptance tests
- `cargo test -p engine symmetry_filter_display_only` — filter on and off give one sim key, payload and statistic, bit for bit; a chart's list of symmetries takes a new entry with no change to the filter; control: the filter feeding a statistic fails (REQ-CHART-135).
- `cargo test -p engine symmetry_filter_fixed_velocity` — on a shape-sphere chart with fixed velocities p_ρ = (0.1, 0.1), p_λ = (0, 0) relative to ρ̃ on +x, Π·p ≠ p asserted after signed C first, the filter blanks nothing and reads no duplicates in this chart; a partner out of the view and in the domain still blanks, and at unequal masses or eq. 3's near-rest start the swap blanks nothing; the same chart at p_ρ = (0.1, 0) does blank; mutation: the partner check removed, blanking by the map's (θ, π − φ) alone, fails (REQ-CHART-137).
- `cargo test -p engine mirror_pairs_statistic` — on a mirror-symmetric rest preset the default statistic counts every start, and with the option on the paired count is half the full count up to the pixels on the mirror line, the legend and the sidecar saying so; the option is off by default; controls: the option on by default fails, a paired count that drops a start from the computed chart fails, the statement missing from the sidecar fails (REQ-CHART-150).
- `cargo test -p engine symmetry_mask_synthetic` — a third-body rest chart declaring the mirror only, one copy, on a synthetic payload whose pairs carry mirrored fields, filtered, plus its mirror, is the full chart bitwise, outcome fields equal, signed fields flipped; mutations: a signed field not flipped fails, the canonical side off by one row fails (REQ-CHART-175).
- `cargo test -p engine symmetry_region_d_area` — the filtered area at equal masses and exact rest within the derived pixel bound of π/6 − √3/8; control: the mask with one body swap undeclared fails (REQ-CHART-176).
- *11 Nov 2026 (R-947, R-944):* Definition: chart_composition § 10.4 states the canonical representative of a mirror pair on a blend, approved by the physics reviewer; the filter's mask test on a blend of two mirror-declaring charts blanks a pair only where the partner check passes (control: a blend that breaks the mirror blanks nothing). **The condition written in: the filter runs on a blend under (D) only once physics review confirms the mechanism and approves this definition (R-72); else the filter is off mid-morph, as R-899's Q8 (a) has it** (REQ-CHART-209).
- *11 Nov 2026 (the physics review of PR #482, review 5852979675, its confirmation 2):* Definition: chart_composition § 10.4 states the canonical representative with its five points: (a) a strict total order on chart points in the frame's window coordinates (row first, then column), not a resolution-dependent pixel index, unchanged by pan, zoom and picture size; (b) "blank `u` iff an equivalent point strictly precedes it", the least member of each orbit kept, orbits of more than two members and a fold's several points covered; (c) a self-partner kept; (d) the partner relation tested symmetric at the tolerance, the control a pair with both members blanked; (e) the legend naming the rule; the physics reviewer approved it (REQ-CHART-209).

## Notes
- Split from TASK-M8-120 by R-916 (9 Nov 2026): REQ-CHART-175 is the owner's rest test on a synthetic payload (F7), REQ-CHART-176 the 0.307 area; TASK-M8-120 keeps REQ-CHART-136, REQ-CHART-138 and REQ-GUI-424.
- **Build order (R-916):** stage 5; TASK-M2-85 depends on it.
- *11 Nov 2026 (the re-plan's addition (`plan/charts/m2-chart-replan-addition.md`) § F.4; R-947):* the partner is found by `place`, `place(g·D(φ(u)), chart)` (TASK-M2-99, a new dependency). The filter on a blend is (D), applied per R-369 and conditional on physics review: the frame declares the symmetries either end declares, and the canonical representative on a blend is a definition owed (REQ-CHART-209). *Was (Depends on):* "TASK-M2-77, TASK-M2-81"; *was (Closes):* "REQ-CHART-135, REQ-CHART-137, REQ-CHART-150, REQ-CHART-175, REQ-CHART-176".
- *11 Nov 2026 (the physics review of PR #482, review 5852979675, its confirmation 2):* the mechanism of (D) is approved, the partner as `place(g·D(φ(u)), frame)` with § 10.4's correctness rule unchanged, and the row-major representative is acceptable as a gauge choice. The written definition is still this task's and still approved here; the physics reviewer holds it to the five points (a) to (e) in the acceptance line of this date. For the owner to know, from their own rulings: with the filter on, the kept half of a mirror pair changes from `η ≥ 0` at `s = 0` to the row-major half at `s = 0⁺`, and it depends on the window's in-plane rotation.
