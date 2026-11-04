# TASK-M4-29 — Written forms for `E_rel` and `|Δn̂|` that use no driver division or `√` in the decision (R-694)

- **Milestone:** M4
- **Closes:** REQ-INT-112
- **Depends on:** TASK-M4-02, TASK-M3-12
- **Needs (earlier milestones):** none
- **Reviewers:** code, qa, physics
- **Pitfalls:** PIT-2.2, PIT-9
- **Size:** ~250 lines

## Goal
R-694 reads R-297's bit-identity as branch decisions, and that holds only if the escape branch's two inputs reach their decisions through no driver's division or root. This task writes both forms, in the integrator contract, and builds them, so TASK-M4-03 enumerates and tests them.

## References
- `decisions.md` § "R-694 — With the compute setting off, R-297's bit-identity means the branch decisions of REQ-INT-057 and the Tier B fields only, continuous values free across drivers; `E_rel` and `|Δn̂|` take written forms with no driver division or `√` in the decision; the header records "Off" only for drivers measured, otherwise wgpu's path with "driver unverified" *(closes RQ-685; refines R-297)*"
- `decisions.md` § "R-72 — A missing definition is written by the task that needs it *(closes RQ-46 to RQ-55, definitions)*"
- `decisions.md` § "R-29 — The escape criterion's undefined parts *(IE-1, amended)*"
- `decisions.md` § "R-547 — Decode and encode numerics: the swap test is the conjugacy; step 0 is skipped within a calibrated `τ_I` and rescales in two steps; a branch input that must agree across backends is built only from operations every IEEE backend rounds alike, with no subnormal intermediate; the native build runs without MIR GVN and CI runs it at the release profile; T1 holds off the mirror band; REQ-SYS-015 is split *(closes RQ-283, RQ-295, RQ-296, RQ-298, RQ-299, RQ-360 and RQ-365; corrects R-34)*"
- `docs/contracts/principia_integrator_contract.md` § "Part 4 — Determinism, and the substep as the subtle seam"
- `docs/contracts/principia_integrator_contract.md` § "Part 7 — Detectors as the `SimState` producer"
- `docs/notes/principia_gpu_determinism_note.md` § "The discipline (each rule = one measured failure)"

## Deliverables
- `docs/contracts/principia_integrator_contract.md` Part 4: the written form of each decision, for example `E_rel > 0` multiplied through by `d` and squared, and `|Δn̂| < tau` squared and multiplied through by the `I`s; the sign conditions each step of the rewrite relies on; the range over which no product overflows or goes subnormal (REQ-INT-064); approved by the physics reviewer before code (REQ-INT-112, R-72).
- `crates/kernel`: the two forms at the decision, f32 and f64 alike; the escape detector (TASK-M3-12) reads them.
- `|Δn̂|`'s inputs traced to the stored or computed values they read: `crates/kernel/src/shape.rs`:51-69 divides by `m01` and `i`, and has a `√` in `s` feeding `p` and `q`, so the form must not rest on `n̂` as computed there.

## Acceptance tests
- `cargo test -p kernel branch_forms` — REQ-INT-112's verify: agreement with the exact comparison on fuzzed states and constructed near-ties at f32 and f64; control: `/` and `√` emulated one ulp off flip R-29's formula and not the written form.
- Review checklist (physics reviewer) — the forms, their sign conditions and their range; no `/` or `√` between the state and the decision (REQ-INT-112).

## Notes
- Added by R-694 (4 Nov 2026, RQ-685 (a)).
- The quantities are R-29's and do not change. A decision may differ from the old form's only at rounding, near a tie; any label fixture that moves is named in the PR.
- If a form cannot avoid a driver operation for an input, stop and queue it: R-694 rests on these forms.
- *4 Nov 2026 (RQ-697, from PR #408):* this task covers `E_rel` and `|Δn̂|` only, as R-694 names them; the gate's total `E > 0` is RQ-697's. It depends on TASK-M3-12, which waits on RQ-697, so it waits through that task. The form here at `ε = 0` must agree with R-670's at `ε ≠ 0`, `|Δv|⁴·(d² + ε²) > 4·(M_pair + m_b)²` (integrator contract Part 4; TASK-M9-39).
