# TASK-M2-24 — Decode across backends and decode totality: every pixel labelled, every chart

- **Milestone:** M2
- **Closes:** REQ-VAL-017, REQ-DEC-008, REQ-DEC-038, REQ-DEC-025
- **Depends on:** TASK-M2-06, TASK-M2-09, TASK-M2-14
- **Needs (earlier milestones):** REQ-GEN-004, REQ-VAL-007, REQ-PAY-014
- **Reviewers:** code, qa, physics
- **Pitfalls:** PIT-9, PIT-10
- **Size:** ~420 lines

## Goal
Over every chart of this milestone, the CPU-f64 and GPU-f32 instantiations agree on continuous decode values within the REQ-DEC-043 factor and take bit-identical branches (DEGENERATE, feasibility) on identical inputs, with every branch input a comparison against constants or single-rounded values (R-34, R-84); every decode satisfies Σmᵢ = 1, CoM = 0, Σpᵢ = 0 and I = 1; and every pixel carries a label — DEGENERATE(reason) is a closed, enumerated cause set, written into dd_decoder §2 with its map onto payload §2's decode_failed detail codes.

## References
- `docs/design/principia_dd_decoder.md` § "2. Consolidated contract (every clause that binds this component)"
- `docs/design/principia_dd_decoder.md` § "5. Unit tests"
- `docs/design/principia_dd_decoder.md` § "4. Seams (its side of each — the integration-test list)"
- `decisions.md` § "R-34 — FMA: explicit fma at every branch input, enumerated *(IE-6)*"
- `decisions.md` § "R-84 — Branch decisions across precisions *(closes RQ-35)*"
- `decisions.md` § "R-71 — A missing value becomes a calibration requirement *(closes RQ-46 to RQ-55, values)*"
- `docs/design/principia_chart_reference.md` § "0.7 Degeneracy — every pixel gets a label"
- `decisions.md` § "R-72 — A missing definition is written by the task that needs it *(closes RQ-46 to RQ-55, definitions)*"
- `docs/design/principia_dd_simstate_payload.md` § "`sample_descriptor` (low 16 bits of `packed_a`) — 10 used, rest reserved"
- `docs/design/principia_chart_reference.md` § "5.2 Tests that can fail"
- `docs/design/principia_chart_reference.md` § "0.2 Configuration — hyperspherical mass-weighted Jacobi"
- `docs/design/principia_chart_reference.md` § "0.3 Momentum — free Jacobi momenta"
- `docs/contracts/principia_parity_contract.md` § "Tier N — tight numerical (checkable because nothing amplifies)"
- `docs/notes/principia_gpu_determinism_note.md` § "The discipline (each rule = one measured failure)"
- `decisions.md` § "R-85 — Native wgpu sets the Tier-N tolerances *(closes RQ-36)*"

## Deliverables
- `crates/engine/tests/decode_cross_backend.rs` and `crates/kernel/tests/decode_totality.rs`, iterating the chart registry so every chart is covered.
- The dd_decoder §2 doc change (every DEGENERATE reason — M01_TINY, the no-qualifying-seed case and any other — with its decode_failed detail code; payload §2 agreeing), with a "Removed lines" note.
- The enumerated list of decoder branch inputs (R-34).

## Acceptance tests
- `cargo test -p engine decode_cross_backend` — decoder test 12: fuzz z and (u, v) on CPU-f64 and GPU-f32 (native wgpu); continuous outputs within the f32-eps-scaled tolerance of REQ-DEC-043; branch tags bitwise equal; feasibility K* ≥ L_z²/2 formed with explicit fma per the enumerated branch inputs (REQ-VAL-017).
- `cargo test -p kernel degenerate_totality` — the DEGENERATE reason enum is closed; fuzz z / (u, v) on every chart and assert every pixel carries an outcome label, none NaN, none missing (REQ-DEC-008).
- Doc review (physics): dd_decoder §2 lists every DEGENERATE reason with its decode_failed detail code; payload §2 agrees; physics reviewer approved (REQ-DEC-038).
- `cargo test -p kernel decode_identities_every_chart` — decoder test 1 and chart_reference §5.2: fuzz z and random (u, v) for every chart; Σpᵢ = 0 to machine precision; I = 1 after the canonical-frame decode (REQ-DEC-025).

## Notes
- The browser-WGSL leg of decoder test 12 is checked with the browser build against the native tolerances (R-85; REQ-VAL-116, M8).
- PIT-9: include a deliberately contaminated branch input to show the bitwise comparison can fail; PIT-10: the result is for identical inputs, decode only.
