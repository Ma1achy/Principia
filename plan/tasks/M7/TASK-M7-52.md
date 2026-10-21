# TASK-M7-52 — The one projection: each target chart's projection, the leaving rule on main's clamped encode and a curve axis's nearest point, one Rust source for Chart position and the trace (R-542 H11, H12) ⚑

- **Milestone:** M7
- **Closes:** REQ-COL-133, REQ-COL-134
- **Depends on:** TASK-M2-25, TASK-M2-26
- **Needs (earlier milestones):** none
- **Reviewers:** code, qa, physics
- **Pitfalls:** none
- **Size:** ~500 lines, the tests included

## Goal
One Rust source for SM1's `P_T`, "the target chart's projection, the one the trace uses": each target chart's projection of an encoded state, the leaving rule defined on main's clamped encode (a shown coordinate clamped by its link; R-72, REQ-COL-133), and a curve axis's nearest point. Its four consumers call it: Chart position (TASK-M7-47, generated to WGSL, R-116), the hover trace's `z(t)` (TASK-M8-10), the line system's trace (TASK-M8-49) and the follow-up's leaving and curve projection (TASK-M8-58). It proposes the one calibration value it needs, REQ-COL-129's native–WGSL agreement tolerance. It reads no chart from a colour occupant, so it does not wait on RQ-489. **SM2 is a hard constraint** (the human's message, R-542): no change to `SimState`, the payload, the `ICDescriptor` or any layout; a need for a stored field is queued in `REVIEW_QUEUE.md`, never added.

## References
- `decisions.md` § "R-542 — The state map, as `principia_state_map.md` gives it and its prototype's code transcribes it: each sample coloured by where its own system is now on a chosen chart, through one stain node, Chart position, on every chart, the shape sphere included, coloured by a base and layers; Shape n̂ a separate node, unchanged; a system that leaves the chart drawn at the edge it left by, its last position not stored; no sim struct changed; Chart position waits on RQ-489 *(extends R-64)*"
- `docs/design/principia_state_map.md` § "SM1. Definition"
- `docs/design/principia_state_map.md` § "SM5. The four results of a projection"
- `docs/design/principia_lines.md` § "8. Traces"
- `docs/contracts/principia_inverse_encode_contract.md` § "Part 3 — Block inverses (closed forms, with their forward mates)"
- `docs/design/principia_dd_encode.md` § "2. Consolidated contract"
- `decisions.md` § "R-71 — A missing value becomes a calibration requirement *(closes RQ-46 to RQ-55, values)*"
- `decisions.md` § "R-297 — Fast-math per shader stage: off for compute by default, an explicit and recorded opt-in; display may keep it *(amends R-84, R-116)*"

## Deliverables
- The projection module in `crates/kernel` beside the encode (the only crate rust-gpu compiles), its leaving rule and curve nearest point; its WGSL snippet emitted to `crates/render/wgsl/` by `cargo xtask build-kernel`, as TASK-M2-25's decode and encode are (R-116); the host trace calls the kernel function. *Was:* "The projection module (`crates/render/`), …, generated to WGSL from the same source (R-116)." (code re-review C2 of PR #270).
- The leave definition, written into `docs/design/principia_state_map.md` SM5 with the porting rule's "Removed lines" note (REQ-COL-133).
- Test `one_projection_agreement`.
- The proposal of REQ-COL-129's projection agreement tolerance (R-71, in R-297's form): the native-against-WGSL agreement measured over the seeded states, the value written in the PR for the human to confirm at the M7 gate; until then `one_projection_agreement` and REQ-COL-135's fragment arm use the measured value, as TASK-M2-25 uses REQ-DEC-043's factor.

## Acceptance tests
- Review checklist — REQ-COL-133's verify (the definition in SM5, checked by the physics reviewer).
- `cargo test -p render one_projection_agreement` — REQ-COL-134's verify (native against generated WGSL; code re-review C3, option (a), applied per R-369). Each consumer's call is REQ-COL-135's, checked in its own task.

## Notes
- **⚑ Physics review is required** (SM1, SM5): this task does not merge on the code reviewer's word alone.
- **Cut by PR #270's code review, 1 (applied per R-369, R-542 H11):** split from TASK-M7-47 so that TASK-M8-58's trace does not wait on RQ-489 through it.
- **Each link's margin (physics re-review note of PR #270):** since R-481 the links clamp at different margins (`tanh_q` and `softsign_q` at `q_max(1 − ε_q)`); this task writes each link's margin into SM5, per REQ-COL-133.
- **Its consumers:** TASK-M7-47 (Chart position), TASK-M8-10 (the hover trace's `z(t)`), TASK-M8-49 (the line system's trace projection and breaks) and TASK-M8-58 (the follow-up's leaving and curve projection), each depending on this task (R-542 H11).
- **REQ-COL-129's agreement entry (PR #270 code re-review, C5, applied per R-369):** this task proposes that one value and uses its measured value until the M7 gate. It does not close REQ-COL-129 (TASK-M7-48 does), so it does not wait on RQ-487, which concerns the tiling defaults and the other constants.
- **References (C5):** REQ-COL-129 is named through its rulings, R-71 and R-297, above; check_plan takes only file sections there.
- **Goal reworded (C5):** *Was:* "Chart position (TASK-M7-47, generated to WGSL, R-116) and the trace (TASK-M8-58) both call it."
- **Applied meanwhile:** RQ-498 (a), the shape-sphere target from the signed n̂; RQ-503 (a), main's `α_min = 0`.
