# TASK-M7-52 — The one projection: each target chart's projection, the leaving rule on main's clamped encode and a curve axis's nearest point, one Rust source for Chart position and the trace (R-542 H11, H12) ⚑

- **Milestone:** M7
- **Closes:** REQ-COL-133, REQ-COL-134
- **Depends on:** TASK-M2-25, TASK-M2-26
- **Needs (earlier milestones):** none
- **Reviewers:** code, qa, physics
- **Pitfalls:** none
- **Size:** ~500 lines, the tests included

## Goal
One Rust source for SM1's `P_T`, "the target chart's projection, the one the trace uses": each target chart's projection of an encoded state, the leaving rule defined on main's clamped encode (a shown coordinate clamped by its link; R-72, REQ-COL-133), and a curve axis's nearest point. Chart position (TASK-M7-47, generated to WGSL, R-116) and the trace (TASK-M8-58) both call it. It reads no chart from a colour occupant, so it does not wait on RQ-489. **SM2 is a hard constraint** (the human's message, R-542): no change to `SimState`, the payload, the `ICDescriptor` or any layout; a need for a stored field is queued in `REVIEW_QUEUE.md`, never added.

## References
- `decisions.md` § "R-542 — The state map, as `principia_state_map.md` gives it and its prototype's code transcribes it: each sample coloured by where its own system is now on a chosen chart, through one stain node, Chart position, on every chart, the shape sphere included, coloured by a base and layers; Shape n̂ a separate node, unchanged; a system that leaves the chart drawn at the edge it left by, its last position not stored; no sim struct changed; Chart position waits on RQ-489 *(extends R-64)*"
- `docs/design/principia_state_map.md` § "SM1. Definition"
- `docs/design/principia_state_map.md` § "SM5. The four results of a projection"
- `docs/design/principia_lines.md` § "8. Traces"
- `docs/contracts/principia_inverse_encode_contract.md` § "Part 3 — Block inverses (closed forms, with their forward mates)"
- `docs/design/principia_dd_encode.md` § "2. Consolidated contract"

## Deliverables
- The projection module (`crates/render/`), its leaving rule and curve nearest point, generated to WGSL from the same source (R-116).
- The leave definition, written into `docs/design/principia_state_map.md` SM5 with the porting rule's "Removed lines" note (REQ-COL-133).
- Test `one_projection`.

## Acceptance tests
- Review checklist — REQ-COL-133's verify (the definition in SM5, checked by the physics reviewer).
- `cargo test -p render one_projection` — REQ-COL-134's verify.

## Notes
- **⚑ Physics review is required** (SM1, SM5): this task does not merge on the code reviewer's word alone.
- **Cut by PR #270's code review, 1 (applied per R-369, R-542 H11):** split from TASK-M7-47 so that TASK-M8-58's trace does not wait on RQ-489 through it.
- **Applied meanwhile:** RQ-498 (a), the shape-sphere target from the signed n̂; RQ-503 (a), main's `α_min = 0`.
