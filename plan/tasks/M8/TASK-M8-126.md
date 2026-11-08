# TASK-M8-126 — Points of interest: saved signed states, their stars on every chart and path frame, and similar-systems overlays (R-870)

- **Milestone:** M8
- **Closes:** REQ-GUI-426, REQ-GUI-427, REQ-GUI-428
- **Depends on:** TASK-M4-37, TASK-M8-124, TASK-M8-108, TASK-M7-43, TASK-M7-72, TASK-M4-33, TASK-M2-57
- **Needs (earlier milestones):** none
- **Reviewers:** code, qa, physics, gui
- **Pitfalls:** none
- **Size:** ~500 lines, the tests included

## Goal
The owner's feature of 8 Nov 2026 (R-870): any pixel's starting state saved as a named point, stored as the signed
state; on every chart and every frame of every path, a filled star where the chart passes through the point exactly,
otherwise a hollow star at the nearest chart point labelled with its off-slice distance; optional "similar systems"
overlays from the primitive registry, by the Tier 1 contour machinery and forward formulas only. Display only, never
scored, distinct from the lock.

## References
- `decisions.md` § "R-870 — Points of interest: any pixel's starting state saved as a named point, shown on every chart and every frame of every path as a filled star where the chart passes through it exactly, otherwise a hollow star at the nearest chart point labelled with its off-slice distance, with optional "similar systems" overlays; display only, never scored, distinct from the lock"
- `decisions.md` § "R-72 — A missing definition is written by the task that needs it *(closes RQ-46 to RQ-55, definitions)*"
- `decisions.md` § "R-71 — A missing value becomes a calibration requirement *(closes RQ-46 to RQ-55, values)*"
- `decisions.md` § "R-854 — Symmetry reductions apply only to how a starting state is described at `t = 0`: trajectories evolve and are shown in full physical space, nothing is folded during a run, and the only operation besides the step is the per-step centre-of-mass projection, energy and `L_z` monitored, never corrected"
- `docs/design/principia_chart_composition.md` § "10.5 Points of interest"
- `docs/design/principia_chart_composition.md` § "10.2 Tier 1 overlays"
- `docs/design/principia_chart_composition.md` § "3. The primitive registry"

## Deliverables
- The definitions R-870 leaves unstated, written into chart_composition § 10.5 under physics review (R-72;
  REQ-GUI-427): "exactly", the nearest chart point, the off-slice metric, the near-match measure, marks on a path frame
  with no inverse, the state group holding the points, off the sim key, and the gauge in which property values and the
  off-slice distance are compared (pixels decode at `I = 1`; the mirror not quotiented, R-854).
- Points saved, named and stored as signed states; their marks drawn on every chart and every frame of a path; the
  similar-systems overlays through TASK-M7-43's and TASK-M7-72's contours.
- The near-match tolerance's proposal, with its evidence (R-71; REQ-GUI-428).

## Acceptance tests
- `cargo test -p gui points_of_interest` — the owner's three tests: a point lying in the chart draws a filled star at the
  pixel that decodes to it; a one-property overlay passes through the star; overlays redraw consistently across a
  morph's frames; and a point off the chart draws a hollow star at the nearest chart point labelled with its distance;
  saving or showing a point changes no sim key, payload or statistic; controls: a star one pixel off, an overlay of
  another property's value, and a point in the sim key, each failing (REQ-GUI-426).
- Review: § 10.5's definitions, approved by the physics reviewer (REQ-GUI-427).
- Proposal: the near-match tolerance with its evidence, checked by the physics reviewer, confirmed by the owner at the M8
  gate and recorded in decisions.md (REQ-GUI-428).

## Notes
- Added by R-870 (8 Nov 2026).
- **Build order (R-863):** the owner's stages don't name it; it is on RQ-744 (C)'s list. It depends on TASK-M8-108
  (stage 4) for its across-a-morph test, so it can't be built before stage 4; no stage edge is made.
- TASK-M2-57 is in Depends on because a point's signed state on the latent chart needs the full-circle `β` (R-865).
