# TASK-M1-11 — The kernel bring-up mode and the live unwrapped phase

- **Milestone:** M1
- **Closes:** REQ-TOOL-013, REQ-TOOL-015, REQ-INT-001
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

## Deliverables
- `crates/kernel/src/bringup.rs`: the bring-up variant (type-selected, monomorphised; f32 SPIR-V and f64 native) writing the pattern into existing `SimState` slots; no new buffer.
- `crates/kernel/src/shape.rs`: `shape(r, m)` → n and `theta_step(theta, n_prev, n)` (principal-value delta).
- `crates/engine`: the sim key includes the kernel variant; dispatch of the bring-up variant over the synthetic flat layout.
- Tests: `crates/kernel/tests/bringup.rs`, `crates/kernel/tests/theta.rs`.

## Acceptance tests
- `cargo test -p kernel debug_variants` — the kernel's debug variants are exactly the bring-up variant; dispatch-flag bits 6–7 stay reserved (R-41); the bring-up output decodes via the normal unpack path (REQ-TOOL-013).
- `cargo test -p engine bringup_pattern` — enabling the bring-up variant writes the known pattern into the payload (read back on the CPU through the generated unpack); it changes the sim key (REQ-TOOL-015).
- `cargo test -p kernel theta_unwrap` — dd_integrator test 8's accumulator half: on a synthetic circulating shape path θ̃ has no 2π jumps, `orbit_count` matches a hand count, and `retrograde` matches the winding sense; the real-orbit form (retrograde vs the L_z sign on a circulating bounded orbit) re-runs when the integrator lands in M3 (REQ-INT-001).

## Notes
- The known pattern is not fixed by the corpus — Appendix A says "e.g. `ctx`-derived UV or a fixed ramp" (milestone Gaps). The task needs it named before the golden and the readback assertion can be written.
- REQ-INT-001's verify is a circulating bounded orbit with the L_z sign; no integrator exists at M1, so the orbit form can't run here (milestone Gaps).
- The bring-up readback is shown able to fail: a pattern written one slot off must fail `bringup_pattern` (PIT-9).
