# TASK-M0-09 — Payload ledger I: SimState (both variants), ICDescriptor and the word buffer, with generated Rust structs

- **Milestone:** M0
- **Closes:** REQ-PAY-002, REQ-PAY-003, REQ-PAY-005, REQ-PAY-007, REQ-PAY-008, REQ-PAY-009, REQ-PAY-010, REQ-PAY-020, REQ-GEN-001, REQ-RENDER-002
- **Depends on:** TASK-M0-07, TASK-M0-08
- **Needs (earlier milestones):** none
- **Reviewers:** code, qa, physics
- **Pitfalls:** none
- **Size:** ~480 lines

## Goal
The ledger transcribes payload §0–§1 and generation-root §3.4–§3.6, and its Rust emitter writes the structs into `crates/kernel`: `SimStateFTLE` (144 B) and `SimStateBase` (96 B, no `r_sh`/`p_sh`), both 8-byte aligned with no further padding, phase state and shadow vec2-grouped, the closure fields (`closure_min: f32`, `closure_step: u16`, `_reserved: u16`) unconditional in both, the accumulators `S`, `theta`, `mean_y`, `C_ty` and the drift references `E_0`, `Lz_0` f32, no bf16 anywhere; `ICDescriptor` with its 12 f32 fields named per R-22 and R-62 (`rho_mag`, `lambda_mag`); the word as a separate buffer of one `vec4<u32>` per sample — the only field split out of the hot struct; and generation-root §3.4's catalogue metadata (scale, range, sentinel; `energy_drift` and `Lz_drift` diverging). No time-indexed per-sample buffer exists, and no per-pair field or view (`enc_01/02/12`, `encounter_count`, `dominant_pair`) is in the ledger (R-38).

## References
- `docs/design/principia_dd_simstate_payload.md` § "0. The two buffers (split by access pattern)"
- `docs/design/principia_dd_simstate_payload.md` § "1. `SimState` — the hot struct"
- `docs/design/principia_dd_simstate_payload.md` § "Why closure is here, at this width, unconditionally"
- `docs/design/principia_dd_simstate_payload.md` § "4. Welford diffusion (streaming regression)"
- `docs/design/principia_dd_simstate_payload.md` § "5. Derived — computed at read, NOT stored"
- `docs/design/principia_dd_simstate_payload.md` § "7. Memory"
- `docs/design/principia_dd_simstate_payload.md` § "8. Build-time settles (measure / specify once running)"
- `docs/design/principia_dd_generation_root.md` § "3.4 `SimState` scalars — with presentation metadata"
- `docs/design/principia_dd_generation_root.md` § "3.5 The live-state block (replaces the checkpoint array — lockstep, ratified)"
- `docs/design/principia_dd_generation_root.md` § "3.6 `ICDescriptor` (12 × f32)"
- `docs/design/principia_dd_generation_root.md` § "6. Deferred / flagged"
- `docs/design/principia_dd_generation_root.md` § "3.3 `free_group_word` (uint4) — mixed-radix packing, **in a separate buffer**"
- `docs/contracts/principia_render_contract.md` § "Part 1 — The payload (render input)"
- `docs/contracts/principia_render_contract.md` § "Unpack layer (generated, one accessor per named field)"
- `docs/contracts/principia_symbolic_dynamics_contract.md` § "Status: OPEN — specification required before per-pair quantities are trusted"
- `docs/contracts/principia_symbolic_dynamics_contract.md` § "Downstream consumers waiting on this contract"
- `docs/contracts/principia_symbolic_dynamics_contract.md` § "3. Third-pair attribution algorithm"
- `docs/design/principia_dd_generation_root.md` § "3.1 `sample_descriptor` (u32)"
- `docs/design/principia_debug_tooling_plan.md` § "B. Payload field views — `sample_descriptor` (bit-packed u32)"
- `docs/contracts/principia_canonical_spec.md` § "6. Memory & deployment model *(authoritative: `memory_tiers`, `caching_contract`, `deep_zoom`, `systems_architecture`)*"
- `docs/contracts/principia_canonical_spec.md` § "11. Still open / downstream (not yet fully in the corpus)"
- `docs/design/principia_systems_architecture.md` § "2. Component inventory, by rung"
- `decisions.md` § "R-22 — Body indices are 0-based; pair ids name the opposite side *(CD-2, amended)*"
- `decisions.md` § "R-62 — `rho_mag` and `lambda_mag` *(amends R-22)*"
- `decisions.md` § "R-38 — Per-pair views are out of v1 until `dominant_pair` is specified *(PL-3)*"
- `decisions.md` § "R-40 — Memory tiers and the quality device key off `eps` *(PL-5)*"
- `decisions.md` § "R-59 — Fix D1 to D6 as listed *(sheet §8)*"
- `decisions.md` § "R-73 — Apply the whole ruling-follow-up checklist now *(closes RQ-56)*"

