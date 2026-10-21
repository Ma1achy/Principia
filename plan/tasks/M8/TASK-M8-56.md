# TASK-M8-56 — The quadtree past the square: roots tiling the plane, canonical addresses, neighbours across seams, poles and folds, scheduling by the union of images, the statistics unchanged (R-537 EX7) ⚑

- **Milestone:** M8
- **Closes:** REQ-SCHED-101, REQ-REF-054, REQ-CHART-079, REQ-CHART-080
- **Depends on:** TASK-M8-05, TASK-M8-44, TASK-M5-15, TASK-M6-01, TASK-M6-04, TASK-M6-11, TASK-M2-36
- **Needs (earlier milestones):** none
- **Reviewers:** code, qa, physics, perf
- **Pitfalls:** none
- **Size:** ~800 lines, the tests included

## Goal
The quadtree covers the extended parameter plane and computes one system once (R-537 EX7): the root lattice tiles the plane, a quad in a repeat maps to its canonical quad by the type group, that quad alone is requested, computed, cached and decided and every image draws it, scheduling reads the union of images, neighbours cross seams, poles and folds through the same map, affine quads past the root are ordinary, not-a-system is never integrated, and the statistics count each system once at its canonical address.

## References
- `decisions.md` § "R-537 — The chart's extension past `[0,1]²` is reached by pan and zoom with the controls shown, not only under F3: wherever the view lands it renders with no gaps, every interaction works there, a pixel in a repeat resolving to its canonical system, and the quadtree, refinement and deep zoom work in extended and wrapped regions, one system computed once ⚑ *(amends R-406, R-407 and R-472)*"
- `decisions.md` § "R-412 — With one hemisphere of the shape sphere drawn, φ's span is remapped to end at the equator and its formula continues past it; the primary range is pole to equator *(closes RQ-264)*"
- `decisions.md` § "R-538 — The trace's markers and icon sets, as `principia_trace_markers.md` gives them: a star at the start, each end event its own icon in its outcome's colour, "?" for an incomplete run, rings where the state leaves and re-enters the chart's reach, syzygies o–o–o and off by default, off-screen icons pinned faint to the edge, every icon repeated in every image of the chart, never drawn through or started in a pixel that is not a system *(amends R-476)*"
- `docs/contracts/principia_chart_decoder_contract.md` § "Past the unit square — each axis's extension type (R-407)"
- `docs/design/principia_coordinate_conventions_note.md` § "The three coordinate spaces (they nest; each is right for its job)"
- `docs/contracts/principia_caching_contract.md` § "Part 4 — Baseline-first: an absolute tier, not a priority weight"
- `docs/design/principia_deep_zoom.md` § "The precision split (the CPU/GPU seam, decode side)"

## Deliverables
- The root lattice, the canonical-address map per extension type and its composition, the union-of-images scheduling, and neighbour lookup through the map: the pure canonical-address map in `crates/kernel/`, the scheduling and the neighbour reads in `crates/engine/` (the quad scheduler, `crates/engine/src/quad`, and the refinement policy).
- Tests `quad_canonical` and `neighbours_across_seams` (`crates/engine`), `stats_canonical` and `tilted_periodic_not_folded` (`crates/kernel`).

## Acceptance tests
- `cargo test -p engine quad_canonical` — REQ-SCHED-101's verify (R-537 T2).
- `cargo test -p engine neighbours_across_seams` — REQ-REF-054's verify (R-537 T7, T8).
- `cargo test -p kernel stats_canonical` — REQ-CHART-079's verify (R-537 T13).
- `cargo test -p kernel tilted_periodic_not_folded` — REQ-CHART-080's verify (R-538 T17).

## Notes
- **⚑ Physics review is required** (R-537 U2): the maps against R-14 and R-412 (U10), the neighbours across poles and folds, and the statistics; it does not merge on the code reviewer's word alone. **Perf** reviews the frame quota and the union-of-images scheduling.
- **2:1 balance** is the refinement policy's `BalanceForced`, whose producer REQ-REF-047 defines (TASK-M6-01); the arms are REQ-REF-025's (TASK-M6-04) (R-537 U11).
- **RQ-410 (a)**, ruled by R-552 (21 Oct 2026), as applied meanwhile: a level-0 repeat across a pole is drawn from its two level-1 halves; the bootstrap is not raised.
- **R-538's follow-up (TF2.2, J8; RQ-430 (a), ruled by R-552):** along an axis that mixes a periodic parameter with a non-periodic coordinate (a tilt) the chart is affine: the canonical address folds an axis only where every term with non-zero weight has that period (REQ-CHART-080). Physics reviews it with the maps.
- **The maps by hemispheres drawn (R-537 U10):** with both hemispheres drawn, R-14's `φ = π(1 − t)`, the pole maps at `t = 1` and `t = 0`; with one hemisphere drawn, R-412's `φ = (π/2)(1 − t)`, the pole map at `t = 1` and the fold at `t = 0`, past `t = −1` the fold then the `t = 1` pole map (REQ-SCHED-101).
- **RQ-472 (a), ruled by R-554:** on a tilted view the pole maps apply iff `N_a = 0` and `N_b = 0`, decided exactly on the stored angles, and otherwise the axis is affine with no image (REQ-SCHED-101; REQ-REF-054's neighbours and REQ-CHART-079's statistics through them); the equator fold is not built on a tilted view, the human's shape sphere ruling of 21 Oct governing it (that clause of RQ-472's ruling not applied). *Was:* "**RQ-472 waits:** on a tilted view the pole maps and the equator fold are not built until RQ-472 is ruled (REQ-SCHED-101, `rq: RQ-472`; REQ-REF-054's neighbours and REQ-CHART-079's statistics through them); the rest does not wait."
- **TASK-M2-36** builds the tilt (R-473) that REQ-CHART-080's test decodes, hence the dependency.
- *Was:* Depends on without TASK-M2-36; acceptance `cargo test -p kernel quad_canonical` and `cargo test -p kernel neighbours_across_seams`; Deliverables "in `crates/kernel/` and the scheduler" and "Tests `quad_canonical`, `neighbours_across_seams` and `stats_canonical`" (code review 5594440613 of PR #263, F2, F3, F5; physics review 5594430380, P2, P3; applied per R-369).
