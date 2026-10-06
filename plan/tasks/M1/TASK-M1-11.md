# TASK-M1-11 — The kernel bring-up mode and the live unwrapped phase

- **Milestone:** M1
- **Closes:** REQ-TOOL-013, REQ-TOOL-015, REQ-INT-001, REQ-TOOL-123, REQ-INT-086
- **Depends on:** TASK-M1-01, TASK-M1-06
- **Needs (earlier milestones):** REQ-PAY-008, REQ-PAY-017, REQ-GEN-004, REQ-SYS-004
- **Reviewers:** code, qa, physics
- **Pitfalls:** PIT-9, PIT-3
- **Size:** ~600 lines

## Goal
Debug-tooling step 0c, first half. The kernel keeps exactly one debug mode — colour_composition Appendix A's bring-up mode — as a baked, pre-built kernel variant (not a dispatch-flag bit) that skips integration and writes a known pattern into the existing payload slots, is part of the sim key, and whose output reads back through the normal unpack path. Beside it, the shared kernel source gains the live shape readout (Montgomery map, `n` from mass-weighted Jacobi vectors) and the unwrapped-phase accumulator θ̃ (principal-value delta of `atan2(n_v, n_u)` per step), with `orbit_count` and `retrograde` derived at read.

## References
- `docs/contracts/principia_render_contract.md` § "Cross-check views (the seams)"
- `docs/contracts/principia_lowering_contract.md` § "Compute side"
- `decisions.md` § "R-41 — `DEBUG_MODE` uses the baked variants, not flag bits *(RS-1 (b))*"
- `docs/design/principia_debug_tooling_plan.md` § "A. Kernel debug dispatch modes (skip integration; reuse payload slots as scratch)"
- `decisions.md` § "R-75 — The kernel keeps one debug mode *(closes RQ-26)*"
- `docs/design/principia_colour_composition.md` § "Appendix A — kernel-side bring-up mode"
- `docs/design/principia_dd_integrator.md` § "3.7 The shape readout and winding (live, per macro-step — lockstep, ratified)"
- `docs/design/principia_dd_simstate_payload.md` § "5. Derived — computed at read, NOT stored"
- `docs/design/principia_dd_generation_root.md` § "3.1 `sample_descriptor` (u32)"
- `docs/contracts/principia_lowering_contract.md` § "Part 3 — The baking rules (compile-time vs uniform)"
- `docs/contracts/principia_render_contract.md` § "Part 3 — Cache tiers and the recompute rule"
- `decisions.md` § "R-72 — A missing definition is written by the task that needs it *(closes RQ-46 to RQ-55, definitions)*"
- `decisions.md` § "R-113 — The placement fixes are accepted as written *(closes RQ-93 to RQ-100)*"
- `decisions.md` § "R-389 — `θ̃` starts at 0, and below a pole radius `r_pole` it holds with a frozen reference, adding the wrapped exit-minus-entry longitude on exit *(closes RQ-223)*"
- `decisions.md` § "R-392 — An IC that starts inside `θ̃`'s pole radius adds no delta at its first exit; `θ̃` counts from the exit longitude *(closes RQ-225)*"
- `decisions.md` § "R-71 — A missing value becomes a calibration requirement *(closes RQ-46 to RQ-55, values)*"
- `docs/design/principia_chart_reference.md` § "0.2 Configuration — hyperspherical mass-weighted Jacobi"
- `docs/design/principia_chart_reference.md` § "3.1 Forward map (already implemented as `shape_vec`)"
- `decisions.md` § "R-14 — One shape-sphere convention, the IC Inspector's *(closes RQ-12, corrects R-12's premise)*"
- `docs/contracts/principia_gui_state_contract.md` § "2. The editable state is the entire coupling surface"
- `docs/contracts/principia_caching_contract.md` § "Part 1 — Two-level keying: identity vs validity"
- `docs/contracts/principia_caching_contract.md` § "Part 2 — What invalidates what (the dependency graph)"

