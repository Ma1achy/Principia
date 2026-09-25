# TASK-M3-16 — Branch-cut crossing detection and the τ tie rule

- **Milestone:** M3
- **Closes:** REQ-INT-031, REQ-INT-046, REQ-INT-047, REQ-INT-048, REQ-INT-079, REQ-PAY-070
- **Depends on:** TASK-M3-04, TASK-M3-15
- **Needs (earlier milestones):** REQ-PAY-016, REQ-RENDER-002
- **Reviewers:** code, qa, physics
- **Pitfalls:** PIT-10
- **Size:** ~350 lines

## Goal
Branch-cut crossings are detected at every accepted substep on the shape sphere, ordered within a segment by crossing fraction `τ = d₀/(d₀ − d₁)` ascending with fixed cut-ID priority only as a tie-break, counted iff the signed distance goes negative → non-negative, identically on CPU and GPU, and appended to the word. The generator ↔ branch-cut convention (which cuts are `a` and `b`, and the crossing sign) is transcribed with citations from the literature (Montgomery; Šuvakov–Dmitrašinović) into the symbolic-dynamics contract §1 (R-125). The integrator contract gains the τ tie rule: the sort, the tolerance within which τ values are equal (a comparison-only branch), and the sign convention (R-72).

## References
- `docs/contracts/principia_integrator_contract.md` § "Part 5 — Units and the horizon (inherited scale gauge)"
- `docs/contracts/principia_parity_contract.md` § "Tier B — integer-exact *given the same branch decisions* (integer & packed fields)"
- `decisions.md` § "R-72 — A missing definition is written by the task that needs it *(closes RQ-46 to RQ-55, definitions)*"
- `docs/design/principia_dd_simstate_payload.md` § "3. The word buffer — `free_group_word`"
- `docs/design/principia_dd_simstate_payload.md` § "8. Build-time settles (measure / specify once running)"
- `docs/contracts/principia_symbolic_dynamics_contract.md` § "1. Generator ↔ branch-cut convention"

- `decisions.md` § "R-113 — The placement fixes are accepted as written *(closes RQ-93 to RQ-100)*"
- `decisions.md` § "R-125 — The branch-cut convention is M3's, transcribed from the literature *(closes RQ-96)*"
## Deliverables
- Doc change: `docs/contracts/principia_symbolic_dynamics_contract.md` §1 — the a/b branch-cut assignment and the crossing-direction sign, transcribed with citations (Montgomery; Šuvakov–Dmitrašinović) and cross-checked with the half-open crossing rule (R-125, REQ-PAY-070).
- Doc change: `docs/contracts/principia_integrator_contract.md` Part 5 — the τ-sort, tie tolerance and sign convention (definition, REQ-INT-079).
- `crates/kernel/src/word/crossing.rs` — signed distance to each cut per substep, the half-open count, τ ordering with the tie-break, feeding `append`.
- Fixtures: double crossing in one substep, crossing both cuts near their shared endpoint, endpoint exactly on a cut, cross-and-recross inside one macro-step.

## Acceptance tests
- `cargo test -p kernel crossing_order` — a synthetic double crossing in one substep orders by τ; equal τ uses cut-ID; an endpoint exactly on the cut follows the half-open rule; (REQ-INT-031).
- `cargo test -p kernel crossing_every_substep` — a fixture with a cross-and-recross inside one macro-step yields both crossings (REQ-INT-046).
- `cargo test -p kernel crossing_tau_sort` — a segment crossing both cuts near their shared endpoint appends symbols in τ order (REQ-INT-047).
- `cargo test -p kernel crossing_half_open` — endpoints exactly on a cut: no duplicate or missed crossing; same symbols on CPU and GPU (REQ-INT-048).
- The integrator contract states the sort, the tie tolerance (a comparison-only branch, identical on CPU and GPU) and the sign convention; physics reviewer approved (REQ-INT-079).
- Review (physics): symbolic_dynamics_contract §1 gives the a/b assignment and the crossing sign with their citations, matching the half-open rule; REQ-VAL-043 (TASK-M3-30) reproduces the published braid classes with it; confirmed by the human at the M3 gate (REQ-PAY-070).

## Notes
- R-125 settles the gap: symbolic_dynamics_contract §1's convention is REQ-PAY-070, closed here and transcribed from the literature (not defined under R-72). The punctured-sphere relation and pair attribution (REQ-PAY-071/072) are retired to v2 (PL-3, R-38).
- Definitions written here (R-72; physics reviewer approves before merge): REQ-INT-079.
- RQ-96 ruled: R-125 (above).
- RQ-97 ruled: R-113 — REQ-INT-031's "word identical CPU/GPU" is dropped from M3; M4 covers it by REQ-VAL-072 (the word's integer arithmetic).
- Waits on **RQ-120** (whether REQ-INT-048's GPU arm leaves M3, as RQ-97's fix dropped the others) — REQ-INT-048 carries it; its `same symbols on CPU and GPU` check needs the M4 kernel.
