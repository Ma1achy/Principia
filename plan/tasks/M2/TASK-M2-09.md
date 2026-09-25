# TASK-M2-09 — Invariant-momentum construction: rigid rotation, the seed family and feasibility

- **Milestone:** M2
- **Closes:** REQ-DEC-020, REQ-DEC-021, REQ-DEC-022, REQ-DEC-040
- **Depends on:** TASK-M2-03, TASK-M2-06
- **Needs (earlier milestones):** REQ-PAY-014, REQ-PAY-004
- **Reviewers:** code, qa, physics
- **Pitfalls:** PIT-10, PIT-9
- **Size:** ~420 lines

## Goal
The momentum stage for invariant charts follows chart_reference §2.2 exactly: v⁽ᴸ⁾ = (L_z/I)·J rᵢ; the seed family (ρ,0), (0,λ), (Jρ,0), (0,Jλ) converted to particle velocities, CoM drift and angular momentum projected out, mass-weighted normalisation; among seeds with ‖w⁽²⁾‖²_m > ε_w the largest-norm one, ties by seed order, DEGENERATE only when none qualifies (R-82); then a = √(2(K* − L_z²/2I)), vᵢ = vᵢ⁽ᴸ⁾ + a·wᵢ, pᵢ = mᵢvᵢ. A pixel with K* < L_z²/2I is tagged infeasible exactly on that inequality, branch-identical across backends, and its label (state and detail) is written into chart_reference §2.2.

## References
- `docs/design/principia_dd_decoder.md` § "3.4 Momentum"
- `docs/design/principia_chart_reference.md` § "2.2 Deterministic momentum construction"
- `docs/design/principia_chart_reference.md` § "2. Invariant-momentum charts `(Lz, E)` and `(Lz, K)`"
- `docs/design/principia_chart_reference.md` § "5.2 Tests that can fail"
- `docs/design/principia_dd_decoder.md` § "5. Unit tests"
- `decisions.md` § "R-82 — One mirror test, one seed rule *(closes RQ-33)*"
- `decisions.md` § "R-72 — A missing definition is written by the task that needs it *(closes RQ-46 to RQ-55, definitions)*"
- `docs/contracts/principia_chart_decoder_contract.md` § "The four axis kinds — a closed set"
- `docs/contracts/principia_chart_decoder_contract.md` § "Part 5 — Well-posedness and the validation contract"
- `docs/design/principia_dd_simstate_payload.md` § "`sample_descriptor` (low 16 bits of `packed_a`) — 10 used, rest reserved"
- `docs/notes/principia_gpu_determinism_note.md` § "The discipline (each rule = one measured failure)"

## Deliverables
- `crates/kernel/src/decode/invariant.rs` (the construction, the seed selection, the feasibility branch with its input formed by explicit `fma` per R-34).
- The chart_reference §2.2 doc change naming the infeasible pixel's state and detail, consistent with payload §2's enum, with a "Removed lines" note.
- Tests `crates/kernel/tests/invariant_construction.rs`; the GPU branch check in `crates/engine/tests/invariant_gpu.rs`.

## Acceptance tests
- `cargo test -p kernel invariant_construction` — property over random (u, v) and a grid of feasible (L_z, K*) at fixed configuration, including near K* → K_min: Σpᵢ = 0, L_z(p) = L_z target and K(p) = K* to machine precision (decoder test 8; chart_reference §5.2) (REQ-DEC-020).
- `cargo test -p kernel seed_selection` — a fixture where several seeds qualify selects the largest-norm one, not the first; an exact tie selects the earlier seed; forcing the primary seed to degenerate (ρ = 0) selects a fallback, not DEGENERATE; DEGENERATE fires only where no seed has ‖w⁽²⁾‖²_m > ε_w (REQ-DEC-021).
- `cargo test -p engine invariant_infeasible` — decoder test 8: infeasible pairs are tagged exactly on K* < L_z²/2, never dropped or NaN; the feasibility comparison is branch-identical on CPU-f64 and GPU-f32 for identical inputs (REQ-DEC-022).
- Doc review (physics): chart_reference §2.2 names the state and detail an infeasible pixel carries, consistent with payload §2's enum; physics reviewer approved (REQ-DEC-040).

## Notes
- Payload §2's `decode_failed` detail codes are 0 non-finite, 1 degenerate configuration, 2 invalid mass construction, 3 other/reserved; the definition must say whether an infeasible pixel is `decode_failed` (and which code) or another label. The full DEGENERATE reason map is REQ-DEC-038 (TASK-M2-24).
- Gap G9: decoder test 8's "to tolerance" for L_z and K has no value beyond "machine precision" in §5.2.
