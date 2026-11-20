# TASK-M2-67 — CPU decode and encode through the composition: every chart through resolve, every inverse through the registry (R-916)

- **Milestone:** M2
- **Closes:** REQ-CHART-170
- **Depends on:** TASK-M2-66, TASK-M2-65
- **Needs (earlier milestones):** none
- **Reviewers:** code, qa, physics
- **Pitfalls:** none
- **Size:** ~400 lines, the tests included

## Goal
R-916 puts CPU decode and encode through the composition in M2: every chart decodes through the registry's `resolve`, and every inverse (encode, lookup exact and partial, ingest) runs through the registry's rows, the per-chart routes deleted, every result bit for bit TASK-M2-61's baseline where no ruled change applies.

## References
- `decisions.md` § "R-916 — M2 is the whole non-visual chart system: the primitive registry, axes from primitives and warps, charts as a pair of axes with their dependence rules, refusals and labels, curved slices, CPU decode and encode through the composition, and the non-visual path maths, each an engine function with its tests, landing before any further downstream chart work, today's charts and view operations re-expressed in it bit for bit first; GPU lowering, contours and display, GUI routes, panels, the Sequence's UI, time-axis mode's UI and overlays keep their milestones as thin adapters onto it; a task mixing maths and visuals is split *(amends R-863; supersedes R-873 (A) (b) and (E) (i); refines R-864 and R-894)*"
- `decisions.md` § "R-917 — The build order: the chart system and its encoder and decoder, complete and tested, as M2; then the integrator; then everything visual; work in flight or approved finishes, and no new integrator or visual task starts until M2's gate passes, but fixes and what M2's own tests need; M2's gate, "fully working and tested", is the checklist G1 to G14 in `plan/MILESTONES.md`, the owner's lock-with-morphs test among it *(amends R-390 and R-627: the GUI track's exception, for new dispatch; refines R-914: the 217 tasks it released wait for the gate)*"
- `docs/design/principia_chart_composition.md` § "3. The primitive registry"
- `docs/design/principia_chart_composition.md` § "7. Lowering: three ways a pair runs"

## Deliverables
- `crates/engine`: decode through `resolve`; encode, lookup and ingest dispatched through the registry's rows; the chart-specific routes deleted.
- The encode-eps-phys gate's interior set extended to every composed chart, at f64 and f32.

## Acceptance tests
- `cargo test -p engine composed_decode_encode` — every chart of kernel::chart::REGISTRY and the gallery decodes through resolve and encodes, looks up (exact and partial) and ingests through the registry's rows, bit for bit the baseline (fixtures/chart_composition/digest.json); the encode-eps-phys gate's interior set extended to every composed chart, T2 within ε_phys at f64 and f32; controls: a chart decoded by its old route beside resolve fails, an inverse dispatched outside the registry fails, a mantissa bit flipped in a digest fails; plus review: a word-bounded git grep finds no deleted route's name (REQ-CHART-170).

## Notes
- New with R-916 (9 Nov 2026); no task held it before.
- **Build order (R-916):** stage 3, after TASK-M2-66.
