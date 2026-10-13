# TASK-M2-30 — The f32 images of π/2 and π rounded toward zero, and the link test's exact bounds (R-417)

- **Milestone:** M2
- **Closes:** REQ-GEN-034
- **Depends on:** TASK-M2-01
- **Needs (earlier milestones):** none
- **Reviewers:** code, qa, physics
- **Pitfalls:** PIT-3
- **Size:** ~80 lines, the regenerated links included

## Goal
**R-417 (RQ-270, option (b)):** the link generator emits the f32 images of π/2 and π rounded toward zero, `fl↓(π/2)`
(bits `0x3FC9_0FDA`) and `fl↓(π)` (`0x4049_0FDA`), not the nearest values, which lie above π/2 and π. So no bounded
link's f32 image passes its upper bound, and f32 saturation no longer decodes to `α > π/2` or `β > π`, the overshoot
that made §3.3 turn the state by π and mirror it. The f64 images stay the nearest values, which lie below. TASK-M2-01's
merged `link_properties_a_constraint` is followed up: it checks bounded f32 outputs at the exact bounds, not at their
f32-rounded images, so the round-to-nearest constants fail it.

## References
- `decisions.md` § "R-417 — The f32 images of π/2 and π are generated rounded toward zero; the decode's range is closed: the lower poles are reached at underflow and `q` reaches `±q_max` *(closes RQ-270)*"
- `docs/design/principia_dd_generation_root.md` § "3.9 The link registry (consolidated from chart contract Part 2.5)"
- `docs/design/principia_dd_generation_root.md` § "5. Tests (properties any generator must satisfy)"
- `docs/design/principia_dd_decoder.md` § "3.2 Configuration — hyperspherical mass-weighted Jacobi"
- `docs/design/principia_dd_decoder.md` § "5. Unit tests"
- `docs/notes/principia_validation_ground_truth_note.md` § "Degenerate configs: open & unguarded, approach as a limit"
- `decisions.md` § "R-290 — qa may change test files that only qa has committed to *(closes RQ-172, amends R-237)*"

## Deliverables
- `crates/ledger/src/gen/links.rs`: the f32 literals of π/2 and π emitted rounded toward zero (`f32::from_bits`, or an
  exact literal of the same bits), the f64 literals unchanged; the emitter's rule stated in its doc comment, citing
  R-417.
- `crates/kernel/src/generated/links.rs` regenerated (`cargo xtask codegen`); no other generated output changes.
- `crates/kernel/tests/links.rs`: `link_properties_a_constraint` checks every bounded link's f32 output, saturated
  controls included, in the closed `[a, b]` at the exact bounds (`u.lo`, `u.hi` in f64, not `as f32`), with a
  registered negative control.
- `crates/ledger/tests/`: `link_f32_pi_toward_zero`, the emitted literals' bits.

## Acceptance tests
- `cargo test -p ledger link_f32_pi_toward_zero` — the emitted f32 literals of π/2 and π have bits `0x3FC9_0FDA` and `0x4049_0FDA`, each below its exact value, and the f64 literals are the nearest values (REQ-GEN-034).
- `cargo test -p kernel link_properties_a_constraint` — every bounded link's f32 output lies in the closed `[a, b]` at the exact bounds, saturated controls included (`α ≤ π/2`, `β ≤ π`, `|q| ≤ q_max`); control: the round-to-nearest literals fail it (REQ-GEN-034).
- `cargo xtask codegen` leaves the tree clean, and `cargo test -p kernel` and `cargo test -p ledger` pass (REQ-GEN-034).
- Review checklist (physics): the lower poles `α = 0` and `β = 0` are still reached where the link underflows, and `q` reaches `±q_max` at saturation; no link clamps its output (R-417) (REQ-GEN-034).

## Notes
- New task, applied per R-369 in R-417's port: TASK-M2-01 has merged, so the generator change, the regenerated links and
  the follow-up to its test need their own PR.
- **Who depends on it:** TASK-M2-03, whose `decode_ranges` (REQ-DEC-027) asserts `α ≤ π/2` and `β ≤ π` at f32 saturation
  and whose `chart_landmarks_lagrange_euler` (REQ-VAL-020) has β approach π from below; TASK-M2-07, through TASK-M2-03,
  whose `canonical_noop` (REQ-DEC-018) asserts the no-op at f32 saturation. TASK-M2-02 does not: its REQ-DEC-028 is a
  review checklist, and its configuration-block tests pass with either constant.
- The hashed bytes are unchanged: a link tree's `num` is the f64 value's bits (dd_generation_root §3.9, "The bytes"),
  and the f32 rounding is the emitter's. No cached or saved result predates the change (caching is M5's).
- A qa test that pins the f32-rounded bounds (for example in `crates/kernel/tests/qa_TASK-M2-01.rs`) is a ruling-forced
  change (R-417): qa makes it, and the PR lists it with the assertion it changes (R-290, R-369).
