# TASK-M6-07 — The linearised decoder: x₀ and J_D in f64 on the CPU, x₀ + J_D·δ on the GPU

- **Milestone:** M6
- **Closes:** REQ-DEC-034, REQ-DEC-035, REQ-DEC-036, REQ-DEC-042, REQ-GEN-018
- **Depends on:** TASK-M5-04
- **Needs (earlier milestones):** REQ-DEC-031, REQ-DEC-032, REQ-SCHED-024, REQ-SYS-015, REQ-CHART-003, REQ-CHART-032, REQ-CHART-037
- **Reviewers:** code, qa, physics
- **Pitfalls:** PIT-9, PIT-10
- **Size:** ~450 lines

## Goal
Deep quads get the linearised decode (deep_zoom §2): per quad the CPU evaluates `x₀ = D(c_u, c_v)` and `J_D = ∂(physical)/∂(u,v)` by central differences in f64 over the whole chart→IC composite (Φ → decode → canonicalise), chart-agnostic with no per-axis-kind special-casing; the GPU evaluates `x = x₀ + J_D·δ` in f32 and never runs the nonlinear pipeline. Global precision stays in f64 on the CPU, the GPU does local f32 arithmetic, and the inspector stays an f64 witness. `|det J_D|` is checked against the link registry's log-det Jacobians. The task proposes decoder test 11's slope tolerance (R-71).

## References
- `docs/design/principia_dd_decoder.md` § "2. Consolidated contract (every clause that binds this component)"
- `docs/design/principia_dd_decoder.md` § "4. Seams (its side of each — the integration-test list)"
- `docs/design/principia_dd_decoder.md` § "5. Unit tests"
- `docs/design/principia_chart_reference.md` § "3.3 The chart map"
- `docs/design/principia_chart_reference.md` § "5.1 One trait, one dispatch"
- `docs/design/principia_deep_zoom.md` § "Couplings to the contract"
- `decisions.md` § "R-71 — A missing value becomes a calibration requirement *(closes RQ-46 to RQ-55, values)*"
- `docs/design/principia_deep_zoom.md` § "The precision split (the CPU/GPU seam, decode side)"
- `docs/design/principia_deep_zoom.md` § "2. Linearised decoder — IC precision"
- `docs/design/principia_dd_encode.md` § "4. Seams (obligations → integration tests)"

## Deliverables
- `crates/engine/src/deep/linearise.rs`: f64 `x₀`, `J_D` by central differences of the composite (reusing the shared decoder's f64 instantiation), `|det J_D|` computed once per quad.
- `crates/kernel/src/decode/linear.rs`: the f32 linear path `x₀ + J_D·δ` in the shared source, producing the same `(m, r, p)` input type as the full decode.
- `crates/validation/src/decoder_test11.rs` + `cargo xtask gate linear-decode` (test 11, O(h²) slope across depths, affine and exponential-map shape-sphere charts, depth ≥ 50 distinguishability) and `cargo xtask gate det-jd-links`.

## Acceptance tests
- `cargo xtask gate linear-decode` (decoder test 11; slope tolerance: REQ-DEC-042 (calibrated)) — decoder test 11: at quad centre x₀ + J_D·0 equals the full decode exactly; at half-width the error vs full decode shrinks ∝ h² across depths; identical behaviour for an affine chart and the exponential-map shape-sphere chart (REQ-DEC-034).
- Review checklist (physics, code) — CPU quadtree types are f64; GPU uniforms are f32 c, h, x₀, J_D; inspector integrates in f64 (REQ-DEC-035).
- `cargo xtask gate linear-decode --depth-sweep` — at the switchover depth, linear vs full decode agree to O(h²); the linear path distinguishes adjacent samples to depth ≥ 50 (REQ-DEC-036).
- `cargo xtask gate linear-decode --fit-slope` — the proposal fits the log-log slope across depths for an affine and a curve chart and states the tolerance about 2; recorded in decisions.md; proposal with its evidence in the PR, reviewer-checked; the human confirms the value at the M6 gate and it is recorded in `decisions.md` (REQ-DEC-042).
- `cargo xtask gate det-jd-links` — compare numeric |det J_D| against the sum of registry log-det Jacobians at sampled points, per link (REQ-GEN-018).

## Notes
- Calibration proposed: REQ-DEC-042 (slope tolerance about 2).
- PIT-10: the O(h²) agreement holds at fixed inputs; state the domain (decode only, not trajectory outcomes).
- Waits on RQ-99 (`REVIEW_QUEUE.md`): M5 requirements that need M6, M7 or M8.