## Deliverables
- `crates/kernel/src/bringup.rs`: the bring-up variant (type-selected, monomorphised; f32 SPIR-V and f64 native) writing the pattern into existing `SimState` slots; no new buffer.
- `crates/kernel/src/shape.rs`: `shape(r, m)` → n and `theta_step(theta, n_prev, n)` (principal-value delta).
- `crates/engine`: the sim key includes the kernel variant (physics or the bring-up mode), a `SimConfig` field beside the integrator occupant (gui_state §2, render contract Part 3, caching contract Parts 1–2; RQ-221); dispatch of the bring-up variant over the synthetic flat layout.
- `crates/validation/tests/bringup.rs`: `bringup_pattern_spirv`, the f32 SPIR-V variant's GPU readback; its binary joins `.config/nextest.toml`'s `gpu-kernel` filter and leaves the `ci` filter, so CI's `gpu-kernel` job, which builds the kernel, runs it (RQ-222).
- Tests: `crates/kernel/tests/bringup.rs`, `crates/kernel/tests/theta.rs`.

## Acceptance tests
- `cargo test -p kernel debug_variants` — the kernel's debug variants are exactly the bring-up variant, a baked variant selected by type that reads no `DEBUG_MODE` flag bit (R-41); the bring-up output decodes via the normal unpack path (REQ-TOOL-013). Bits 6–7's reservation is checked on `QuadRequest.flags` by `quad_request_flags` (REQ-PAY-064, TASK-M5-04), where the flags word exists (RQ-220).
- `cargo test -p engine bringup_pattern` — enabling the f64 native bring-up variant writes the known pattern into the payload (read back on the CPU through the generated unpack); it changes the sim key (REQ-TOOL-015).
- `cargo xtask build-kernel && cargo test -p validation bringup_pattern_spirv` — the f32 SPIR-V bring-up variant, dispatched on the GPU, writes the same pattern, read back through the generated unpack; in CI it runs in the `gpu-kernel` job (REQ-TOOL-015).
- `cargo test -p kernel theta_unwrap` — on a synthetic n(t) path circulating the w axis θ̃ accumulates with no 2π jumps, `orbit_count` matches the path's turn count, and `retrograde` matches its direction (REQ-INT-001; dd test 8 on a real orbit is REQ-INT-082, TASK-M3-11).
- `cargo test -p kernel theta_unwrap` — R-389's start and pole rule: θ̃ is 0 at the start whatever n(0)'s longitude; a synthetic path passing through the pole disc √(n_u² + n_v²) < r_pole adds no delta inside and exactly wrap(exit − stored) into (−π, π] on exit, the stored longitude the last one outside; a step whose longitude difference is exactly ±π, and a pole passage whose exit-minus-stored difference is exactly ±π, each add +π (REQ-INT-001).
- `cargo test -p kernel theta_unwrap` — R-392's IC inside the pole disc: a synthetic path that starts inside √(n_u² + n_v²) < r_pole leaves θ̃ unchanged, at 0, at its first exit (no delta added, there being no stored longitude), and accumulates from the exit longitude after it; a later passage through the disc adds wrap(exit − stored) as above (REQ-INT-001).
- Proposal: `r_pole` with its evidence — the longitude's round-off error near the poles at f32 and f64, and that the hold fires on the pole passage and never on the circulating paths; the human confirms it at the M1 gate (REQ-INT-086).
- Definition: the bring-up pattern and its payload slots written into colour_composition Appendix A and approved by the physics reviewer (REQ-TOOL-123).

## Notes
- The known pattern is not fixed by the corpus — Appendix A says "e.g. `ctx`-derived UV or a fixed ramp" (milestone Gaps). The task needs it named before the golden and the readback assertion can be written.
- The bring-up readback is shown able to fail: a pattern written one slot off must fail `bringup_pattern` (PIT-9).
- RQ-94 ruled: R-113 — REQ-INT-001 keeps the accumulator on a synthetic path at M1; dd test 8 on a real circulating bounded orbit is a new M3 requirement (REQ-INT-082, TASK-M3-11).
- Closes, for gaps the corpus leaves open: REQ-TOOL-123 (R-72 definition) (classification accepted by R-132).
- RQ-220, RQ-221, RQ-222 and RQ-224 decided per R-369 (`docs/archive/review_queue/M0.md`): `debug_variants` checks what exists at M1; the kernel variant is on the sim key in all three lists; the SPIR-V readback is a `validation` test in the `gpu-kernel` job; the shape map's references are cited.
- RQ-223 ruled: R-389 — θ̃(0) = 0, and θ̃ holds below `r_pole` with a frozen reference, the exact-π case adding +π (dd_integrator §3.7); `r_pole` is REQ-INT-086 (R-71, the M1 gate).
- RQ-225 ruled: R-392 — an IC that starts inside the disc has no stored longitude, so its first exit adds no delta and θ̃ counts from the exit longitude (dd_integrator §3.7).
