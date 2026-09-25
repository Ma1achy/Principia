# TASK-M7-10 — Combinators and the physics overlay (blob blend)

- **Milestone:** M7
- **Closes:** REQ-COL-017, REQ-COL-036
- **Depends on:** TASK-M7-09
- **Needs (earlier milestones):** REQ-VAL-023
- **Reviewers:** code, qa, physics
- **Pitfalls:** none
- **Size:** ~350 lines

## Goal
The combinators mix_const, mix_field, bandmask and site_overlay as `vec3(+ctx) → vec3` operations, and the physics overlay as exactly `site_overlay(base, SiteBlend{physics(m), vmf(κ), per-site colours}, s)`, blending c_out = c_base + Σ_j w_j(c_j − c_base) with w_j = s·4·max(0, exp(κ_j(n̂·p̂_j − 1)) + 0.01), κ = 11 for BC and 9 for Euler/Lagrange.

## References
- `docs/design/principia_colour_composition.md` § "1.3 Combinators  →  `vec3`"
- `docs/design/principia_dd_colouring.md` § "3.4 Physics overlay (blob blend) and the house encoding (stability × hue)"
- `docs/design/principia_colour_composition.md` § "7.1 The complete map list (R-16)"

## Deliverables
- `crates/render/shaders/wgsl/frag/post/mix_const.wgsl`, `mix_field.wgsl`, `bandmask.wgsl`, `site_overlay.wgsl`; `lib/blob_blend.wgsl`.
- Combinator node variants and codegen in the occupant tree; the physics-overlay expansion helper used by the preset library.

## Acceptance tests
- `cargo test -p render combinators` — each combinator compiles; the physics overlay preset expands to site_overlay(base, SiteBlend{physics(m), vmf, per-site}, s) (REQ-COL-017).
- `cargo test -p render physics_overlay` — dd_colouring unit test 5: blob maxima exactly at b̂/ê/l̂ (equal and unequal masses); s = 0 is the identity; κ = 11 BC, 9 Euler/Lagrange (REQ-COL-036).

## Notes
- Gap: as written, max(0, exp(…) + 0.01) never clips (the argument is always > 0.01), so every blob adds a floor weight of 0.04·s everywhere; the task implements the formula as written and the report asks whether − 0.01 was meant.