## Deliverables
- `crates/ledger/src/payload.rs` — the entries for `SimStateFTLE`/`SimStateBase`, `ICDescriptor`, the word buffer and the §3.4 scalar metadata.
- `crates/ledger/src/gen/rust.rs` — the struct emitter (`#[repr(C)]`, `no_std`-compatible, vec2 groups as `[[f32; 2]; 3]`) writing `crates/kernel/src/payload/generated.rs`.
- `crates/kernel/tests/payload_layout.rs` — sizes, alignments and every field offset against payload §1.
- `crates/ledger/tests/payload_ledger.rs` — field-by-field comparison with the ledger tables, the render_contract Part 1 subset check, precision, catalogue metadata and the no-per-pair check.
- The recomputed widths printed by the layout test and recorded in the PR (R-40, R-59 D6).

## Acceptance tests
- `cargo test -p ledger payload_fields` — generated Rust layouts compared field-by-field against the ledger tables; render_contract Part 1's field lists are a subset (REQ-PAY-002).
- `cargo test -p kernel payload_layout` — `align_of::<SimState*>() == 8`; `size_of` recorded for the FTLE-on and FTLE-off variants (144 / 96), and the corpus quotes match (REQ-PAY-003); size_of/align_of of both generated variants are 144/8 and 96/8 and every field offset matches the §1 table (REQ-PAY-008).
- `cargo test -p kernel closure_fields` — both variants carry `closure_min: f32` and `closure_step: u16`; review checklist (physics §8): any ledger change touching `_reserved` includes a size/alignment recheck (REQ-PAY-009).
- `cargo test -p ledger payload_precision` — the ledger types of `r`, `p`, `r_sh`, `p_sh`, `S`, `theta`, `mean_y`, `C_ty`, `E_0`, `Lz_0` are f32 and no bf16 type appears in any generated artefact (REQ-PAY-010).
- `cargo test -p kernel icdescriptor_names` — the generated `ICDescriptor` has `rho_mag` and `lambda_mag` and no `rho0_mag`/`rho1_mag` (REQ-PAY-020).
- `cargo test -p ledger catalogue_scalars` — a catalogue entry for each §3.4 field with its scale, range and sentinel; `energy_drift` and `Lz_drift` have scale = diverging (REQ-GEN-001).
- `cargo test -p ledger payload_buffers` — the generated layout has exactly two payload buffers (SimState + word); `closure_min`/`closure_step` are SimState members, not a parallel buffer (REQ-PAY-007); no time-indexed per-sample buffer exists — review checklist (code §4) (REQ-PAY-005).
- `cargo test -p ledger no_per_pair` — none of `enc_01`, `enc_02`, `enc_12`, `encounter_count`, `dominant_pair` is a ledger or catalogue entry; review checklist (code §7): no per-pair view in the v1 catalogue or colour-mode list (REQ-RENDER-002).

## Notes
- The payload doc is canonical where it and generation-root §3 could drift (generation-root §3; R-70, R-86).
- The f32 layout is the one transcribed here; the per-precision emission is TASK-M0-14 (REQ-PAY-017).
- `δ_dep`'s departed bit (REQ-VAL-055) and the f32-field failed-state contents (R-72) are M3; not here.
- See Gaps: §3.8 has no location kind for the derived §3.4 fields (`ftle`, `energy_drift`, `Lz_drift`, `diffusion`).
