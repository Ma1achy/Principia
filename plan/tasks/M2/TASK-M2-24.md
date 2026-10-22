# TASK-M2-24 — Decode across backends and decode totality: every pixel labelled, every chart

- **Milestone:** M2
- **Closes:** REQ-DEC-008, REQ-DEC-038, REQ-DEC-025
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
- `docs/design/principia_dd_decoder.md` § "3.7 Energy normalisation (optional; keep-or-drop is decision B9)"
- `docs/contracts/principia_lowering_contract.md` § "Part 3 — The baking rules (compile-time vs uniform)"
- `decisions.md` § "R-25 — `η_E` is kept, with an explicit off switch *(CD-5)*"

## Deliverables
- `crates/engine/tests/decode_cross_backend.rs` and `crates/kernel/tests/decode_totality.rs`, iterating the chart registry so every chart is covered.
- The dd_decoder §2 doc change (every DEGENERATE reason — M01_TINY, the no-qualifying-seed case and any other — with its decode_failed detail code; payload §2 agreeing), with a "Removed lines" note.
- The enumerated list of decoder branch inputs (R-34).
- The decode stage's `E*` with `η_E` on (applied per R-369; Notes): `crates/kernel/src/stage/decode.rs` applies dd_decoder §3.7 through the shared `energy_norm` with `E*` read as a uniform, its explicit off switch (R-25) a uniform flag, not a baked variant, on a chart that allows it; the decode-only dispatch and `decode_only` take the same `Option`.

## Acceptance tests
- `cargo test -p engine decode_cross_backend` — decoder test 12: fuzz z and (u, v) on CPU-f64 and GPU-f32 (native wgpu); continuous outputs within the f32-eps-scaled tolerance of REQ-DEC-043; branch tags bitwise equal; feasibility K* ≥ L_z²/2 formed with explicit fma per the enumerated branch inputs; run with `η_E` off and on, the on runs covering rest starts (`K₀ = 0`) at `E* > V₀` (`ENERGY_UNSCALABLE`), targets `E* < V₀` (`ENERGY_INFEASIBLE`) and `E* = V₀` (untouched, untagged); on each backend the energy tag equals dd_decoder §3.7 recomputed from that backend's own emitted `V₀`, `E*`, `s` and `r_min_pair_0`, and a disagreement between backends is a fork only where those inputs are bit-identical, otherwise precision, as dd_decoder §3.1 reads `M₀₁` (applied per R-369, physics review 5541460731 F2) (REQ-VAL-017).
- `cargo test -p kernel degenerate_totality` — the DEGENERATE reason enum is closed; fuzz z / (u, v) on every chart and assert every pixel carries an outcome label, none NaN, none missing (REQ-DEC-008).
- Doc review (physics): dd_decoder §2 lists every DEGENERATE reason with its decode_failed detail code; payload §2 agrees; physics reviewer approved (REQ-DEC-038).
- `cargo test -p kernel decode_identities_every_chart` — decoder test 1 and chart_reference §5.2: fuzz z and random (u, v) for every chart; Σpᵢ = 0 to machine precision; I = 1 after the canonical-frame decode (REQ-DEC-025).

## Notes
- REQ-VAL-017, held by RQ-298 when this task merged, moves to TASK-M2-48, which rewrites the fma-built branch inputs and re-proves decoder test 12 (R-547, applied per R-369); the acceptance line above is this task's as merged.
- The browser-WGSL leg of decoder test 12 is checked with the browser build against the native tolerances (R-85; REQ-VAL-116, M8).
- **The `η_E` target on the GPU decode (applied per R-369, the porter's survey on the R-439 rulings PR).** No task held it: TASK-M2-04 built the CPU side (`energy_norm`) and TASK-M2-06 decodes with `η_E` off ("No `E*` uniform is in this task", PR #217). The corpus settles that `η_E` runs in the kernel's decode: the chart map and decode lower into the compute pipeline (lowering_contract Part 1), §3.7's rescale is a step of the decode, between the momentum block and the derived invariants, and decoder test 12 requires its branch tags bit-identical on every backend (dd_decoder §5). It settles that `E*` is a uniform: lowering_contract Part 3's compute table puts the chart params, "invariant targets" among them, under UNIFORM, and bakes only what is different code, so no edit compiles. The energy tags compare `E*` with `V₀`, which differs by precision and by the backends' `sqrt` and division, so the cross-backend test reads each backend's tag against its own emitted inputs, as §3.1 does for `M₀₁` (physics review 5541460731 F2). `E*` changes the decoded IC, so it is on the sim key, read here as part of `SimConfig`'s "chart id + params" (gui_state_contract §2). This task holds it because its cross-backend test is the first to need the energy tags on the GPU.
- PIT-9: include a deliberately contaminated branch input to show the bitwise comparison can fail; PIT-10: the result is for identical inputs, decode only.
- **The invariant charts' edge columns (applied per R-369, physics review 5554286403 F2 of TASK-M2-12's PR #230).** TASK-M2-12 holds `L_max ≤ √(2K)` exactly by stepping the rounded root down at most four floats, proven and tested on the CPU at f32 and f64; WGSL's nominal `sqrt` bound can exceed four ulps, so the GPU's edges are verified here: `decode_cross_backend`'s fuzz on `InvariantLE` and `InvariantLK` includes the edge columns `s ∈ {0, 1}`, where the construction's feasibility tag (INVARIANT_INFEASIBLE) must be bitwise equal across CPU-f64 and GPU-f32.
