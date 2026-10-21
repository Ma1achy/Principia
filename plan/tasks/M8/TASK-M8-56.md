# TASK-M8-56 — The quadtree past the square: roots tiling the plane, canonical addresses, neighbours across seams, poles and folds, scheduling by the union of images, the statistics unchanged (R-537 EX7) ⚑

- **Milestone:** M8
- **Closes:** REQ-SCHED-101, REQ-REF-054, REQ-CHART-079, REQ-CHART-080, REQ-SCHED-104
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
- `decisions.md` § "R-544 — The shape sphere chart is the whole sphere: pixel `(θ, φ)` decodes to the system whose signed `n̂` is that point, R-141's decode gauge removed for it, no hemisphere toggle, no remap and no fold at the equator, every pixel its own system; the mirror a fact about fates, tested and never identified, and shown as information in the initial-condition constructor *(amends R-113, R-141, R-537, R-59, R-178, R-408, R-503, R-538 and R-542; withdraws R-412; settles RQ-498 and RQ-504 where they turn on the hemisphere)*"
- `decisions.md` § "R-570 — Latent and flat charts have no limit on pan or zoom-out: every latent `z` decodes to a system, so a pan never stops there and R-469's stop never fires for it; zoom-out is limited only by the frame budget and the baseline cover, a frame's cost set by the screen, not by the view's extent; every pixel is a system at any pan or zoom; far out the field goes flat by the decoder's saturation, which is correct and is never clamped, rescaled, wrapped or faded; precision is never a navigation limit *(states what R-407, R-467, R-469 and RQ-409 (a) imply; carries RQ-409 (a)'s "no zoom-out limit" from periodic charts to every chart whose axes are affine)*"
- `docs/contracts/principia_scheduler_contract.md` § "Infinite zoom-out: levels coarser than the root"

## Deliverables
- The root lattice, the canonical-address map per extension type and its composition, the union-of-images scheduling, and neighbour lookup through the map: the pure canonical-address map in `crates/kernel/`, the scheduling and the neighbour reads in `crates/engine/` (the quad scheduler, `crates/engine/src/quad`, and the refinement policy).
- Tests `quad_canonical` and `neighbours_across_seams` (`crates/engine`), `stats_canonical` and `tilted_periodic_not_folded` (`crates/kernel`).
- R-570: a frame's cost set by the screen at any zoom-out on a latent or flat chart, by quadtree levels `L < 0` coarser than the root (RQ-535 (a), decided per R-369), and the test `zoom_out_cost_bounded` (`crates/engine`).

## Acceptance tests
- `cargo test -p engine quad_canonical` — REQ-SCHED-101's verify (R-537 T2).
- `cargo test -p engine neighbours_across_seams` — REQ-REF-054's verify (R-537 T7, T8).
- `cargo test -p kernel stats_canonical` — REQ-CHART-079's verify (R-537 T13).
- `cargo test -p kernel tilted_periodic_not_folded` — REQ-CHART-080's verify (R-538 T17).
- `cargo test -p engine zoom_out_cost_bounded` — REQ-SCHED-104's verify (R-570): a frame at 10⁶ × the home view computes no more quads than the baseline cover of the viewport, whose count, at a level `L < 0`, equals the count at home zoom; control: a cover held at `L ≥ 0` grows with the view's extent and fails.

## Notes
- **⚑ Physics review is required** (R-537 U2): the maps against R-14 and R-544 (U10; R-412 withdrawn), the neighbours across poles and folds, and the statistics; it does not merge on the code reviewer's word alone. **Perf** reviews the frame quota and the union-of-images scheduling.
- **2:1 balance** is the refinement policy's `BalanceForced`, whose producer REQ-REF-047 defines (TASK-M6-01); the arms are REQ-REF-025's (TASK-M6-04) (R-537 U11).
- **RQ-410 (a)** applied meanwhile: a level-0 repeat across a pole is drawn from its two level-1 halves.
- **R-538's follow-up (TF2.2, J8; RQ-430 (a) applied meanwhile):** along an axis that mixes a periodic parameter with a non-periodic coordinate (a tilt) the chart is affine: the canonical address folds an axis only where every term with non-zero weight has that period (REQ-CHART-080). Physics reviews it with the maps.
- **The maps (R-537 U10, as R-544 amends it):** R-14's `φ = π(1 − t)`, θ's period and the pole maps at `t = 1` and `t = 0`, and no map at the equator: a quad below it is its own canonical quad, never its mirror's (REQ-SCHED-101). *Was:* "with one hemisphere drawn, R-412's `φ = (π/2)(1 − t)`, the pole map at `t = 1` and the fold at `t = 0`, past `t = −1` the fold then the `t = 1` pole map"; R-544 withdraws R-412 and EX7(b)'s equator fold.
- **RQ-472 waits:** on a tilted view the pole maps are not built until RQ-472 is ruled (REQ-SCHED-101, `rq: RQ-472`; REQ-REF-054's neighbours and REQ-CHART-079's statistics through them); the rest does not wait.
- **RQ-508 waits (R-544):** RQ-472 (a)'s equator fold on a tilted view clashes with the reply's "no folding of any kind on the shape sphere chart"; meanwhile no equator fold is built at any tilt (RQ-508 (a)), and the tilted case waits on RQ-508 with RQ-472 (`rq: RQ-508` on REQ-SCHED-101, REQ-REF-054 and REQ-CHART-079).
- **TASK-M2-36** builds the tilt (R-473) that REQ-CHART-080's test decodes, hence the dependency.
- *Was:* Depends on without TASK-M2-36; acceptance `cargo test -p kernel quad_canonical` and `cargo test -p kernel neighbours_across_seams`; Deliverables "in `crates/kernel/` and the scheduler" and "Tests `quad_canonical`, `neighbours_across_seams` and `stats_canonical`" (code review 5594440613 of PR #263, F2, F3, F5; physics review 5594430380, P2, P3; applied per R-369).
- **R-570 (21 Oct 2026) and RQ-535, decided per R-369 (22 Oct 2026, by the orchestrator, not a human ruling):** zoom-out on a latent or flat chart has no limit and a frame's cost is set by the screen (REQ-SCHED-104). The quadtree gains levels `L < 0`, a level-`L` quad spanning `2^(−L) × 2^(−L)` level-0 roots in R-97's frame, and the baseline cover is taken at the level whose quads are the smallest still at least its on-screen size (`principia_scheduler_contract.md` Part 3, "Infinite zoom-out: levels coarser than the root"). ⚑ Physics reviews the addressing with EX7's maps; perf reviews the cost. *Was:* "**R-570 (21 Oct 2026) and RQ-535 waits:** … REQ-SCHED-104 waits on RQ-535, and the rest of the task does not. Zoom-out is never refused or capped meanwhile."
