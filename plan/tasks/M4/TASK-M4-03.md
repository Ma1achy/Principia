# TASK-M4-03 — Tier L: branch decisions bit-identical on identical inputs, at the enumerated branch inputs

- **Milestone:** M4
- **Closes:** REQ-INT-057, REQ-INT-061, REQ-INT-064, REQ-INT-066, REQ-EVT-021, REQ-VAL-059, REQ-VAL-078
- **Depends on:** TASK-M4-02
- **Needs (earlier milestones):** REQ-INT-028, REQ-INT-029, REQ-EVT-003, REQ-EVT-004, REQ-EVT-005, REQ-EVT-020, REQ-DEC-017, REQ-ENC-016, REQ-DEC-022, REQ-DEC-008, REQ-DEC-007, REQ-PAY-018
- **Reviewers:** code, qa, physics
- **Pitfalls:** PIT-9, PIT-10
- **Size:** ~450 lines

## Goal
The determinism pin, held and tested. Every branch input of the kernel is enumerated in the source (R-34): `d²` for the substep bucket and collision, the windowed `|Δn̂|` and `E_rel` for escape, the cap and horizon decisions, and the rest. Each multi-op branch input is written with explicit `fma` in a fixed form, and `d²` is position-quantised. The stateless no-fork suite then feeds identical inputs to CPU-f64, CPU-f32 and the native GPU and asserts every Tier L decision equal, against a control that must fork. Descriptor determinism is audited at the inputs that feed it, not at the packing expression.

## References
- `docs/contracts/principia_canonical_spec.md` § "2. The determinism law (the other defining decision)"
- `docs/contracts/principia_canonical_spec.md` § "9. The load-bearing invariants (the walls — the primary comparison checklist)"
- `docs/design/principia_systems_architecture.md` § "1. The rings — cross-cutting services"
- `docs/design/principia_systems_architecture.md` § "2. Component inventory, by rung"
- `docs/design/principia_systems_architecture.md` § "5. The seam catalogue"
- `docs/design/principia_systems_architecture.md` § "6. Cross-cutting invariants (the load-bearing walls)"
- `docs/design/principia_core_design.md` § "Principle: divergence is the observable"
- `docs/contracts/principia_integrator_contract.md` § "Part 4 — Determinism, and the substep as the subtle seam"
- `docs/contracts/principia_integrator_contract.md` § "Part 2 — Occupants and the capability profile"
- `docs/design/principia_dd_integrator.md` § "2. Consolidated contract"
- `docs/notes/principia_gpu_determinism_note.md` § "Why branches are special (continuous divergence is fine; branch divergence is not)"
- `docs/contracts/principia_parity_contract.md` § "Tier L — exact, no tolerance (the bulk of the suite)"
- `decisions.md` § "R-84 — Branch decisions across precisions *(closes RQ-35)*"
- `docs/contracts/principia_integrator_contract.md` § "Rules the new kernel must hold by construction"
- `docs/design/principia_dd_integrator.md` § "3.3 Substep law — and the determinism pin, made concrete"
- `docs/notes/principia_gpu_determinism_note.md` § "The one-line law"
- `docs/notes/principia_gpu_determinism_note.md` § "The discipline (each rule = one measured failure)"
- `docs/contracts/principia_parity_contract.md` § "Tier B — integer-exact *given the same branch decisions* (integer & packed fields)"
- `docs/contracts/principia_integrator_contract.md` § "And FMA contraction is a live question"
- `decisions.md` § "R-34 — FMA: explicit fma at every branch input, enumerated *(IE-6)*"
- `docs/read_first/principia_INDEX.md` § "Known open items"
- `docs/read_first/principia_01_pitfalls.md` § "9. A PARITY CHECK THAT MASKS THE BITS THE FORK LANDS IN"
- `docs/contracts/principia_parity_contract.md` § "2. The three tiers"

## Deliverables
- `crates/kernel/src/branch_inputs.rs`: the enumerated list of branch inputs (name, formula, the explicit-`fma` written form, the decisions it feeds, the descriptor fields downstream of it), referenced from each use site.
- Kernel edits: `d²` as f32 positions → lone subtract → lone multiply → `fma(dy, dy, dx*dx)`; collision as `d² < r_coll²`; `|Δn̂|` and `E_rel` in their fixed `fma` forms; no runtime `pow`/`sqrt`/`div` feeding a decision (the spike control `ceil((r_sub/√d²)^1.5)` absent from the kernel).
- `crates/validation/tests/tier_l.rs`: property tests over fuzzed and synthetic boundary states; the boundary-state fixture set (`fixtures/gates/tier_l_boundary_states`); a control kernel variant built only in the test with the runtime-transcendental `N_sub`, which must fork.
- `crates/validation/tests/descriptor_inputs.rs`: for each descriptor field, fork-tests at its branch inputs (per the enumerated list), plus a packing-level control that must read 0 forks where the input-level test finds them (pitfalls §9's 0 vs 83).

## Acceptance tests
- `cargo test -p validation tier_l_d2` — `d²` computed on every backend is bit-identical for fuzzed position pairs (REQ-INT-066).
- `cargo test -p validation tier_l_collision` — fuzzed pairs near `r_coll` on CPU and GPU: the collision decision is bit-identical (REQ-EVT-021).
- `cargo test -p validation tier_l_branches` — identical per-step inputs (fuzzed states) dispatched on GPU-f32 and CPU-f64: `N_sub`, cap, collision, escape, horizon, `SIM_FAILED` and terminal decisions bit-exact; no code adjusts continuous values toward agreement; no test asserts label equality along a chaotic trajectory (REQ-INT-057).
- `cargo test -p validation tier_l_boundary_states` — stateless test over boundary states: `N_sub`, terminal label and precedence, `state`, escaper identity, collision pair, feasibility and degenerate branch + reason, mirror-tie choice, coincident-pair rejection, decode-mode selection and loop iteration structure identical across CPU-f64, CPU-f32 and native GPU; spike baseline 705 states, 0 forks; the runtime-`pow` control forks on the same inputs (REQ-VAL-059).
- Physics reviewer, against `branch_inputs.rs`: an enumerated list of branch inputs exists; each multi-op input is written with explicit `fma`; nothing relies on per-backend FP-contraction settings (REQ-INT-064); every branch input is enumerated and none is fed by runtime `pow`/`sqrt`/`div`, the spike control absent (REQ-INT-061).
- `cargo test -p validation descriptor_input_forks` — the parity audit enumerates the branch inputs feeding each descriptor field (per R-34's list) and fork-tests those; the packing-level control reads 0 forks against the input-level 83 (REQ-VAL-078).

## Notes
- REQ-VAL-059's verify names a browser-GPU leg; the browser build is M8. See the REVIEW_QUEUE entries below — this task runs the three native legs and does not mark the fourth done.
- Domain statement required in the test output: fixed inputs, per step (pitfalls §10).
- Waits on RQ-97 (`REVIEW_QUEUE.md`): GPU and browser legs before the GPU kernel or the browser exists.
