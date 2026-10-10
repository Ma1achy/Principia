# TASK-M8-44 — The shape sphere past [0,1]² with one hemisphere drawn: φ's extension and primary range (RQ-264)

- **Milestone:** M8
- **Closes:** REQ-CHART-057
- **Depends on:** TASK-M8-05, TASK-M8-06
- **Needs (earlier milestones):** REQ-CHART-002
- **Reviewers:** code, qa, physics
- **Pitfalls:** none
- **Size:** ~150 lines

## Goal
With the shape sphere's hemisphere toggle drawing one hemisphere (R-113: the label and render are TASK-M2-28's, the GUI
toggle TASK-M8-06's), the view past `[0,1]²` follows R-412 (RQ-264's option (a)): φ's `[0,1]` span is remapped to run
from the pole to the equator; past the equator, the edge that is not a pole, its formula continues into the mirror
hemisphere, real and repeated systems; its primary range is pole to equator, so the continuation is not counted, both
for the drawing (R-407's extension and fallback) and for the area statistics (R-408's counting). TASK-M8-05 builds the extension types and the counting with both hemispheres drawn;
this task adds the one-hemisphere case to them and changes nothing with both hemispheres drawn.

## References
- `decisions.md` § "R-412 — With one hemisphere of the shape sphere drawn, φ's span is remapped to end at the equator and its formula continues past it; the primary range is pole to equator *(closes RQ-264)*"
- `decisions.md` § "R-408 — Area statistics count each system once, through the axis types: a visible pixel counts only if every axis is inside its primary range; domain-hatched pixels leave the count and the total; only validity failures are forbidden *(closes RQ-263)*"
- `decisions.md` § "R-407 — Past `[0,1]²` each chart axis extends by the type it declares: affine, periodic, pole-crossing or bounded, the default; a pixel that fails is hatched as forbidden *(closes RQ-262)*"
- `decisions.md` § "R-141 — The shape sphere is 2-to-1 over its φ hemispheres *(closes RQ-71, corrects R-104)*"
- `docs/contracts/principia_chart_decoder_contract.md` § "Past the unit square — each axis's extension type (R-407)"
- `docs/design/principia_chart_reference.md` § "3.3 The chart map"
- `docs/design/principia_chart_reference.md` § "5.4 Past the unit square — each axis's extension type (R-407)"
- `docs/gui/principia_render_gui_spec.md` § "G2. Explore — the everyday view (`01_main.png`)"

## Deliverables
- In `crates/kernel/src/chart/`, the shape sphere's φ axis, with one hemisphere drawn, given the extension and the
  primary range R-412 fixes (`φ = (π/2)·(1 − t)`, pole-crossing at the pole edge, the formula continued past the
  equator, primary range the chart's `[0,1]` span), read by TASK-M8-05's extension types and classifier.
- Cases added to the tests `chart_extension` and `area_stats_primary_range` for the sphere with one hemisphere drawn.

## Acceptance tests
- `cargo test -p kernel chart_extension` and `cargo test -p kernel area_stats_primary_range` and Review checklist
  (physics reviewer) — on the shape sphere with one hemisphere drawn: `t = 1` is the pole and `t = 0` the equator;
  past the equator the pixels decode by the formula's continuation to the mirror hemisphere's systems, none hatched,
  and leave the count and the total, so a window reaching past the equator gives the same count and total as `[0,1]²`
  alone; past the pole φ is pole-crossing as with both drawn; with both hemispheres drawn every output is bit-identical
  to TASK-M8-05's; controls: a φ axis hatching past the equator (RQ-264's option (b)) and one counting the
  continuation each fail; the physics reviewer checks it against R-412 (REQ-CHART-057).

## Notes
- Split from TASK-M8-05 so that RQ-264 holds only this leaf (applied per R-369, R-408 B4, code review 5478800754, F4).
  Nothing depends on it. RQ-264 is ruled (R-412, 10 Oct 2026, option (a)), so it no longer waits; its deliverables and
  acceptance line follow R-412. Applied per R-369 in R-412's port: the remap `φ = (π/2)·(1 − t)` keeps the pole at
  `t = 1`; the hemisphere drawn is the upper, `w ≥ 0`, the canonical decode's; past `t = −1` the continuation reaches
  the far pole, where φ's pole-crossing applies as R-407 gives it.
- It depends on TASK-M8-06, which builds the hemisphere toggle in the Manifold view's Chart section (REQ-GUI-161), and
  through it on TASK-M8-05. The hemisphere label and render are TASK-M2-28's (REQ-CHART-002, M2).
