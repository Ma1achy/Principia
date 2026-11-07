# TASK-M8-117 — Paths under a lock: the stored anchor

- **Milestone:** M8
- **Closes:** REQ-CHART-115
- **Depends on:** TASK-M8-106, TASK-M2-23
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
- `cargo test -p engine path_lock_centre` — one set of words at the probe on every frame, δ = 0 and δ ≠ 0, snap-back bit for bit; control: the select removed (REQ-CHART-115).

## Notes
- The linearised decoder's per-frame tangent plane (design § 17.8) needs REQ-DEC-033 first.
