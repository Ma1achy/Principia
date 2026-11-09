# TASK-M2-63 — Save view as chart on curved views, the pivot record at a centred pin, and tilts into the held ν₀, L_z and K, in the engine (R-475, R-899, R-916, R-919)

- **Milestone:** M2
- **Closes:** REQ-CHART-167, REQ-CHART-157, REQ-CHART-141, REQ-CHART-188
- **Depends on:** TASK-M2-61, TASK-M2-36, TASK-M2-37, TASK-M2-38, TASK-M2-40, TASK-M2-52
- **Needs (earlier milestones):** none
- **Reviewers:** code, qa, physics
- **Pitfalls:** none
- **Size:** ~350 lines, the tests included

## Goal
Save view as chart on a curved view, and the pivot record at a centred pin, built in the engine through the chart-path object's Save view family (R-864, R-916), so the path object no longer waits on the GUI tasks R-873 (E) (i) named. With them, the tilt into the held `ν₀`, `L_z` and `K`, in M2 by R-919: its slider span is RQ-679's, proposed by the physics reviewer and awaiting the owner's confirmation on the rulings pull request, and the `K_max` and `γ_K` of `Φ_θ,L_z`'s held-`K` warp are this task's calibration (R-833).

## References
- `decisions.md` § "R-475 — "Save view as chart", in the Chart section and on View ›, commits the current view as a named custom chart that can be tilted, sliced and saved again, so tilts compose and any plane is reachable: a latent view saves `(z₀, q₁, q₂)` as an oblique chart, a curved one its parent, slice centre and tilt; reopened at zero tilt it reproduces the view bitwise; one undo entry, the picker's "Saved" group, its name editable and its provenance shown; "twelve tilt directions" is the full count"
- `decisions.md` § "R-477 — The tests of R-469 to R-476, each with an arm that can fail: the same state through two charts, tilt at zero the identity, the two maps equal where they should be and different where not, over the pole, Tip as sheet orthonormal, never leaving the manifold under fuzzed navigation, the mass tilt on the simplex, saved charts round-tripping bitwise, blank never a colour, and exactly the right views re-integrating"
- `docs/gui/principia_render_gui_spec.md` § "G2. Explore — the everyday view (`01_main.png`)"
- `decisions.md` § "R-568 — RQ-378 is parked again, a deferral by ruling with no target: the entry stays open, its option (a) stays applied, nothing waits, and it returns when the human raises it *(amends R-510)*"
- `decisions.md` § "R-916 — M2 is the whole non-visual chart system: the primitive registry, axes from primitives and warps, charts as a pair of axes with their dependence rules, refusals and labels, curved slices, CPU decode and encode through the composition, and the non-visual path maths, each an engine function with its tests, landing before any further downstream chart work, today's charts and view operations re-expressed in it bit for bit first; GPU lowering, contours and display, GUI routes, panels, the Sequence's UI, time-axis mode's UI and overlays keep their milestones as thin adapters onto it; a task mixing maths and visuals is split *(amends R-863; supersedes R-873 (A) (b) and (E) (i); refines R-864 and R-894)*"
- `decisions.md` § "R-899 — The chart-path brief's nine questions: the lock test reads the recorded state at the pin; snap-back undoes the slice only, auto-snap off; jumps under a lock kept and labelled until the sliding anchor exists; the pin's screen position stored, "assumed" at the centre where none can be derived; a pivot record in the view, untilt (a1), amending R-473; points of interest one task in stage 3; their star filled within the lock's tolerance, the off-slice distance shown; marks mid-morph to pixel resolution, the filter off, labelled; the block line built in stage 2 *(amends R-473, and the snap-back of chart_decoder_contract Part 4 and design § 17.8)*"
- `docs/contracts/principia_chart_decoder_contract.md` § "Tilt (basis edit)"
- `decisions.md` § "R-833 — `Φ_θ,L_z`'s held `K` warp takes a `K_max` and `γ_K` of its own, a calibration (R-71), with `K_max` at the chart's calibrated `K` proposed at the gate *(closes RQ-710)*"
- `docs/design/principia_chart_reference.md` § "4.5 The Burrau-family chart maps"
- `decisions.md` § "R-919 — The owner's answers to the chart plan: F1 (a), the merged GUI routes made thin adapters in TASK-M2-64; F10, R1 gates bitwise and R2 is undo; F11, G4's reading; F13 (b), `τ` provisional at M2; F14, the slider spans of `ν₀`, the held `L_z` and the held `K` answered now, proposed by physics for the owner to confirm, and tilts into them in M2; TASK-M2-57's three questions answered; the R-864 type sketch approved when TASK-M2-61 brings it, from type-sketch v10; the trajectory interface's three-body fixtures the Lagrange equilateral homographic solution and the figure-eight, two-body Kepler only where the interface is generic, the constants in a lower crate; F2 to F20 stand; every calibration set by measurement with its criterion written first *(supersedes R-882; refines R-864 and R-902)*"
- `docs/design/principia_chart_reference.md` § "2.1 Feasibility, and the warp that makes every pixel valid"
- `decisions.md` § "R-513 — A new tilt of a saved curved chart into its own saved target composes the rotations, `(a, b, d) = R·R′·(u, v, 0)`, the saved `R` acting on `(u′, v′, d′)`; the saved target is offered again *(closes RQ-403; amends R-473 and R-475)*"

