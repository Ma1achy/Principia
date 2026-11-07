# TASK-M8-117 — Paths under a lock: the stored anchor

- **Milestone:** M8
- **Closes:** REQ-CHART-115, REQ-CHART-146
- **Depends on:** TASK-M4-37, TASK-M8-106, TASK-M2-23
- **Needs (earlier milestones):** none
- **Reviewers:** code, qa, physics
- **Pitfalls:** none
- **Size:** ~250 lines

## Goal
Under a lock a path is a based homotopy: the anchor's IC words fixed at the centre on every frame, ends included, where the anchor stays in B; where it can't exist in B, the anchor slides continuously to its nearest point in B and the centre follows it (R-849).

## References
- `docs/design/principia_chart_composition.md` § "17.8 Lock: a based homotopy"
- `docs/contracts/principia_chart_decoder_contract.md` § "The lock (projective microscope)"
- `decisions.md` § "R-849 — A typed combination with no registered inverse gets a general numerical solve on the CPU, recording its branch rule, marking its seams and labelled CPU-only; and under the lock a chart path never jumps: the anchor slides continuously to its nearest point in a constrained chart *(closes RQ-733; on RQ-721, closed by R-850)*"

## Deliverables
- `crates/engine/src/lock.rs`: the words in the lock group; a chart-mode switch recording them.
- The offset form and the centre's select in each lowering.
- The sliding anchor for a B the anchor can't exist in, and the definition of its nearest point in B written into chart_composition § 17.8 (R-849, R-72; REQ-CHART-146).

## Acceptance tests
- `cargo test -p engine path_lock_centre` — one set of words at the probe on every frame, ends included, affine and nonlinear, δ = 0 and δ ≠ 0, snap-back bit for bit, the area factor at the centre within 0.5 of the ends' on swapped and orthogonal planes, all of it where the anchor stays in B; where it can't (a region D, a rest and a Burrau B), the stored words at `s = 0`, B's nearest point to the anchor at `s = 1`, the slide continuous between (the largest step in the probe's state shrinking each time the frame step is halved, twice over), no label and no jump; a time-axis chart to an IC-only chart keeps its labelled jump; controls: the select removed, offsets blended linearly, a labelled jump where the anchor can't exist in B (REQ-CHART-115).
- Review: § 17.8's definition of the nearest point in B, approved by the physics reviewer (REQ-CHART-146).

## Notes
- The linearised decoder's per-frame tangent plane (design § 17.8) needs REQ-DEC-033 first.
- Waits on its rulings item: RQ-721. Nothing here is implemented until ruled (the order's A6, "Nothing implemented until ruled"); TASK-M4-37 in Depends on holds it.
- *7 Nov 2026 (R-849, R-850):* RQ-721 and its rider are ruled. Under the lock no path makes a labelled jump: where the anchor can't exist in chart B, it slides continuously to its nearest point in B and the centre pixel follows it, the model's Theorem 8 its construction. "Nearest" is a definition (R-72) this task writes under physics review, the owner's example keeping shape and masses where B allows and easing the held-fixed coordinates to B's values along the path. The jump stays only for a time-axis chart to an IC-only chart.
