# TASK-M2-68 — Region D on the CPU: the third-body pair, its preset, its velocities and the closed round trips (R-916)

- **Milestone:** M2
- **Closes:** REQ-CHART-101, REQ-CHART-142, REQ-CHART-171, REQ-CHART-180
- **Depends on:** TASK-M2-66, TASK-M2-67, TASK-M2-20, TASK-M2-65
- **Needs (earlier milestones):** none
- **Reviewers:** code, qa, physics
- **Pitfalls:** none
- **Size:** ~350 lines, the tests included

## Goal
The first registered closed-form pair on the CPU: `(ξ, η)` through the explicit-configuration path at any masses, region D as a preset of it (the full disc, unmasked, R-871), exact rest and, at equal masses, eq. 3's velocities, the closed rows' round trips, and stage 2's region-D slide arm (F9). Its baked entry, GPU reference and lowering row stay TASK-M4-34's.

## References
- `docs/design/principia_chart_composition.md` § "8.3 The pairs it carries"
- `docs/design/principia_chart_composition.md` § "12. Tests that can fail"
- `decisions.md` § "R-851 — Region D's default velocities are exact rest, with eq. 3 the second preset; AA-3 and AA-5 read eq. 3, AA-8 runs under both, eq. 3 reproduces Agekyan & Anosova's velocities in their own units before canonicalising, and its 19% spread at `I = 1` is stated beside the preset *(on RQ-714, which stays open)*"
- `docs/design/principia_chart_composition.md` § "8.6 The momentum, and the rescale per pixel"
- `docs/design/principia_dd_replication.md` § "5.8 Region D: Agekyan & Anosova, and Anosova 1969"
- `docs/design/principia_chart_composition.md` § "8.2 The closed form"
- `docs/design/principia_chart_composition.md` § "8.4 What the chart is: the domain"
- `docs/contracts/principia_lowering_contract.md` § "Appendix — worked enumeration of the current chart set"
- `decisions.md` § "R-871 — Region D's chart is the plane, unmasked: the full disc rendered literally, every mirror and body swap declared, so the symmetry filter leaves exactly region D with no mask, on by default for the paper-frame presets only; its check, the filtered area 0.307 at equal masses and rest *(closes RQ-714; on RQ-738, which stays open)*"
- `decisions.md` § "R-914 — The stage 1 / stage 3 dependency cycle, RQ-752: option (a), the edge from TASK-M4-06 (the GPU compute variants) to TASK-M4-34 (region D) dropped; the GPU reference that reads region D moves into TASK-M4-34, which can't close until that reference reads region D as R-871 requires; R-871's substance unchanged, only the task that applies its link; TASK-M4-06 and the 217 tasks queued behind it released *(closes RQ-752)*"
- `decisions.md` § "R-916 — M2 is the whole non-visual chart system: the primitive registry, axes from primitives and warps, charts as a pair of axes with their dependence rules, refusals and labels, curved slices, CPU decode and encode through the composition, and the non-visual path maths, each an engine function with its tests, landing before any further downstream chart work, today's charts and view operations re-expressed in it bit for bit first; GPU lowering, contours and display, GUI routes, panels, the Sequence's UI, time-axis mode's UI and overlays keep their milestones as thin adapters onto it; a task mixing maths and visuals is split *(amends R-863; supersedes R-873 (A) (b) and (E) (i); refines R-864 and R-894)*"
- `decisions.md` § "R-917 — The build order: the chart system and its encoder and decoder, complete and tested, as M2; then the integrator; then everything visual; work in flight or approved finishes, and no new integrator or visual task starts until M2's gate passes, but fixes and what M2's own tests need; M2's gate, "fully working and tested", is the checklist G1 to G14 in `plan/MILESTONES.md`, the owner's lock-with-morphs test among it *(amends R-390 and R-627: the GUI track's exception, for new dispatch; refines R-914: the 217 tasks it released wait for the gate)*"
- `decisions.md` § "R-919 — The owner's answers to the chart plan: F1 (a), the merged GUI routes made thin adapters in TASK-M2-64; F10, R1 gates bitwise and R2 is undo; F11, G4's reading; F13 (b), `τ` provisional at M2; F14, the slider spans of `ν₀`, the held `L_z` and the held `K` answered now, proposed by physics for the owner to confirm, and tilts into them in M2; TASK-M2-57's three questions answered (RQ-754): one periodic `β` link, the presets and goldens along `z_β` redefined, an unpinned saved chart converted from `sigmoid_beta` with a notice; the R-864 type sketch approved when TASK-M2-61 brings it, from type-sketch v10; the trajectory interface's three-body fixtures the Lagrange equilateral homographic solution and the figure-eight, two-body Kepler only where the interface is generic, the constants in a lower crate; F2 to F20 stand; every calibration set by measurement with its criterion written first *(closes RQ-754; supersedes R-882; refines R-864 and R-902)*"
- `docs/design/principia_dd_validation_orbits.md` § "1.6 Agekyan–Anosova region D — the near-rest starts (R-592)"

