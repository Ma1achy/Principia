# TASK-M4-04 — Tier B and Tier N: integer fields exact, continuous values within measured native-wgpu tolerances

- **Milestone:** M4
- **Closes:** REQ-VAL-060, REQ-VAL-061, REQ-VAL-064, REQ-VAL-072, REQ-VAL-073, REQ-VAL-075, REQ-SYS-026, REQ-VAL-140, REQ-VAL-141
- **Depends on:** TASK-M4-02
- **Needs (earlier milestones):** REQ-PAY-013, REQ-PAY-051, REQ-PAY-052, REQ-PAY-053, REQ-PAY-027, REQ-PAY-043, REQ-GEN-006, REQ-VAL-017, REQ-DEC-043, REQ-ENC-004, REQ-ENC-024, REQ-INT-031
- **Reviewers:** code, qa, physics
- **Pitfalls:** PIT-9, PIT-10, PIT-3
- **Size:** ~450 lines

## Goal
The two remaining per-step tiers. Tier B: given the same branch decisions and integer inputs, the integer and packed fields — step indices, `total_substeps`, `closure_step`, every descriptor field at its bit offset, and the word's mixed-radix arithmetic — match CPU↔GPU bit for bit; the f16 display scalars are not asserted there. Tier N: decode, `ICDescriptor`, `E₀`, one STEP from a shared state for every occupant, short pre-divergence trajectories and the encode round trip compare elementwise within tolerances that are **measured** on native in-process `wgpu` (R-85) and recorded, not guessed. Continuous divergence is never reconciled.

## References
- `docs/contracts/principia_parity_contract.md` § "Tier B — integer-exact *given the same branch decisions* (integer & packed fields)"
- `docs/notes/principia_gpu_determinism_note.md` § "The parity consequence (what shared source does and does not buy)"
- `decisions.md` § "R-84 — Branch decisions across precisions *(closes RQ-35)*"
- `decisions.md` § "R-86 — The payload doc governs the eight payload items *(closes RQ-37)*"
- `docs/contracts/principia_parity_contract.md` § "Tier N — tight numerical (checkable because nothing amplifies)"
- `docs/contracts/principia_parity_contract.md` § "4. Tolerance — and the cross-backend reality"
- `decisions.md` § "R-85 — Native wgpu sets the Tier-N tolerances *(closes RQ-36)*"
- `docs/contracts/principia_parity_contract.md` § "6. The harness"
- `docs/design/principia_dd_simstate_payload.md` § "Why closure is here, at this width, unconditionally"
- `docs/design/principia_dd_generation_root.md` § "3.1 `sample_descriptor` (u32)"
- `docs/design/principia_dd_generation_root.md` § "3.4 `SimState` scalars — with presentation metadata"
- `docs/design/principia_dd_simstate_payload.md` § "3. The word buffer — `free_group_word`"
- `docs/notes/principia_gpu_determinism_note.md` § "Why branches are special (continuous divergence is fine; branch divergence is not)"
- `docs/design/principia_dd_integrator.md` § "2. Consolidated contract"
- `docs/contracts/principia_parity_contract.md` § "2. The three tiers"
- `docs/contracts/principia_parity_contract.md` § "3. The load-bearing discipline: never accumulate before comparing"
- `decisions.md` § "R-72 — A missing definition is written by the task that needs it *(closes RQ-46 to RQ-55, definitions)*"
- `decisions.md` § "R-71 — A missing value becomes a calibration requirement *(closes RQ-46 to RQ-55, values)*"

## Deliverables
- `crates/validation/tests/tier_b.rs`: integer-exact assertions fed identical branch decisions; the parity fixture set run on both sides for the trajectory-level integer fields, restricted to trajectories away from the word's crossing-ambiguity band.
- `xtask` gate `tier-n-tolerances`: runs the kernel through native in-process `wgpu`, records the spread per quantity class, and writes the tolerance file `fixtures/gates/tier_n_tolerances.toml` (the parity contract's §4 starting figures replaced by the measured values, each above the measured spread).
- `xtask` gates `tier-n` and `gpu-vs-f64`: elementwise comparisons against that file.
- `crates/validation/tests/no_reconcile.rs`: a grep/structure test that no code path copies or snaps GPU state to CPU state or tightens an envelope below the recorded spread.

## Acceptance tests
- `cargo test -p validation tier_b_fields` — identical branch decisions and integer inputs fed to both sides: `t_end_step`, `t_dmin_step`, `total_substeps`, every packed descriptor field (`state`, `detail`, `saturated`, `dmin_pair`, `last_symbol`) at its bit offset, and the word arithmetic including `fgw_length_raw` match bit for bit; `d_min`, `dE_max`, `dLz_max` are checked at Tier N/S only (REQ-VAL-060).
- `cargo test -p validation tier_b_fixture_set` — the parity fixture set on CPU and GPU: bitwise equality of the packed descriptor, `t_end_step`, `t_dmin_step`, `closure_step`, `total_substeps`, the `state` values and the word's integer arithmetic, for trajectories away from the crossing-ambiguity band (REQ-VAL-072); the fixture words are bit-identical CPU vs GPU outside the ambiguity band (REQ-VAL-075).
- `cargo xtask gate tier-n-tolerances` — the kernel run through native in-process `wgpu`; the spread per quantity recorded; each tolerance set above it; CI has no Dawn job (REQ-VAL-064).
- `cargo xtask gate tier-n` — decode → `(m, r, p)`, `ICDescriptor` and `E₀`; one STEP from a shared state for every occupant; short pre-divergence trajectories against a growing bounded envelope; the encode round trip in physical units — each within its REQ-VAL-064 measured tolerance (starting points ~1e-5 relative decode, ~1e-5 one-step scaled by force magnitude, ~1e-6 `E₀`/`L_z` at t = 0, replaced by the measured values) (REQ-VAL-061).
- `cargo xtask gate gpu-vs-f64` — GPU vs the f64 CPU path on the parity fixtures: particle state, energy, `L_z` and trajectory-derived quantities within the parity contract's declared tolerances (the REQ-VAL-064 file), not bit for bit (REQ-VAL-073).
- `cargo test -p validation no_reconcile` plus the physics reviewer — no code path copies or snaps GPU state to CPU state; Tier N/S envelopes are not tightened below the measured spread (REQ-SYS-026).
- Definition: the pre-divergence envelope's form and the onset-time rule written into parity_contract Tier N and approved by the physics reviewer (REQ-VAL-140).
- Proposal: the ambiguity band's width, with the measured crossing-distance spread and the excluded fraction as evidence; the human confirms it at the M4 gate (REQ-VAL-141).

## Notes
- The recorded Tier N tolerances are R-85's measurement, not an R-71 calibration; the PR shows the spread and the margin chosen above it, and the physics reviewer checks the margin covers it comfortably (parity §4).
- Gaps: the width of the word's numerical-ambiguity band (REQ-VAL-075) and the form of Tier N's growing bounded envelope (REQ-VAL-061) are not given by the corpus.
- Closes, for gaps the corpus leaves open: REQ-VAL-140 (R-72 definition), REQ-VAL-141 (R-71 calibration) (REVIEW_QUEUE RQ-110 lists them for the human).
