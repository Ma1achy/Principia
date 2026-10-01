# TASK-M0-11 — Payload ledger III: the word layout and continuation table, QuadReduction's members and spread_event

- **Milestone:** M0
- **Closes:** REQ-PAY-001, REQ-PAY-019
- **Depends on:** TASK-M0-09, TASK-M0-10
- **Needs (earlier milestones):** none
- **Reviewers:** code, qa, physics
- **Pitfalls:** none
- **Size:** ~350 lines

## Goal
The ledger completes the M0 payload. The word buffer's `.w` layout (payload bits 0–24, `length` 25–31, 127 the truncation sentinel) and payload §3's frozen continuation table (`inverse = [1,0,3,2]`, `cont_symbol[0..2]`, `continuation_index` derived, with 3 in its four `next = inverse(prev)` cells, R-307) are ledger entries, emitted to Rust with the `fgw_*` accessors. Generation-root §3.7's `QuadReduction` member list is transcribed as ledger data, each member's name and §3.7 type, not emitted (R-306) — `spread_event` stored as f16 (R-18) and no `ensemble_outcome_agreement`. The generated struct is TASK-M5-01's (R-306, R-113). With these, REQ-PAY-001's size facts hold together: `ICDescriptor` 64 B with declared padding and no stored E₀, and descriptor bits 10–15 reserved. `QuadReduction`'s sizing and alignment, and the fixed-size-scalars check, are M5's (TASK-M5-01, R-113).

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
- `decisions.md` § "R-306 — `QuadReduction`'s member list is ledger data at M0; the struct is built at M5 *(closes RQ-178)*"
- `decisions.md` § "R-307 — `continuation_index` holds 3 where `next` is `prev`'s inverse *(closes RQ-179)*"
- `decisions.md` § "R-313 — `ICDescriptor` follows `Real`; R-86's 64 B is its f32 instantiation *(closes RQ-185; amends R-86)*"
- `decisions.md` § "R-315 — `n_unresolved` is a u16 `QuadReduction` member, like `valid_sample_count` *(closes RQ-183)*"
- `decisions.md` § "R-319 — An out-of-range input to the continuation tables is a `debug_assert!` failure; in release it returns 3 *(vetoes #99's item 8)*"
- `decisions.md` § "R-321 — The continuation-table functions are total: each input is debug-asserted < 4, then masked to 2 bits *(amends R-319)*"
- `decisions.md` § "R-324 — The digit argument of `continuation_symbol` and `predecessor_symbol` is debug-asserted < 3, then clamped with `min(d, 2)` *(completes R-321)*"

## Deliverables
- `crates/ledger/src/payload.rs` — the word `.w` entries, the frozen continuation table as ledger data, and `QuadReduction`'s §3.7 member list as ledger data: each member's name and §3.7 type, not a §3.8 entry and not emitted (R-306).
- `crates/ledger/src/gen/rust.rs` — the table and `fgw_*` emitters. The `QuadReduction` struct is TASK-M5-01's (R-306).
- `crates/kernel/tests/payload_sizes.rs`.

## Acceptance tests
- `cargo test -p kernel payload_sizes` — `size_of::<ICDescriptor>() == 64` (the f32 instantiation, R-313) with the padding a declared member and no stored E₀ field; descriptor bits 10–15 zero (REQ-PAY-001).
- `cargo test -p ledger spread_event` — the ledger's `QuadReduction` member list has `spread_event` typed f16 and no `ensemble_outcome_agreement` (REQ-PAY-019, R-306).
- `cargo test -p kernel continuation_table_rust` — the Rust tables equal payload §3's frozen arrays, `continuation_index` included with 3 in its four `next = inverse(prev)` cells (R-307) (the WGSL half and REQ-PAY-016 close in TASK-M0-13).
- `cargo test -p kernel continuation_table_out_of_range` and `cargo test --release -p kernel continuation_table_out_of_range` — each table function given a symbol input ≥ 4 fails its `debug_assert!` in the debug build and in the release build returns the cell at `input & 3` (R-321, amending R-319); `continuation_index` returns 3 only for its four inverse cells (R-307); `continuation_symbol` and `predecessor_symbol` given a digit ≥ 3 fail their `debug_assert!` in the debug build and in the release build return the digit-2 cell (`min(d, 2)`, R-324).

## Notes
- REQ-PAY-019's "the agreement view computes from spread_event" is a view; field views are M1 (REQ-TOOL-009 onward). This task holds the storage half.
- The latch `running_max_divergence` is per footprint, not a `QuadReduction` member (R-99); its layout is M5's (REQ-REF-045, TASK-M5-19, R-113).
- RQ-93 ruled: R-113 (option b) — REQ-PAY-001 keeps the ICDescriptor size and the descriptor bits; QuadReduction's sizing (REQ-PAY-089) and REQ-PAY-006 move to TASK-M5-01 with its member order, packing and histogram N (REQ-PAY-075, REQ-PAY-077).
- From PR #42's physics review (29 Sep): §3.8's vector rule (TASK-M0-07) treats a `u-bits` component at a scalar index
  as a full u32, so §3.7's `class_histogram[N]` (u8 × N) doesn't fit it. This task places it or files it in
  REVIEW_QUEUE. Filed as RQ-178; R-306 moves its placement, with the struct, to TASK-M5-01.
- RQ-179 ruled: R-307 — the four `next = inverse(prev)` cells of `continuation_index` hold 3 ("invalid"), as
  `dmin_pair`'s 3 does; the WGSL half (TASK-M0-13) carries the same table.
- RQ-183 ruled: R-315 — `n_unresolved` is u16 (§3.7's temporal-accumulators row), so the member list types it u16.
- R-319 (1 Oct) vetoes PR #99's item 8: an out-of-range input to a table function is a `debug_assert!` failure, and
  returns 3 in release, never the last cell.
- R-321 (1 Oct) amends R-319: the table functions are total — each input `debug_assert!`-ed < 4, then masked `& 3` —
  so an input ≥ 4 reads the cell at `input & 3`; R-319's "returns 3" is replaced. The value at a digit of 3 is flagged
  to the human, not yet ruled.
