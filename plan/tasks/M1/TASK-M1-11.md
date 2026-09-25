# TASK-M1-11 — The kernel bring-up mode and the live unwrapped phase

- **Milestone:** M1
- **Closes:** REQ-TOOL-013, REQ-TOOL-015, REQ-INT-001, REQ-TOOL-123
- **Depends on:** TASK-M1-01, TASK-M1-06
- **Needs (earlier milestones):** REQ-PAY-008, REQ-PAY-017, REQ-GEN-004, REQ-SYS-004
- **Reviewers:** code, qa, physics
- **Pitfalls:** PIT-9, PIT-3
- **Size:** ~380 lines

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

## Deliverables
- `crates/kernel/src/bringup.rs`: the bring-up variant (type-selected, monomorphised; f32 SPIR-V and f64 native) writing the pattern into existing `SimState` slots; no new buffer.
- `crates/kernel/src/shape.rs`: `shape(r, m)` → n and `theta_step(theta, n_prev, n)` (principal-value delta).
- `crates/engine`: the sim key includes the kernel variant; dispatch of the bring-up variant over the synthetic flat layout.
- Tests: `crates/kernel/tests/bringup.rs`, `crates/kernel/tests/theta.rs`.

## Acceptance tests
- `cargo test -p kernel debug_variants` — the kernel's debug variants are exactly the bring-up variant; dispatch-flag bits 6–7 stay reserved (R-41); the bring-up output decodes via the normal unpack path (REQ-TOOL-013).
- `cargo test -p engine bringup_pattern` — enabling the bring-up variant writes the known pattern into the payload (read back on the CPU through the generated unpack); it changes the sim key (REQ-TOOL-015).
- `cargo test -p kernel theta_unwrap` — on a synthetic n(t) path circulating the w axis θ̃ accumulates with no 2π jumps, `orbit_count` matches the path's turn count, and `retrograde` matches its direction (REQ-INT-001; dd test 8 on a real orbit is REQ-INT-082, TASK-M3-11).
- Definition: the bring-up pattern and its payload slots written into colour_composition Appendix A and approved by the physics reviewer (REQ-TOOL-123).

## Notes
- The known pattern is not fixed by the corpus — Appendix A says "e.g. `ctx`-derived UV or a fixed ramp" (milestone Gaps). The task needs it named before the golden and the readback assertion can be written.
- The bring-up readback is shown able to fail: a pattern written one slot off must fail `bringup_pattern` (PIT-9).
- RQ-94 ruled: R-113 — REQ-INT-001 keeps the accumulator on a synthetic path at M1; dd test 8 on a real circulating bounded orbit is a new M3 requirement (REQ-INT-082, TASK-M3-11).
- Closes, for gaps the corpus leaves open: REQ-TOOL-123 (R-72 definition) (classification accepted by R-132).
