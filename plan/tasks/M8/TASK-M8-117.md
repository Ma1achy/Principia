# TASK-M8-117 — Paths under a lock: the stored anchor

- **Milestone:** M8
- **Closes:** REQ-CHART-115
- **Depends on:** TASK-M4-37, TASK-M8-106, TASK-M2-23, TASK-M8-124
- **Needs (earlier milestones):** none
- **Reviewers:** code, qa, physics
- **Pitfalls:** none
- **Size:** ~250 lines

## Goal
Under a lock a path is a based homotopy: the anchor's IC words fixed at the centre on every frame, ends included.

## References
- `docs/design/principia_chart_composition.md` § "17.8 Lock: a based homotopy"
- `docs/contracts/principia_chart_decoder_contract.md` § "The lock (projective microscope)"

## Deliverables
- `crates/engine/src/lock.rs`: the words in the lock group; a chart-mode switch recording them.
- The offset form and the centre's select in each lowering.

## Acceptance tests
- `cargo test -p engine path_lock_centre` — one set of words at the probe on every frame, ends included, affine and nonlinear, δ = 0 and δ ≠ 0, snap-back bit for bit, the area factor at the centre within 0.5 of the ends' on swapped and orthogonal planes, `anchor not in B` labelled and unbased; controls: the select removed, offsets blended linearly (REQ-CHART-115).

## Notes
- **Build order (R-863):** stage 2, locked paths: the based morph, then the sliding anchor, with `path_lock_centre`, the test that the centre's IC is constant. It depends on stage 1's task, TASK-M8-124, and merges only with its tests green. It also waits, through TASK-M8-106 and TASK-M8-102, on three stage-3 tasks (TASK-M4-33, TASK-M4-34 and TASK-M8-100), against the order: RQ-744 (A).
- The linearised decoder's per-frame tangent plane (design § 17.8) needs REQ-DEC-033 first.
- Waits on its rulings item: RQ-721. Nothing here is implemented until ruled (the order's A6, "Nothing implemented until ruled"); TASK-M4-37 in Depends on holds it.
