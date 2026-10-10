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
toggle TASK-M8-06's), the view past `[0,1]²` follows RQ-264's ruling: what the φ axis does past the edge of the chart's
span that is not a pole, and its primary range there, both for the drawing (R-407's extension and fallback) and for the
area statistics (R-408's counting). TASK-M8-05 builds the extension types and the counting with both hemispheres drawn;
this task adds the one-hemisphere case to them and changes nothing with both hemispheres drawn. It waits for RQ-264.

## References
- `decisions.md` § "R-408 — Area statistics count each system once, through the axis types: a visible pixel counts only if every axis is inside its primary range; domain-hatched pixels leave the count and the total; only validity failures are forbidden *(closes RQ-263)*"
- `decisions.md` § "R-407 — Past `[0,1]²` each chart axis extends by the type it declares: affine, periodic, pole-crossing or bounded, the default; a pixel that fails is hatched as forbidden *(closes RQ-262)*"
- `decisions.md` § "R-141 — The shape sphere is 2-to-1 over its φ hemispheres *(closes RQ-71, corrects R-104)*"
- `docs/contracts/principia_chart_decoder_contract.md` § "Past the unit square — each axis's extension type (R-407)"
- `docs/design/principia_chart_reference.md` § "3.3 The chart map"
- `docs/design/principia_chart_reference.md` § "5.4 Past the unit square — each axis's extension type (R-407)"
- `docs/gui/principia_render_gui_spec.md` § "G2. Explore — the everyday view (`01_main.png`)"

## Deliverables
- In `crates/kernel/src/chart/`, the shape sphere's φ axis, with one hemisphere drawn, given the extension and the
  primary range RQ-264's ruling fixes, read by TASK-M8-05's extension types and classifier.
- Cases added to the tests `chart_extension` and `area_stats_primary_range` for the sphere with one hemisphere drawn.

## Acceptance tests
- `cargo test -p kernel chart_extension` and `cargo test -p kernel area_stats_primary_range` and Review checklist
  (physics reviewer) — on the shape sphere with one hemisphere drawn, past the edge of the chart's span that is not a
  pole, the drawing and the counting follow RQ-264's ruling; with both hemispheres drawn every output is bit-identical
  to TASK-M8-05's; control: a φ axis built to the option the ruling rejects fails; the physics reviewer checks it
  against the ruling (REQ-CHART-057).

## Notes
- Split from TASK-M8-05 so that RQ-264 holds only this leaf (applied per R-369, R-408 B4, code review 5478800754, F4).
  Nothing depends on it. It starts once RQ-264 is ruled; its deliverables and acceptance line follow the ruling.
- It depends on TASK-M8-06, which builds the hemisphere toggle in the Manifold view's Chart section (REQ-GUI-161), and
  through it on TASK-M8-05. The hemisphere label and render are TASK-M2-28's (REQ-CHART-002, M2).
