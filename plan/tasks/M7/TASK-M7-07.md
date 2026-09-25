# TASK-M7-07 — The seamless LUT sphere and analytic Cubehelix

- **Milestone:** M7
- **Closes:** REQ-COL-035, REQ-COL-050, REQ-COL-041
- **Depends on:** TASK-M7-06
- **Needs (earlier milestones):** REQ-RENDER-020
- **Reviewers:** code, qa, physics
- **Pitfalls:** PIT-3
- **Size:** ~350 lines

## Goal
The seamless LUT sphere: N_e = 16 LUT samples placed as equatorial poles (cos 2πi/N_e, sin 2πi/N_e, 0), the LUT endpoints at the north and south poles, blended with the vMF law in RGB — for Viridis, Cividis, Plasma, Magma, Inferno, Twilight (cyclic, closing exactly at the wrap), Cool-warm, Principia and Cubehelix. Cubehelix is generated analytically (φ = 2π(s/3 − λt), a = h·t(1−t)/2, s = 0.5, λ = 1.5, h = 1, dd_colouring §3.8's R, G, B forms). The blend tolerance of dd_colouring unit test 4 is proposed with its evidence (R-71).

## References
- `docs/design/principia_dd_colouring.md` § "3.2 The vMF engine"
- `docs/design/principia_colour_composition.md` § "7.1 The complete map list (R-16)"
- `docs/design/principia_dd_colouring.md` § "5. Unit tests"
- `decisions.md` § "R-71 — A missing value becomes a calibration requirement *(closes RQ-46 to RQ-55, values)*"
- `docs/design/principia_dd_colouring.md` § "3.8 Palettes and CVD"

## Deliverables
- `crates/render/shaders/wgsl/lib/lut.wgsl` (`lut_sample` and the LUT tables) and `lib/cubehelix.wgsl`.
- `crates/render/src/colour/lut_sphere.rs` — builds `SiteBlend{ring(16) + 2 poles, vmf(κ), colours = lut(name, i/N), rgb}`.
- `fixtures/gates/lut-sphere-blend/` and the `lut-sphere-blend` gate.
- The R-71 proposal in the PR.

## Acceptance tests
- `cargo test -p render lut_sphere` — dd_colouring unit test 4: an equator sweep reproduces each 1-D LUT within tolerance REQ-COL-050 (calibrated); Twilight closes exactly at the wrap (REQ-COL-035).
- `cargo xtask gate lut-sphere-blend` — the proposal shows the measured maximum deviation of the equator sweep from each shipped LUT at N_e = 16 and the margin the tolerance leaves; confirmed by the human at the M7 gate (REQ-COL-050).
- `cargo test -p render cubehelix` — sampled Cubehelix matches the formula; lightness monotone increasing (REQ-COL-041).

## Notes
- Calibration (R-71): REQ-COL-050.
- Gap: the Principia palette is given only as "indigo → teal → gold" and Cool-warm only as "diverging" (§7.1); their stops are not in the corpus (see the milestone report).