## Deliverables
- `crates/engine`: Save view as chart for curved views (parent chart, slice centre, tilt with angles, targets and map), reopening bitwise, a tilt into the saved target composing `R·R′` (R-513).
- The pivot record in a curved chart's view, stored as the pinned state and the pin's screen position, the kernel tilt gaining the pivot's origin and offset (REQ-CHART-157).
- The tilt into the held `ν₀`, `L_z` and `K`, its span per RQ-679, awaiting the owner's confirmation, written into render_gui_spec § G2 and chart_reference § 2.1 once ruled (R-72; REQ-CHART-188).
- The calibration proposal for `Φ_θ,L_z`'s held-`K` `K_max` and `γ_K`, its criterion written first, set by measurement (R-649/650) (REQ-CHART-141).

## Acceptance tests
- `cargo test -p engine saved_curved_view` — a shape-sphere view tilted into a mass logit saved and reopened at zero tilt: the sim key equal and every pixel's decode bit for bit; a 20° tilt into its saved target decodes as R·R′ applied to (u, v, 0), to 1 ulp; controls: a saved curved chart that drops the slice centre differs, a tilt into the saved target decoded as the offsets added fails (REQ-CHART-167).
- `cargo test -p engine curved_pivot` — on a tilted and zoomed curved chart locked at its centre, the pinned pixel decodes the pinned state after each tilt and zoom about it; the record holds the pinned state and the pin's screen position, d0 equal to zero by value on this never-pinned-off-centre view; never-pinned views bitwise today's; controls: the pivot stored as chart coordinates fails (REQ-CHART-157).
- Review: the proposal states K_max and γ_K for Φ_θ,L_z's held K, with t₀ at the calibrated K and the share of a default tilted view's pixels each value moves; physics review; recorded in decisions.md (REQ-CHART-141).
- Review: DEFINITION (R-72), per RQ-679, awaiting the owner's confirmation: the spans as the owner rules them written into § G2 and § 2.1, approved by the physics reviewer; backed by cargo test -p engine held_span_tilt, the predicates RQ-679's answer sets (the physics reviewer's proposal, § 4: at a single 90° tilt the tilted axis's hidden values equal the sibling axis's at the same u; a pan at fixed τ leaves each span bitwise unchanged; a saved tilted chart reopens bitwise with its stored span), each with its control (REQ-CHART-188).

## Notes
- Split from TASK-M8-48 by R-916 (9 Nov 2026): the engine half of REQ-GUI-244 is REQ-CHART-167; REQ-CHART-157 and REQ-CHART-141 move here. TASK-M8-48 keeps REQ-GUI-241 to REQ-GUI-244, the GUI's side, and depends on this task.
- *R-919 (F14):* RQ-679's spans are proposed by physics; the owner confirms on the rulings pull request. Until then REQ-CHART-188 reads "per RQ-679, awaiting the owner's confirmation", and this task doesn't merge a span the owner hasn't confirmed.
- The MixedEnergy virial-axis held `L_z` has no chart-constant `K_max`; that is RQ-701's, deferred to TASK-M8-48 (R-818), not this task's.
- **Build order (R-916):** stage 1, after TASK-M2-61; TASK-M2-64 and TASK-M2-65 depend on it.
