# TASK-M8-126 — Points of interest: saved signed states, their stars on every chart and path frame, and similar-systems overlays (R-870)

- **Milestone:** M8
- **Closes:** REQ-GUI-426
- **Depends on:** TASK-M2-80, TASK-M7-43, TASK-M7-72
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
- `decisions.md` § "R-899 — The chart-path brief's nine questions: the lock test reads the recorded state at the pin; snap-back undoes the slice only, auto-snap off; jumps under a lock kept and labelled until the sliding anchor exists; the pin's screen position stored, "assumed" at the centre where none can be derived; a pivot record in the view, untilt (a1), amending R-473; points of interest one task in stage 3; their star filled within the lock's tolerance, the off-slice distance shown; marks mid-morph to pixel resolution, the filter off, labelled; the block line built in stage 2 *(amends R-473, and the snap-back of chart_decoder_contract Part 4 and design § 17.8)*"

## Deliverables
- Points saved from the GUI, named; their marks drawn on every chart and every frame of a path from TASK-M2-80's maths; the similar-systems overlays through TASK-M7-43's and TASK-M7-72's contours.

## Acceptance tests
- `cargo test -p gui points_of_interest` — a point lying in the chart draws a filled star at the pixel that decodes to it, within the lock's tolerance, its off-slice distance shown on it; on a mid-morph frame the marks are labelled "to pixel resolution" and the filter is off; a one-property overlay passes through the star; overlays redraw consistently across a morph's frames (the owner's three tests); a point off the chart draws a hollow star at the nearest chart point labelled with its distance; saving or showing a point changes no sim key, payload or statistic; controls: a star drawn one pixel off fails, an overlay of another property's value fails to pass through the star, a point in the sim key fails (REQ-GUI-426).

## Notes
- Added by R-870 (8 Nov 2026).
- **Build order (R-863):** the owner's stages don't name it; it is on RQ-744 (C)'s list. It depends on TASK-M8-108
  (stage 4) for its across-a-morph test, so it can't be built before stage 4; no stage edge is made.
- TASK-M2-57 is in Depends on because a point's signed state on the latent chart needs the full-circle `β` (R-865).
- *8 Nov 2026 (R-873, R-899):* **Build order (applied per R-369):** stage 3, after stage 2 (TASK-M2-65); its across-a-morph test runs on stage 2's line on blocks, so it no longer depends on TASK-M8-108, and the note above is overtaken. The brief's Q7 (c): "exactly" is within the lock's tolerance, the off-slice distance shown, so REQ-GUI-427 no longer owes that definition; Q8 (a): mid-morph marks to pixel resolution, the filter off, labelled. *Was:* "- **Depends on:** TASK-M4-37, TASK-M8-124, TASK-M8-108, TASK-M7-43, TASK-M7-72, TASK-M4-33, TASK-M2-57"
- *9 Nov 2026 (R-916):* the chart core is M2's (R-916, R-917, R-919), so this header follows plan/tasks.yaml. *Was:* "Closes: REQ-GUI-426, REQ-GUI-427, REQ-GUI-428"; "Depends on: TASK-M4-37, TASK-M8-124, TASK-M7-43, TASK-M7-72, TASK-M4-33, TASK-M2-57, TASK-M8-117".
- *9 Nov 2026 (R-916):* split: the maths, its definitions and the near-match tolerance (REQ-CHART-172, REQ-GUI-427, REQ-GUI-428) are TASK-M2-80's; this task keeps the drawing (REQ-GUI-426). *Was (Acceptance):* the § 10.5 review line and the near-match proposal line, now at TASK-M2-80; the two `points_of_interest` lines merged into REQ-GUI-426's verify.
- *11 Nov 2026 (the re-plan's addition (`plan/charts/m2-chart-replan-addition.md`) § F.4; R-947, R-944):* the marks are `place`'s point and residual (TASK-M2-80, TASK-M2-99); mid-morph they meet the lock's tolerance, with the filter on, under (D), applied per R-369 and conditional on physics review; until it confirms, R-899's Q8 (a) is what is drawn and tested. Where the `points_of_interest` line above differs from REQ-GUI-426 as it now reads, the requirement binds.
