# TASK-M0-10 — Payload ledger II: packed_a, packed_b, times and the descriptor, with Rust pack/unpack

- **Milestone:** M0
- **Closes:** REQ-PAY-004, REQ-PAY-012, REQ-PAY-013, REQ-PAY-014, REQ-PAY-015, REQ-PAY-018, REQ-GEN-005
- **Depends on:** TASK-M0-09
- **Needs (earlier milestones):** none
- **Reviewers:** code, qa, physics
- **Pitfalls:** PIT-9
- **Size:** ~450 lines

## Goal
The ledger transcribes payload §2's bit layouts and the Rust emitter writes the kernel/host pack/unpack (`no_std`): `sample_descriptor` in the low 16 bits of `packed_a` — `state` 0–2 (0 escape, 1 bounded, 2 collision, 3 running, 4 sim_failed, 5 decode_failed; 6–7 reserved and decoded as finished and untrusted), `detail` 3–4, `saturated` 5, `dmin_pair` 6–7, `last_symbol` 8–9, bits 10–15 reserved and decoding as zero — with `d_min` f16 in bits 16–31; `packed_b` with `dE_max` f16 low and `dLz_max` f16 high; `times` with `t_end_step` u16 low and `t_dmin_step` u16 high. Accessors carry the payload §6 names (`sd_*`, `pa_d_min`, `pb_dE_max`, `pb_dLz_max`, `tm_*`, `set_last_symbol`, the state predicates, `total_substeps_log2`); the f16 packer clamps to ±65504 before packing (payload §1); and `roundtrip_ctl` compares the whole raw word, so a contaminated value in any bit fails it — including bits an unpack masks off (pitfalls §9).

## References
- `docs/design/principia_dd_simstate_payload.md` § "2. Bit layouts (the packed u32s)"
- `docs/design/principia_dd_simstate_payload.md` § "`packed_a` (u32)"
- `docs/design/principia_dd_simstate_payload.md` § "`sample_descriptor` (low 16 bits of `packed_a`) — 10 used, rest reserved"
- `docs/design/principia_dd_simstate_payload.md` § "`packed_b` (u32)"
- `docs/design/principia_dd_simstate_payload.md` § "`times` (u32)"
- `docs/design/principia_dd_simstate_payload.md` § "6. Accessors (illustrative of generated output; source is the Rust layout definition — emitted as Rust for the kernel/host and WGSL for the fragment side)"
- `docs/design/principia_dd_simstate_payload.md` § "Why closure is here, at this width, unconditionally"
- `docs/design/principia_dd_generation_root.md` § "3.1 `sample_descriptor` (u32)"
- `docs/design/principia_dd_generation_root.md` § "3. The ledger (the exact content — two generators must emit identical bits)"
- `docs/design/principia_dd_generation_root.md` § "5. Tests (properties any generator must satisfy)"
- `docs/design/principia_dd_generation_root.md` § "6. Deferred / flagged"
- `docs/contracts/principia_render_contract.md` § "Unpack layer (generated, one accessor per named field)"
- `docs/contracts/principia_render_contract.md` § "Field views (one per field, every struct)"
- `docs/read_first/principia_01_pitfalls.md` § "9. A PARITY CHECK THAT MASKS THE BITS THE FORK LANDS IN"
- `decisions.md` § "R-86 — The payload doc governs the eight payload items *(closes RQ-37)*"
- `decisions.md` § "R-70 — Where two docs conflict, the later consolidated doc wins"
- `decisions.md` § "R-22 — Body indices are 0-based; pair ids name the opposite side *(CD-2, amended)*"

## Deliverables
- `crates/ledger/src/payload.rs` — entries for `packed_a`, the descriptor, `packed_b`, `times`, `total_substeps`.
- `crates/ledger/src/gen/rust.rs` — pack/unpack/insert emitters → `crates/kernel/src/payload/generated.rs` (accessors named as payload §6; f16 via a `no_std` binary16 conversion matching `pack2x16float`, clamping first).
- `crates/kernel/src/payload/roundtrip.rs` — `roundtrip_ctl`: pack → unpack → repack and compare the full raw u32, never the masked fields.
- Tests in `crates/kernel/tests/` and `crates/ledger/tests/`.

## Acceptance tests
- `cargo test -p kernel sd_accessors` — the generated `sd_*` accessors extract exactly bits 0–2, 3–4, 5, 6–7, 8–9; pack/unpack round-trips all 1024 values of bits 0–9; bits 10–15 decode as zero (REQ-PAY-004).
- `cargo test -p kernel packed_words` — packing known values gives the expected raw u32 bit patterns, and `pa_d_min`, `pb_dE_max`, `pb_dLz_max`, `tm_t_end_step`, `tm_t_dmin_step` return them (REQ-PAY-012).
- `cargo test -p ledger descriptor_table` — the generated extract/insert offsets and widths of every descriptor field equal payload §2's table (REQ-PAY-013).
- `cargo test -p kernel state_codes` — codes 0–7 decode; 6 and 7 report finished (`sd_is_finished`) and untrusted (`sd_is_failed`) (REQ-PAY-014).
- `cargo test -p kernel reserved_bits` — with every field packed at its maximum, bits 10–15 are zero; the static disjointness test lists 10–15 as reserved (REQ-PAY-015).
- `cargo test -p kernel roundtrip_ctl` (property) — flipping each bit of a packed descriptor word in turn makes `roundtrip_ctl` fail for every bit, including bits the unpack masks off; the recorded pitfalls §9 pass on a contaminated value now fails (REQ-PAY-018).
- `cargo test -p kernel f16_pairs` (property) — f16 pairs round-trip through the Rust `pack2x16float`/`unpack2x16float` equivalents within f16 epsilon over finite values in ±65504 (REQ-GEN-005).

## Notes
- The GPU halves of the round trips (the WGSL unpack and the kernel on the GPU) are TASK-M0-15.
- Dispatch's refusal of ⌈T/dt⌉ > 65535 (R-86) belongs to the dispatch that doesn't exist yet; the limit is in the constants register (TASK-M0-08).
- Failure `detail` categories follow payload §2; payload §8 says to confirm them against the integrator contract's failure modes when that is finalised (M3).