## Deliverables
- `crates/kernel/src/chart/third_body.rs`: the pair, `Chart`, `ChartDecode`, `Tiltable`, ending in the signed canonicalisation.
- The registry's rows and the preset; region D's velocity presets (R-851).
- Choice V5 decided, with its evidence in the PR.
- The calibration proposal for the closed rows' round-trip tolerance, its criterion written first, set by measurement (R-649/650) (REQ-CHART-180).

## Acceptance tests
- `cargo test -p kernel closed_inverse_round_trip` — every closed row of the registry, the virial row by its mapping's forward at a held L_z ≠ 0; each case reads back the row's Holds; control: a named mutation per row that fires there (design § 12: a factor or exponent on a scale solve) fails (REQ-CHART-101).
- `cargo test -p engine region_d_velocity_presets` — the preset opens at exact rest, q = 0 bitwise; the eq. 3 preset's velocities in Agekyan & Anosova's units equal eq. 3's printed values before canonicalising, and after C the speed ratio across D's equal-mass corners is √(ℓ_max/ℓ_min) to f64 rounding; the preset's statement names the spread; controls: eq. 3 applied after canonicalising fails the units arm; the statement dropped fails (REQ-CHART-142).
- `cargo test -p kernel region_d_plane` — every Table 1 row's pixel decodes to the import's canonical state within REQ-VAL-120's tolerance; eq. 8's edges; eq. 3's momenta at √ℓ; I = 1; the stereographic identity at random masses; the signed half: on both presets, every pixel with η < 0 decodes to Π of the (ξ, −η) pixel's state to the decode's rounding, λ̃_y < 0 kept, and at eq. 3 with L_z opposite; every pixel with η ≥ 0 is bit-identical to the decode ending in canonicalise; and stage 2's region-D slide (REQ-CHART-115's arm, F9); controls: bodies 0 and 1 swapped misses the rows, q not scaled by √ℓ fails, μ_ρ/μ_λ inverted fails, d dropped at unequal masses fails, a decode ending in canonicalise (step 3 taken) fails (REQ-CHART-171).
- Review: the proposal gives the tolerance at f64 and f32 with its criterion, written before the run, and the measured worst over a stated sample (`cargo test -p kernel closed_inverse_round_trip`); checked by the physics reviewer; confirmed at the M2 gate and recorded in decisions.md (REQ-CHART-180).

## Notes
- Split from TASK-M4-34 by R-916 (9 Nov 2026): REQ-CHART-171 is REQ-CHART-100's CPU half; REQ-CHART-101 and REQ-CHART-142 move here. Stage 2's region-D slide arm moves here from TASK-M2-65, since region D lands in stage 3 (F9).
- The preset's base point, centre and window: RQ-711, ruled (R-888).
- **Build order (R-916):** stage 3.
