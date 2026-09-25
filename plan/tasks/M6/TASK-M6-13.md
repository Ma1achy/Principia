# TASK-M6-13 — The symbolic-dynamics contract §1–3 and the deterministic pair attribution

- **Milestone:** M6
- **Closes:** REQ-PAY-070, REQ-PAY-071, REQ-PAY-072
- **Depends on:** TASK-M3-30
- **Needs (earlier milestones):** REQ-PAY-023, REQ-INT-031, REQ-INT-047, REQ-INT-079, REQ-TOOL-035, REQ-VAL-043
- **Reviewers:** code, qa, physics
- **Pitfalls:** PIT-9
- **Size:** ~350 lines

## Goal
The symbolic-dynamics contract stops being OPEN for per-pair quantities (R-38): §1 fixes which branch cuts are `a` and `b` and the crossing-direction sign, matching the integrator's half-open convention; §2 states the thrice-punctured-sphere relation for the third loop; §3 gives the deterministic algorithm from a reduced word to `enc_01`, `enc_02`, `enc_12` and `dominant_pair = argmax`, invalid for truncated words. The algorithm is implemented once in the shared source and gives the same tallies on CPU and GPU.

## References
- `docs/contracts/principia_symbolic_dynamics_contract.md` § "1. Generator ↔ branch-cut convention"
- `docs/contracts/principia_symbolic_dynamics_contract.md` § "Status: OPEN — specification required before per-pair quantities are trusted"
- `docs/contracts/principia_symbolic_dynamics_contract.md` § "2. The punctured-sphere relation (third pair)"
- `docs/contracts/principia_symbolic_dynamics_contract.md` § "3. Third-pair attribution algorithm"
- `docs/design/principia_dd_simstate_payload.md` § "3. The word buffer — `free_group_word`"

## Deliverables
- Doc change: `docs/contracts/principia_symbolic_dynamics_contract.md` §1, §2, §3 written (and §4's truncation check stated as the algorithm's precondition).
- `crates/kernel/src/word/attribution.rs`: `attribute(word) -> Option<PairTallies>` (None when `fgw_reduced_length_valid` is false).
- Property test in `crates/kernel/tests/attribution.rs`: same word → same tallies on the f64 CPU and f32 GPU instantiations.

## Acceptance tests
- Review checklist (physics) (cross-checked against REQ-INT-079's sign convention) — contract §1 written and cross-checked with the integrator's sign convention (REQ-PAY-070).
- Review checklist (physics) — contract §2 written (REQ-PAY-071).
- `cargo test -p kernel pair_attribution_parity` (proptest) — same word → same tallies on CPU and GPU (REQ-PAY-072).

## Notes
- No per-pair view ships in M6; this task only makes the contract and the algorithm exist (R-38).
- These three are not `kind: definition`, but the corpus does not give the convention, relation or algorithm — who writes them is a Gap.
- Waits on RQ-96 (`REVIEW_QUEUE.md`): The branch-cut convention is needed in M3, required in M6, and has no author.
