# TASK-M2-29 — The DECODE and ROUNDTRIP presets' tolerance under fragment fast-math, calibrated

- **Milestone:** M2
- **Closes:** REQ-COL-060
- **Depends on:** TASK-M2-26, TASK-M0-44
- **Needs (earlier milestones):** REQ-SYS-074
- **Reviewers:** code, qa, physics
- **Pitfalls:** PIT-3
- **Size:** ~200 lines

## Goal
The fragment shaders may compile with fast-math, and the compute shaders don't, by default (R-297). So the presets that
recompute physics in the fragment to check agreement, the DECODE agreement presets (fragment decode against the compute
kernel's values) and ROUNDTRIP's residual, compare within a stated tolerance, not bit-exactly. That tolerance is a
calibration requirement (R-71): this task measures the fragment-against-compute differences on both CI backends,
proposes the tolerance with its evidence, and puts the agreement gate and the ROUNDTRIP render on it. The human
confirms the value at the M2 gate.

## References
- `decisions.md` § "R-297 — Fast-math per shader stage: off for compute by default, an explicit and recorded opt-in; display may keep it *(amends R-84, R-116)*"
- `decisions.md` § "R-116 — The fragment decode and encode are generated from the one source *(closes RQ-84)*"
- `decisions.md` § "R-71 — A missing value becomes a calibration requirement *(closes RQ-46 to RQ-55, values)*"
- `docs/design/principia_colour_composition.md` § "6. Debug views as presets"
- `docs/contracts/principia_render_contract.md` § "Cross-check views (the seams)"
- `docs/contracts/principia_parity_contract.md` § "4. Tolerance — and the cross-backend reality"
- `docs/design/principia_debug_tooling_plan.md` § "A. Kernel debug dispatch modes (skip integration; reuse payload slots as scratch)"

## Deliverables
- A measurement in `gpu-metal` and `gpu-lavapipe`: over the M2 charts, the agreement presets' fragment-against-compute
  differences (|E(fragment-decode) − E₀| and the like) and ROUNDTRIP's residual, with the fragment compiled as the
  backend compiles it and the compute shaders with fast-math off.
- The proposed tolerance with that evidence in the PR; `cargo xtask gate decode-agreement` and the ROUNDTRIP render
  read it, marked provisional until the human confirms it (R-182).
- Negative controls for this task's tests (R-176).

## Acceptance tests
- `cargo xtask gate decode-agreement` in `gpu-metal` and `gpu-lavapipe` — the agreement presets pass within the proposed tolerance, with the per-chart measurement written to the gate report; a deliberate dispatch scramble still fails it; proposal: the tolerance with its evidence, reviewer-checked, confirmed by the human at the M2 gate and recorded in decisions.md (REQ-COL-060).
- `cargo xtask golden roundtrip-preset` in `gpu-metal` and `gpu-lavapipe` — the proposal also states, with its evidence, the tolerance ROUNDTRIP's residual is checked within under fragment fast-math; the render passes within it outside the tagged-expected regions (REQ-COL-060).

## Notes
- REQ-DEC-043's calibrated f32 decode factor stays the tolerance for the fragment decode against the f64
  `decodeOnly()` (TASK-M2-25's `decode_preset_vs_decode_only`, R-133). This task's tolerance is the fragment, compiled
  with fast-math, against the compute kernel, compiled without it; the agreement gate moves to it.
