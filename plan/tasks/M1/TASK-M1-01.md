# TASK-M1-01 — The read-side SimState: derived accessors and validity predicates

- **Milestone:** M1
- **Closes:** REQ-PAY-021, REQ-PAY-022, REQ-PAY-026, REQ-PAY-027, REQ-PAY-030, REQ-PAY-031, REQ-PAY-032, REQ-RENDER-013, REQ-RENDER-019
- **Depends on:** TASK-M0-15
- **Needs (earlier milestones):** REQ-GEN-002, REQ-GEN-004, REQ-GEN-007, REQ-PAY-002, REQ-PAY-008, REQ-PAY-010, REQ-PAY-012, REQ-RENDER-001, REQ-TOOL-003
- **Reviewers:** code, qa, physics
- **Pitfalls:** PIT-9, PIT-3
- **Size:** ~450 lines

## Goal
The generation root (M0) emits the stored layout and its pack/unpack. This task adds the read side the fragment and the host both program against: one unified read-side `SimState` type, fixed across tiers, whose derived members are computed at read and never stored — `ftle` (S_final/(step_count·dt_macro) with the partial renorm interval finalised), `diffusion_slope` (C_ty/C_tt(n), −1.0 for n < 2), `total_substeps_log2`, the time fractions, `orbit_count`/`retrograde`, current drifts — plus the derived validity predicates (`ftle_valid`, `diffusion_slope_valid`, the `sd_is_*` state predicates). The two stored Rust variants (`SimStateFTLE`, `SimStateBase`) unpack into that one read type; a tier-absent feature reads NaN at unpack and is never stored. Emitted as Rust (kernel/host) and WGSL (fragment) from the one layout definition.

## References
- `docs/contracts/principia_canonical_spec.md` § "6. Memory & deployment model *(authoritative: `memory_tiers`, `caching_contract`, `deep_zoom`, `systems_architecture`)*"
- `docs/contracts/principia_canonical_spec.md` § "9. The load-bearing invariants (the walls — the primary comparison checklist)"
- `docs/design/principia_temporal_architecture_note.md` § "Stability metrics are just running accumulators (they get simpler)"
- `docs/design/principia_debug_tooling_plan.md` § "B. Payload field views — `sample_descriptor` (bit-packed u32)"
- `docs/design/principia_temporal_architecture_note.md` § "Render swapping is unaffected — the struct is the boundary"
- `decisions.md` § "R-79 — NaN and sentinels *(closes RQ-30)*"
- `docs/design/principia_dd_simstate_payload.md` § "1. `SimState` — the hot struct"
- `docs/design/principia_dd_simstate_payload.md` § "5. Derived — computed at read, NOT stored"
- `docs/design/principia_dd_simstate_payload.md` § "6. Accessors (illustrative of generated output; source is the Rust layout definition — emitted as Rust for the kernel/host and WGSL for the fragment side)"
- `docs/design/principia_dd_simstate_payload.md` § "`sample_descriptor` (low 16 bits of `packed_a`) — 10 used, rest reserved"
- `docs/design/principia_dd_generation_root.md` § "3.1 `sample_descriptor` (u32)"
- `docs/contracts/principia_render_contract.md` § "Unpack layer (generated, one accessor per named field)"
- `docs/contracts/principia_render_contract.md` § "Field views (one per field, every struct)"
- `decisions.md` § "R-86 — The payload doc governs the eight payload items *(closes RQ-37)*"
- `docs/design/principia_dd_simstate_payload.md` § "4. Welford diffusion (streaming regression)"
- `docs/design/principia_dd_generation_root.md` § "3.4 `SimState` scalars — with presentation metadata"
- `docs/design/principia_dd_generation_root.md` § "5. Tests (properties any generator must satisfy)"
- `docs/contracts/principia_render_contract.md` § "Part 2 — Fixed pipeline, swappable slots"
- `docs/contracts/principia_lowering_contract.md` § "Part 3a — The uniform read-side interface (tier features degrade by NaN, not by struct shape)"
- `docs/design/principia_dd_simstate_payload.md` § "`times` (u32)"

## Deliverables
- `crates/ledger`: derived-accessor emission for both targets — `ftle`, `ftle_valid`, `diffusion_slope`, `diffusion_slope_valid`, `total_substeps_log2`, `tm_t_end_fraction`, `tm_t_dmin_fraction`, `orbit_count`, `retrograde`, the `sd_is_resolved_outcome/_running/_failed/_finished` predicates — each with exactly the payload §6 name.
- `crates/ledger`: the unified read-side `SimState` (WGSL struct + Rust struct) with plain members; the unpack from `SimStateFTLE` and from `SimStateBase` into it; tier-absent members filled with the canonical quiet-NaN bits.
- Generated output checked in under the generated-file guard (M0), e.g. `crates/ledger/generated/read_side.{rs,wgsl}`.
- Tests: `crates/ledger/tests/derived.rs` (unit + proptest), including a ledger scan asserting none of the removed fields exist.

## Acceptance tests
- `cargo test -p ledger derived_not_stored` — the generated ledger has no ftle / spread / log-proxy / n fields; the derived `ftle` equals a reference computed with the finalised renorm (REQ-PAY-021).
- `cargo test -p ledger ftle_baked_out` — with FTLE baked out, `ftle` reads NaN and `ftle_valid` is false; a grep of the generated validity logic finds no `isnan` (REQ-PAY-022).
- `cargo test -p ledger read_type_both_tiers` — the WGSL read type compiles for both tiers with an identical struct shape; with `has_ftle = false` the `ftle` accessor returns NaN (REQ-PAY-026).
- `cargo test -p ledger total_substeps_resume` (proptest) — resuming a synthetic N_sub accumulation at random steps gives the uninterrupted `total_substeps`; the proxy equals ⌊log₂ total⌋ for totals ≥ 2 and 0 for totals 0 and 1, across 0…2³²−1 (REQ-PAY-027).
- `cargo test -p ledger diffusion_slope` — n = 0, 1 → −1.0 and `diffusion_slope_valid` false; a latched sample uses its own n (`t_end_step`), not the playhead (REQ-PAY-030).
- Review checklist (code): the generated ledger contains none of the removed fields — `arc_length_n`, peak substep, a stored `ftle_valid`, `encounter_count`, per-pair tallies, `dominant_pair`, `trajectory_stats`, `timeout`, per-sample ensemble spread (REQ-PAY-031).
- `cargo test -p ledger ftle_valid_truth_table` — full truth table over tier / state / n / completed renorms (REQ-PAY-032).
- `cargo test -p ledger tier_absent_nan_bits` — at a no-FTLE variant `ftle` bitcasts to the canonical quiet-NaN pattern; at E = 0 `ensemble_spread` does likewise; an unbound word reads the sentinel word (REQ-RENDER-013).
- `cargo test -p ledger time_fraction` — `tm_t_end_fraction(w, 0) == 0`; (65535, 65535) → 1.0 exactly (REQ-RENDER-019).

## Notes
- Each unit test is shown able to fail (VAL-007 discipline, PIT-9): e.g. a mutated finalisation (plain S/t) must fail `derived_not_stored`, and a stored-NaN variant must fail `tier_absent_nan_bits`.
- The canonical quiet-NaN bit pattern and the "empty/sentinel word" an unbound word buffer reads are not given by the corpus (see Gaps in the milestone report); the task waits on them for REQ-RENDER-013.
- `ensemble_spread` is resolve-stage (M5); at M1 its read-side member exists and reads NaN at E = 0, which is all REQ-RENDER-013 asserts. RQ-75 (whether the fragment keeps a baked `has_ensemble`) is carried by TASK-M1-03.
