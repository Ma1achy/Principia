# TASK-M6-11 — Tile priority: P_tile, the pointer-centred P_focus, quad-relative relevance and baseline-first

- **Milestone:** M6
- **Closes:** REQ-SCHED-056, REQ-SCHED-065, REQ-SCHED-066, REQ-SCHED-067, REQ-SCHED-071, REQ-SCHED-080, REQ-SCHED-081, REQ-SCHED-090
- **Depends on:** TASK-M6-02, TASK-M5-13, TASK-M5-24
- **Needs (earlier milestones):** REQ-SCHED-019, REQ-SCHED-023, REQ-SCHED-074, REQ-DEC-031, REQ-CHART-038
- **Reviewers:** code, qa, physics, perf
- **Pitfalls:** none
- **Size:** ~400 lines

## Goal
The camera is wired into scheduler priority (R-44): `P_tile = w_v·P_visible + w_z·P_zoom + w_c·P_complexity + w_f·P_focus` at defaults 10/2/3/1, weights exposed in research mode; `P_focus` is centred on the pointer while it is in view and on the viewport centre otherwise (R-55); relevance terms are computed relative to the camera or quad centre with centre-plus-half-width, not global UV (R-46). No refinement job dispatches while the baseline cover is missing. The task writes the reconciled priority rule (Part 6 vs policy §0.1) and `P_focus`'s decay law into scheduler_contract Part 6 (R-72).

## References
- `docs/contracts/principia_caching_contract.md` § "Part 4 — Baseline-first: an absolute tier, not a priority weight"
- `docs/contracts/principia_caching_contract.md` § "Part 8 — The composed guarantee"
- `docs/contracts/principia_scheduler_contract.md` § "Part 6 — The settled policy"
- `docs/design/principia_debug_tooling_plan.md` § "F. Structural views — `RenderQuad` / quadtree (read quad metadata, not payload)"
- `decisions.md` § "R-55 — Cursor bias is `P_focus` centred on the pointer *(GU-4 (a))*"
- `docs/design/principia_deep_zoom.md` § "1. Quad-local coordinates — UV precision"
- `decisions.md` § "R-46 — Relevance is computed relative to the camera or quad centre *(RS-6)*"
- `docs/read_first/principia_INDEX.md` § "Known open items"
- `decisions.md` § "R-44 — The camera is wired; scheduler Part 6 is reconciled with policy §0.1 *(RS-4)*"
- `docs/design/principia_dd_refinement_policy.md` § "0.1 In view, the camera decides depth and the criterion decides ORDER"
- `decisions.md` § "R-72 — A missing definition is written by the task that needs it *(closes RQ-46 to RQ-55, definitions)*"
- `decisions.md` § "R-71 — A missing value becomes a calibration requirement *(closes RQ-46 to RQ-55, values)*"

## Deliverables
- Doc change: `docs/contracts/principia_scheduler_contract.md` § "Part 6 — The settled policy" — the reconciled off-screen rule and how order in view enters `P_tile` (REQ-SCHED-080); `P_focus`'s decay law (REQ-SCHED-081), any constant it introduces raised as a calibration (R-71).
- `crates/engine/src/sched/priority.rs`: `P_tile`, the four terms, weights in a research-mode settings struct.
- Baseline-first gate in the dispatcher: refinement jobs held until the baseline cover is complete.
- `cargo xtask gate relevance-deep` (depth ≥ 40, f64 reference ordering).

## Acceptance tests
- `cargo test -p engine baseline_first_nav` (proptest) — random navigations: assert no refinement dispatch precedes baseline completion (REQ-SCHED-056).
- `cargo test -p engine p_tile_defaults` — priority of fixture quads matches the formula at default weights; the research-mode settings expose all four weights (REQ-SCHED-065).
- `cargo test -p engine p_focus_pointer` — with pointer in view, the quad under the pointer gets maximal P_focus; with pointer out of view, the centre quad does (REQ-SCHED-066).
- `cargo xtask gate relevance-deep` — at depth ≥ 40 (where the global-UV defect shows), P_visible and P_focus match an f64 reference ordering of quads exactly (REQ-SCHED-067).
- `cargo test -p engine pan_changes_priority` — Pan sequence regression: the quadtree/priority order changes with the camera (a byte-identical tree at every pan step fails, cf. philosophy §4.4) (REQ-SCHED-071).
- Review checklist (physics) — Part 6 and policy §0.1 no longer disagree about off-screen quads; the doc says how order in view enters P_tile; the doc change is merged with the physics reviewer's approval (REQ-SCHED-080).
- Review checklist (physics) — the doc states the decay law; any constant it introduces is recorded as a calibration (R-71); the doc change is merged with the physics reviewer's approval (REQ-SCHED-081).
- Proposal: the P_focus decay constants with scripted-hover evidence; the human confirms them at the M6 gate (REQ-SCHED-090).

## Notes
- Definitions written: REQ-SCHED-080, REQ-SCHED-081. A decay constant introduced by REQ-SCHED-081 is REQ-SCHED-090 (R-71), not chosen here.
- Closes, for gaps the corpus leaves open: REQ-SCHED-090 (R-71 calibration) (classification accepted by R-132).
