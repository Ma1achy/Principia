# TASK-M0-11 — Payload ledger III: the word layout and continuation table, QuadReduction's members and spread_event

- **Milestone:** M0
- **Closes:** REQ-PAY-001, REQ-PAY-019
- **Depends on:** TASK-M0-09, TASK-M0-10
- **Needs (earlier milestones):** none
- **Reviewers:** code, qa, physics
- **Pitfalls:** none
- **Size:** ~350 lines

## Goal
The ledger completes the M0 payload. The word buffer's `.w` layout (payload bits 0–24, `length` 25–31, 127 the truncation sentinel) and payload §3's frozen continuation table (`inverse = [1,0,3,2]`, `cont_symbol[0..2]`, `continuation_index` derived) are ledger entries, emitted to Rust with the `fgw_*` accessors. Generation-root §3.7's `QuadReduction` member list is transcribed — `spread_event` stored as f16 (R-18) and no `ensemble_outcome_agreement`. With these, REQ-PAY-001's size facts hold together: `ICDescriptor` 64 B with declared padding and no stored E₀, and descriptor bits 10–15 reserved. `QuadReduction`'s sizing and alignment, and the fixed-size-scalars check, are M5's (TASK-M5-01, R-113).

## References
- `docs/design/principia_dd_simstate_payload.md` § "3. The word buffer — `free_group_word`"
- `docs/design/principia_dd_generation_root.md` § "3.3 `free_group_word` (uint4) — mixed-radix packing, **in a separate buffer**"
- `docs/design/principia_dd_generation_root.md` § "3.3a The word buffer — a parallel cold buffer"
- `docs/design/principia_dd_generation_root.md` § "3.6 `ICDescriptor` (12 × f32)"
- `docs/design/principia_dd_generation_root.md` § "3.7 `QuadReduction` — completed ledger"
- `docs/contracts/principia_canonical_spec.md` § "6. Memory & deployment model *(authoritative: `memory_tiers`, `caching_contract`, `deep_zoom`, `systems_architecture`)*"
- `docs/design/principia_systems_architecture.md` § "3. The membrane — the deployment view (demoted, not diminished)"
- `docs/design/principia_systems_architecture.md` § "6. Cross-cutting invariants (the load-bearing walls)"
- `decisions.md` § "R-86 — The payload doc governs the eight payload items *(closes RQ-37)*"
- `decisions.md` § "R-18 — The agreement value is `spread_event` *(closes RQ-16)*"
- `decisions.md` § "R-113 — The placement fixes are accepted as written *(closes RQ-93 to RQ-100)*"

## Deliverables
- `crates/ledger/src/payload.rs` — the word `.w` entries, the frozen continuation table as ledger data, and the `QuadReduction` entries.
- `crates/ledger/src/gen/rust.rs` — the table and `fgw_*` emitters, and the `QuadReduction` struct.
- `crates/kernel/tests/payload_sizes.rs`.

## Acceptance tests
- `cargo test -p kernel payload_sizes` — `size_of::<ICDescriptor>() == 64` with the padding a declared member and no stored E₀ field; descriptor bits 10–15 zero (REQ-PAY-001).
- `cargo test -p ledger spread_event` — the generated payload has `spread_event: f16` and no `ensemble_outcome_agreement` (REQ-PAY-019).
- `cargo test -p kernel continuation_table_rust` — the Rust tables equal payload §3's frozen arrays (the WGSL half and REQ-PAY-016 close in TASK-M0-13).

## Notes
- REQ-PAY-019's "the agreement view computes from spread_event" is a view; field views are M1 (REQ-TOOL-009 onward). This task holds the storage half.
- The latch `running_max_divergence` is per footprint, not a `QuadReduction` member (R-99); its layout is M5's (REQ-REF-045, TASK-M5-19, R-113).
- RQ-93 ruled: R-113 (option b) — REQ-PAY-001 keeps the ICDescriptor size and the descriptor bits; QuadReduction's sizing (REQ-PAY-089) and REQ-PAY-006 move to TASK-M5-01 with its member order, packing and histogram N (REQ-PAY-075, REQ-PAY-077).
