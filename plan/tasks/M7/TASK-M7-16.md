# TASK-M7-16 — The bake tier: the equirect texture for pure-f(n̂) occupants, and the sphere widget

- **Milestone:** M7
- **Closes:** REQ-RENDER-058, REQ-RENDER-059, REQ-RENDER-067, REQ-SCHED-072, REQ-RENDER-065, REQ-RENDER-070, REQ-RENDER-080
- **Depends on:** TASK-M7-06, TASK-M7-07, TASK-M7-10, TASK-M7-15, TASK-M5-24, TASK-M5-30
- **Needs (earlier milestones):** REQ-SYS-036, REQ-SCHED-043
- **Reviewers:** code, qa, physics, perf
- **Pitfalls:** none
- **Size:** ~480 lines

## Goal
Colour occupants that are pure f(n̂) bake into an equirect texture keyed by the colour-node source and its uniforms, rebaked debounced (~120 ms) and never synchronously inside the frame callback; the GUI preview canvas is the uploaded texture itself. The bake cache is keyed only on colour occupant + params, stays outside the quad cache and its eviction, and is chart- and IC-independent. Occupants reading dynamical fields skip the bake and evaluate per fragment; the physics overlay is one of them — a per-fragment occupant, never baked (R-121). Baked and directly-evaluated colour agree within texture quantisation. Render config never touches sim buffers. The sphere widget projects with s_x = (p_x−c_x)/R, s_y = −(p_y−c_y)/R, s_z = √max(0, 1−s_x²−s_y²), R = W/2 − 4.

## References
- `docs/contracts/principia_render_contract.md` § "Part 3 — Cache tiers and the recompute rule"
- `docs/design/principia_dd_colouring.md` § "2. Consolidated contract"
- `decisions.md` § "R-64 — The stain editor is a free, typed node graph *(closes RQ-20)*"
- `docs/design/principia_dd_colouring.md` § "5. Unit tests"
- `docs/contracts/principia_caching_contract.md` § "Part 2 — What invalidates what (the dependency graph)"
- `docs/contracts/principia_caching_contract.md` § "Part 6 — The responsiveness invariant: the main thread never waits"
- `docs/design/principia_dd_colouring.md` § "3.3 Sphere sampling conventions"
- `docs/design/principia_dd_colouring.md` § "6. Deferred / flagged"
- `docs/design/principia_systems_architecture.md` § "3. The membrane — the deployment view (demoted, not diminished)"
- `docs/design/principia_colour_composition.md` § "2. Site-set kinds"
- `decisions.md` § "R-71 — A missing value becomes a calibration requirement *(closes RQ-46 to RQ-55, values)*"
- `decisions.md` § "R-121 — The physics overlay isn't baked *(closes RQ-89)*"

## Deliverables
- `crates/render/src/bake.rs` — bakeability test over the occupant tree (inputs n̂ and uniforms only), bake key, equirect rasterisation, debounced rebake as low-priority background work (REQ-SCHED-043).
- `crates/engine/src/bake_cache.rs` — the bake cache outside the quad cache; the texture upload as the membrane's bake crossing (REQ-SYS-036).
- `crates/render/src/sphere_widget.rs` — widget projection and the equirect mapping, shared by the preview.
- `fixtures/golden/bake-equivalence/` and the `bake-equivalence` golden suite.

## Acceptance tests
- Review (code, perf): bake key = hash(colour source, colour uniforms); the preview widget samples the same GPU texture the render uses; rebake debounced ~120 ms and off the frame callback; the physics overlay is not on the bake path (R-121) (REQ-RENDER-058).
- Review (code): the bake path is only enabled for occupants whose inputs are n̂ and uniforms (REQ-RENDER-059).
- `cargo xtask golden bake-equivalence` — dd_colouring unit test 11: texture-sampled vs direct evaluation over a sphere lattice for every pure-f(n̂) occupant, max difference ≤ one texel quantisation step (REQ-RENDER-067).
- `cargo test -p engine bake_cache` — a chart switch reuses the bake; a param change triggers one debounced rebake; quad-cache eviction never touches it (REQ-SCHED-072).
- `cargo test -p render sphere_projection` — the widget projection formulas and R = W/2 − 4; the equirect v for n_z = 0.5 per REQ-RENDER-065's verify, subject to the Gap in Notes (REQ-RENDER-065).
- `cargo test -p engine render_config_membrane` — render-config updates (slot uniforms, playhead t) issue no writes to sim buffers; the baked texture is identical across charts and ICs; the physics overlay is evaluated per fragment and never enters the bake (R-121) (REQ-RENDER-070).
- Proposal: the equirect bake texture's resolution and texel format, with evidence (bake-vs-direct difference over a sphere lattice, bake time); the human confirms it at the M7 gate (REQ-RENDER-080).

## Notes
- Gap: REQ-RENDER-065 states the equirect mapping as (φ, n_z) ∈ [−π, π]×[−1, 1] with v linear in n_z, but dd_colouring §3.3 (R-14) gives (θ, φ) ∈ [0, 2π]×[0, π] with φ the polar angle on the vertical axis. The source wins over the requirement; the requirement needs correcting before this acceptance line is final.
- R-121 settles the physics-overlay gap: it isn't baked; the mass-constant hoist is an allowed optimisation, not a bake tier (render_contract Parts 3 and 4 conformed in step 7).
- Gap: the equirect texture's resolution and format (which set "one texel quantisation step") are not given.
- RQ-89 ruled: R-121 (above).
- Closes, for gaps the corpus leaves open: REQ-RENDER-080 (R-71 calibration) (classification accepted by R-132).
