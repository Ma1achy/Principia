# TASK-M2-100 — The measurements P2 and P4, reported to the owner, with P3's timing (R-951)

- **Milestone:** M2
- **Closes:** REQ-CHART-201, REQ-CHART-202, REQ-CHART-203
- **Depends on:** TASK-M2-99, TASK-M2-90, TASK-M2-82
- **Needs (earlier milestones):** none
- **Reviewers:** code, qa, physics
- **Pitfalls:** none
- **Size:** a measurement and its report; no pass line

## Goal
Two measurements of a trace on a blend, off the chart's surface, where almost every trace vertex is: P2 and P4 (R-951). Both are reported, not gates: neither has a pass line, and no threshold and no fallback is picked by the implementer, the orchestrator or a reviewer. The owner sees both before TASK-M8-49 and TASK-M8-58 (the trace tasks) are built, and decides then.

## References
- `decisions.md` § "R-951 — Two measurements before the trace tasks are built: P4, beside P2, the share of neighbouring trace vertices that land more than 8 pixels apart on a blend at `s = 0.25`, `0.5` and `0.75`, set against each end chart's own trace, with the median and worst residual; P2 and P4 are reported, not gates, shown to the owner before TASK-M8-49 and TASK-M8-58 are built, and no threshold or fallback is picked by anyone else *(no RQ; RQ-785, new, asks the picture size the pixels are counted at)*"
- `decisions.md` § "R-947 — Placement, the re-plan addition's question (A): option (a), one placement function for "where is this state on this chart", two routes chosen by the chart, the existing reading where a chart has an inverse and the nearest point everywhere else, every blend included, refined to the lock's precision on the CPU *(no RQ; answers the re-plan addition's question (A); refines R-542 and R-926)*"
- `decisions.md` § "R-919 — The owner's answers to the chart plan: F1 (a), the merged GUI routes made thin adapters in TASK-M2-64; F10, R1 gates bitwise and R2 is undo; F11, G4's reading; F13 (b), `τ` provisional at M2; F14, the slider spans of `ν₀`, the held `L_z` and the held `K` answered now, proposed by physics for the owner to confirm, and tilts into them in M2; TASK-M2-57's three questions answered (RQ-754): one periodic `β` link, the presets and goldens along `z_β` redefined, an unpinned saved chart converted from `sigmoid_beta` with a notice; the R-864 type sketch approved when TASK-M2-61 brings it, from type-sketch v10; the trajectory interface's three-body fixtures the Lagrange equilateral homographic solution and the figure-eight, two-body Kepler only where the interface is generic, the constants in a lower crate; F2 to F20 stand; every calibration set by measurement with its criterion written first *(closes RQ-754; supersedes R-882; refines R-864 and R-902)*"
- `decisions.md` § "R-649 — Replication and verification targets are measurements, not gates: failing one never blocks building; each declares the tier it binds, (a) the GPU explorer, agreeing statistically or qualitatively only, or (b) the CPU IC inspector, where exact targets bind; three outcomes, PASS, EXPLAINED DIVERGENCE (shown by a measurement) and UNEXPLAINED (open, labelled, non-blocking); no default, physics, criterion, architecture or tolerance is changed and nothing is tuned per target in pursuit of a pass; a comparison is checked like-for-like before anything is called a failure; verification deepening comes after the core build *(amends R-594, R-636, R-637 and R-639 for external targets; applies to RQ-598's targets and every target in force)*"
- `decisions.md` § "R-72 — A missing definition is written by the task that needs it *(closes RQ-46 to RQ-55, definitions)*"
- `decisions.md` § "R-186 — GitHub-hosted runners first; no self-hosted runner *(amends R-110, R-169, R-174)*"
- `decisions.md` § "R-923 — Integrator work before charts, for now: R-917's order (charts first, then integrator, then visuals) is set aside for now, on the owner's say; only integrator tasks that don't depend on the points the owner hasn't decided are taken, nothing that fixes how starting states reach the integrator on the GPU or the horizon as one value per dispatch; TASK-M3-14 stays stopped under R-918; no new chart task starts meanwhile *(amends R-917 in part, for now; no RQ)*"
- `docs/design/principia_chart_composition.md` § "10.6 Placement: one function for "where is this state on this chart""
- `docs/design/principia_chart_composition.md` § "18.7 The trajectory interface (R-918, R-919)"
- `docs/design/principia_lines.md` § "8. Traces"

## Deliverables
- The measurement harness and **the report, shown to the owner through the rulings report**, with every number below, at the picture size the owner states (REQ-CHART-202).
- The set-up both share: the trajectory fixtures of R-919's F18, built by TASK-M2-82 (the figure-eight and the Lagrange equilateral solution); two blend pairs (the latent chart to the shape sphere's exponential map, and the `(L_z, E)` chart to the shape sphere's exponential map); 16 seeded pixels per pair, **the seed written into this file before any run**; the placement TASK-M2-99's `place`, on the end charts by their own routes and on the blend by its nearest-point route.

## Acceptance tests
- Report: the report exists with every number: P2 (median and worst jump in pixels per vertex between `s = 0` and `s = 10⁻⁶`); P4 (at `s = 0.25`, `0.5` and `0.75`, the share of neighbouring trace vertices more than 8 pixels apart on the blend, against the same share on each end chart's own trace; the median and worst residual); P3's timing (the time to place a trace of a stated length on a blend frame, on the Mac (R-186), one thread, the slowest of five warm runs, a number with no pass line); each at the picture size the owner states (REQ-CHART-202); no pass line anywhere; and it has been shown to the owner (REQ-CHART-201).
- Definition: RQ-785 is answered by the owner and the picture size is written into this file before any run; nothing was run before it (REQ-CHART-202).
- Definition: the owner has stated what a seeded pixel contributes and whether P2 is also read at `s = 1` against `s = 1 − 10⁻⁶`, and both are written into this file before any run (REQ-CHART-203).

## Notes
- Entered 11 Nov 2026 as N20 of the re-plan's addition (`plan/charts/m2-chart-replan-addition.md`) (applied per R-369: task scoping); its acceptance is the addition's § A.5; R-951. The plan documents are under `plan/charts/`; the lines above carry what they say.
- **Entered, not released (R-923).** R-923 (integrator work before charts, for now) holds the chart tasks TASK-M2-57 to TASK-M2-85, TASK-M2-82 excepted, and allows no new chart task meanwhile. Entering this task in `plan/tasks.yaml` does not lift the hold: it is a chart task, entered held, and it is not dispatched until the owner lifts R-923's hold, and it runs nothing until RQ-785 (at what picture size P2's and P4's pixels are counted) is answered.
- **Owed before any run, stated here and not guessed (the addition's § A.5):** the two points of REQ-CHART-203. Neither is chosen by the implementer, the orchestrator or a reviewer.
- P3's timing is carried here as a reported number with no pass line (applied per R-369). P3's pass line and degrade rule, fading a trace by its residual as a remedy, and a trace placed by the share-weighted mix of the ends' readings are proposals nothing adopts: none is applied by an implementer or a reviewer.
- The owner's words (10 Nov 2026): "P2 and P4 are reported, not gates. Show me both before TASK-M8-49 and TASK-M8-58 are built. If the numbers are bad I will decide then; the likely fallback is fading the trace by its residual, which your draft already has. Do not pick thresholds or fallbacks yourself."
- TASK-M8-49 and TASK-M8-58 depend on this task and are not built until the owner has seen its report.
