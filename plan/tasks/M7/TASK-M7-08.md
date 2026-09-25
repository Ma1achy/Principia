# TASK-M7-08 — Categorical palettes and the categorical filter

- **Milestone:** M7
- **Closes:** REQ-COL-018, REQ-COL-040, REQ-COL-051, REQ-COL-019
- **Depends on:** TASK-M7-02, TASK-M7-05, TASK-M1-10
- **Needs (earlier milestones):** REQ-COL-002, REQ-COL-004, REQ-TOOL-009, REQ-COL-053
- **Reviewers:** code, qa
- **Pitfalls:** PIT-3
- **Size:** ~300 lines

## Goal
Categorical fields other than the outcome state map through a generated palette swatch-set — an Okabe–Ito cycle for n ≤ 8 classes, golden-angle hues θ_i = 2π·frac(i·φ_g), φ_g = (√5 − 1)/2 beyond — while `state` resolves to colour_composition §1.4's nine-class canonical palette (R-77). Consecutive golden-angle indices exceed a minimum OKLab hue separation, proposed with its evidence up to the Fibonacci-lattice counts (96). Every categorical mode admits a categorical filter (show class ∈ {…}, mute the rest); there is no separate escaper field or escaper render mode.

## References
- `docs/design/principia_colour_composition.md` § "1.4 Categorical colour-assignment — the outcome-state default palette"
- `docs/design/principia_dd_colouring.md` § "3.7 Categorical colour, and how mixed pixels resolve (colour-per-sample → SSAA)"
- `decisions.md` § "R-77 — Replace-L, and the state palette *(closes RQ-28)*"
- `docs/design/principia_dd_colouring.md` § "5. Unit tests"
- `decisions.md` § "R-71 — A missing value becomes a calibration requirement *(closes RQ-46 to RQ-55, values)*"

## Deliverables
- `crates/render/src/colour/palette.rs` and `shaders/wgsl/lib/palette.wgsl` — the generators and the state-palette resolution.
- The categorical filter as a parameter of the categorical ramp (a class mask uniform).
- The R-71 proposal in the PR: the achieved consecutive-index separations for n ≤ 96.

## Acceptance tests
- `cargo test -p render categorical_palette` — the palette generator for n ≤ 8 returns OI colours; for larger n golden-angle hues θ_i = 2π·frac(i·φ_g); the state field resolves to the §1.4 palette, not a generated index palette (REQ-COL-018).
- `cargo test -p render golden_angle_adjacency` — dd_colouring unit test 9 for n up to 96, against the minimum separation REQ-COL-051 (calibrated) (REQ-COL-040).
- Review (code, qa): the proposal lists the achieved consecutive-index OKLab hue separations for n up to 96 and the minimum chosen below them; confirmed by the human at the M7 gate (REQ-COL-051).
- `cargo test -p render categorical_filter` — filter the state map to the escape classes; non-escape pixels are muted; no `escaper` field or render mode exists in the ledger or the registry (REQ-COL-019).

## Notes
- Calibration (R-71): REQ-COL-051.
- Gap: how a muted class is drawn (grey, dimmed, desaturated) is not given (see the milestone report).
