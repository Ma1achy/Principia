# TASK-M7-09 — ScalarField sources, derived operators and the physics site generators

- **Milestone:** M7
- **Closes:** REQ-COL-015, REQ-COL-021
- **Depends on:** TASK-M7-05, TASK-M7-06
- **Needs (earlier milestones):** REQ-COL-003, REQ-COL-005, REQ-COL-006, REQ-RENDER-008, REQ-VAL-023
- **Reviewers:** code, qa, physics
- **Pitfalls:** none
- **Size:** ~500 lines

## Goal
FieldRamp's ScalarField sources, each returning (value, valid) through `ctx`: payload fields (through the generated accessors), geometry-of-n̂ fields (n_z, ‖n̂‖, azimuth/polar in R-14's names, stability, real spherical harmonics ℓ ≤ 3, the Turing wave-triple, 4-octave 3-D value noise), ctx lanes, the lattice classifiers `bands` uses (checker, latitude/longitude stripes, Truchet on (θ, φ)), and the derived operators gradient_magnitude (three evaluations at ±ε) and topk_margin = d_1 − d_2. Alongside, the physics site generators BC(m), Euler(m), Lagrange(m): computed from the decoded IC masses per pixel on the mass-weighted shape sphere (R-14, R-50), with the Euler landmarks the Euler central configurations — the collinear relative equilibria, roots of Euler's quintic in the mass ratios — mapped through the shape map (R-126), hoisted to uniforms only when no basis axis or active tilt touches a mass dimension (an allowed optimisation, not a bake tier, R-121).

## References
- `docs/design/principia_colour_composition.md` § "1.2 Family B — field-ramp  →  `vec3` or `f32`"
- `docs/design/principia_colour_composition.md` § "2. Site-set kinds"
- `docs/design/principia_dd_colouring.md` § "3.4 Physics overlay (blob blend) and the house encoding (stability × hue)"
- `decisions.md` § "R-14 — One shape-sphere convention, the IC Inspector's *(closes RQ-12, corrects R-12's premise)*"
- `decisions.md` § "R-50 — The shape-sphere collision landmarks are mass-weighted *(CO-1 (a))*"
- `docs/design/principia_dd_integrator.md` § "3.7 The shape readout and winding (live, per macro-step — lockstep, ratified)"
- `decisions.md` § "R-121 — The physics overlay isn't baked *(closes RQ-89)*"
- `decisions.md` § "R-124 — Apply the R-25, R-50 and R-102 follow-ups now *(closes RQ-92)*"
- `decisions.md` § "R-126 — The Euler landmarks are the Euler central configurations *(closes RQ-106)*"

## Deliverables
- `crates/render/shaders/wgsl/lib/fields_nhat.wgsl`, `lib/noise.wgsl`, `lib/lattice.wgsl`, `lib/physics_sites.wgsl`.
- `crates/render/src/colour/scalar_field.rs` — the ScalarField node variants and their codegen.
- `crates/render/src/codegen/hoist.rs` — the mass-dimension detection that hoists physics sites to uniforms.
- Doc change: `docs/design/principia_dd_integrator.md` §3.7 — Euler's quintic in the mass ratios transcribed with its citation, and the map of its roots through the shape map (R-126), with the "Removed lines" note; physics-reviewed and confirmed at the M7 gate.

## Acceptance tests
- `cargo test -p render scalar_field_sources` — each source and ramp/compaction kind is constructible and compiles; gradient_magnitude of a constant map is 0; topk_margin equals d_1 − d_2 on fixed sites (REQ-COL-015).
- `cargo test -p render physics_sites` — equal masses: b̂01 = (−1,0,0), b̂12 = (½,√3/2,0), b̂20 = (½,−√3/2,0), ê = −b̂, l̂± = (0,0,±1); unequal masses: collision landmarks per the mass-weighted map and Euler landmarks at the mapped roots of Euler's quintic, against the cited source's values; hoisting toggles with mass-dimension involvement and never produces a baked texture (REQ-COL-021).

## Notes
- RQ-92 ruled: R-124 — R-50 (mass-weighted landmarks) was applied to the docs in step 7; this task implements it.
- RQ-106 ruled: R-126 — the Euler landmarks are the Euler central configurations; dd_integrator §3.7 says the quintic is transcribed with citation by the task that builds the landmarks, which is this one. Physics-reviewed, confirmed at the gate.
