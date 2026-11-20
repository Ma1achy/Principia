# TASK-M7-10 — Combinators and the physics overlay (blob blend)

- **Milestone:** M7
- **Closes:** REQ-COL-017, REQ-COL-036
- **Depends on:** TASK-M7-09
- **Needs (earlier milestones):** REQ-VAL-023
- **Reviewers:** code, qa, physics
- **Pitfalls:** none
- **Size:** ~350 lines

## Goal
The combinators mix_const, mix_field, bandmask and site_overlay as `vec3(+ctx) → vec3` operations, and the physics overlay as exactly `site_overlay(base, SiteBlend{physics(m), vmf(κ), per-site colours}, s)`, blending sequentially over the sites in order (BC, then Euler, then Lagrange), c ← mix(c, c_j, min(1, w_j)), with w_j = s·4·max(0, exp(κ_j(n̂·p̂_j − 1)) + 0.005), κ = 11 for BC and 9 for Euler/Lagrange — the colour explorer's blob weight and sequential clamped mix (R-122). The overlay is a per-fragment occupant, never baked (R-121).

## References
- `docs/design/principia_colour_composition.md` § "1.3 Combinators  →  `vec3`"
- `docs/design/principia_dd_colouring.md` § "3.4 Physics overlay (blob blend) and the house encoding (stability × hue)"
- `docs/design/principia_colour_composition.md` § "7.1 The complete map list (R-16)"
- `decisions.md` § "R-121 — The physics overlay isn't baked *(closes RQ-89)*"
- `decisions.md` § "R-122 — The reference HTML files are the colour oracle *(closes RQ-90 and RQ-101)*"

## Deliverables
- `crates/render/shaders/wgsl/frag/post/mix_const.wgsl`, `mix_field.wgsl`, `bandmask.wgsl`, `site_overlay.wgsl`; `lib/blob_blend.wgsl`.
- Combinator node variants and codegen in the occupant tree; the physics-overlay expansion helper used by the preset library.

## Acceptance tests
- `cargo test -p render combinators` — each combinator compiles; the physics overlay preset expands to site_overlay(base, SiteBlend{physics(m), vmf, per-site}, s) (REQ-COL-017).
- `cargo test -p render physics_overlay` — dd_colouring unit test 5: blob maxima exactly at b̂/ê/l̂ (equal and unequal masses); s = 0 is the identity; κ = 11 BC, 9 Euler/Lagrange; the overlay matches `principia_colour_explorer.html`'s physics op on fixture inputs (R-122) (REQ-COL-036).

## Notes
- RQ-89 ruled: R-121 — the physics overlay is a per-fragment occupant, not on the bake path; the mass-constant hoist is an allowed optimisation, not a bake tier.
- RQ-90 ruled: R-122 — the formulas follow the oracle (the explorer's blob weight, +0.005, and its sequential clamped mix); dd_colouring §3.4 was conformed in step 7.
