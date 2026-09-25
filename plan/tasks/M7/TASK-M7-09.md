# TASK-M7-09 — ScalarField sources, derived operators and the physics site generators

- **Milestone:** M7
- **Closes:** REQ-COL-015, REQ-COL-021
- **Depends on:** TASK-M7-05, TASK-M7-06, TASK-M1-15, TASK-M2-25
- **Needs (earlier milestones):** REQ-COL-003, REQ-COL-005, REQ-COL-006, REQ-RENDER-008, REQ-VAL-023
- **Reviewers:** code, qa, physics
- **Pitfalls:** none
- **Size:** ~500 lines

## Goal
FieldRamp's ScalarField sources, each returning (value, valid) through `ctx`: payload fields (through the generated accessors), geometry-of-n̂ fields (n_z, ‖n̂‖, azimuth/polar in R-14's names, stability, real spherical harmonics ℓ ≤ 3, the Turing wave-triple, 4-octave 3-D value noise), ctx lanes, the lattice classifiers `bands` uses (checker, latitude/longitude stripes, Truchet on (θ, φ)), and the derived operators gradient_magnitude (three evaluations at ±ε) and topk_margin = d_1 − d_2. Alongside, the physics site generators BC(m), Euler(m), Lagrange(m): computed from the decoded IC masses per pixel on the mass-weighted shape sphere (R-14, R-50), hoisted to uniforms only when no basis axis or active tilt touches a mass dimension.

## References
- `docs/design/principia_colour_composition.md` § "1.2 Family B — field-ramp  →  `vec3` or `f32`"
- `docs/design/principia_colour_composition.md` § "2. Site-set kinds"
- `docs/design/principia_dd_colouring.md` § "3.4 Physics overlay (blob blend) and the house encoding (stability × hue)"
- `decisions.md` § "R-14 — One shape-sphere convention, the IC Inspector's *(closes RQ-12, corrects R-12's premise)*"
- `decisions.md` § "R-50 — The shape-sphere collision landmarks are mass-weighted *(CO-1 (a))*"
- `docs/design/principia_dd_integrator.md` § "3.7 The shape readout and winding (live, per macro-step — lockstep, ratified)"

## Deliverables
- `crates/render/shaders/wgsl/lib/fields_nhat.wgsl`, `lib/noise.wgsl`, `lib/lattice.wgsl`, `lib/physics_sites.wgsl`.
- `crates/render/src/colour/scalar_field.rs` — the ScalarField node variants and their codegen.
- `crates/render/src/codegen/hoist.rs` — the mass-dimension detection that hoists physics sites to uniforms.
- R-50 applied before the physics-overlay occupant, as the ruling says: dd_integrator §3.7 and test 9, dd_colouring :90, colour_composition :197–209 and :481, trajectory_viewing :78 and :84, and the two HTML references (`principia_gui_mock.html`, `principia_colour_presets.html`) conformed to mass-weighted landmarks, with the "Removed lines" note.

## Acceptance tests
- `cargo test -p render scalar_field_sources` — each source and ramp/compaction kind is constructible and compiles; gradient_magnitude of a constant map is 0; topk_margin equals d_1 − d_2 on fixed sites (REQ-COL-015).
- `cargo test -p render physics_sites` — equal masses: b̂01 = (−1,0,0), b̂12 = (½,√3/2,0), b̂20 = (½,−√3/2,0), ê = −b̂, l̂± = (0,0,±1); unequal masses move the landmarks per the mass-weighted map; hoisting toggles with mass-dimension involvement (REQ-COL-021).

## Notes
- R-50 is "recorded; applied before the physics-overlay occupant": this task applies it (doc edits under the ruling). dd_integrator §3.7 still reads "audit decision B18, still open".
- Gap: the Euler landmark for unequal masses — the corpus gives ê = −b̂ for equal masses only and does not say whether unequal-mass Euler points are the antipodes of b̂ or the collinear central configurations (see the milestone report).
- Waits on RQ-92 (`REVIEW_QUEUE.md`): Rulings not yet applied to some passages.
- Waits on RQ-106 (`REVIEW_QUEUE.md`): The Euler landmarks for unequal masses.
