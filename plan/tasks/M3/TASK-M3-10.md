# TASK-M3-10 — The monotone latches and their f16 packing

- **Milestone:** M3
- **Closes:** REQ-PAY-041, REQ-PAY-044, REQ-PAY-045, REQ-PAY-050, REQ-INT-045
- **Depends on:** TASK-M3-09
- **Needs (earlier milestones):** REQ-PAY-011, REQ-PAY-012, REQ-GEN-005
- **Reviewers:** code, qa, physics
- **Pitfalls:** PIT-9
- **Size:** ~250 lines

## Goal
`d_min = min_t |separation|`, `dE_max = max_t |ΔE|` and `dLz_max = max_t |ΔL_z|` are absolute monotone latches held in local f32 during the march and packed to f16 only when persistent state is written (semantics (a)): clamped to ±65504 first, 0.0 for failed states, `d_min` in the high half of `packed_a`, `dE_max`/`dLz_max` in `packed_b`. Every threshold and suspect decision reads the live f32, never an unpacked f16.

## References
- `docs/contracts/principia_render_contract.md` § "Unpack layer (generated, one accessor per named field)"
- `docs/design/principia_dd_simstate_payload.md` § "Why closure is here, at this width, unconditionally"
- `docs/design/principia_dd_simstate_payload.md` § "8. Build-time settles (measure / specify once running)"
- `docs/design/principia_dd_simstate_payload.md` § "`sample_descriptor` (low 16 bits of `packed_a`) — 10 used, rest reserved"
- `docs/design/principia_dd_simstate_payload.md` § "5. Derived — computed at read, NOT stored"
- `docs/design/principia_dd_generation_root.md` § "3.1 `sample_descriptor` (u32)"

## Deliverables
- `crates/kernel/src/driver/latch.rs` — the local-f32 latches, persisted at write-out via the generated pack functions with the clamp and failed-state rule.
- Resume path: the latches reload from the payload across persist boundaries.

## Acceptance tests
- `cargo test -p kernel f16_pack_clamp` — pack values beyond ±65504 and failed-state samples; unpack yields finite clamped values and 0.0 respectively (REQ-PAY-041).
- `cargo test -p kernel latch_across_persist` — resume a sample across many persist boundaries; the packed latch equals the f16 of a single f32 latch over the whole run (REQ-PAY-044).
- `cargo test -p kernel f16_clamp_fuzz` — property test with inputs incl. ±1e6, ±Inf: packed halves finite and clamped; failed-state samples unpack to exactly 0.0 (REQ-PAY-045).
- `cargo test -p kernel latches_vs_offline` — property test: latches equal offline max/min over a fixture trajectory (within f16 packing) (REQ-PAY-050).
- Review (code): every control-flow comparison on drift/d_min reads the live f32 value; grep the kernel for `unpack2x16float` feeding a branch — none (REQ-INT-045).

## Notes
- None.
