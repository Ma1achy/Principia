# TASK-M8-117 — Paths under a lock: the stored anchor

- **Milestone:** M8
- **Closes:** REQ-CHART-115, REQ-CHART-146
- **Depends on:** TASK-M4-37, TASK-M8-106, TASK-M2-23, TASK-M8-124, TASK-M8-125
- **Needs (earlier milestones):** none
- **Reviewers:** code, qa, physics
- **Pitfalls:** none
- **Size:** ~250 lines

## Goal
Under a lock a path is a based homotopy: the anchor's IC words fixed at the centre on every frame, ends included, where the anchor stays in B; where it can't exist in B, the anchor slides continuously to its nearest point in B and the centre follows it (R-849).

## References
- `docs/design/principia_chart_composition.md` § "17.8 Lock: a based homotopy"
- `docs/contracts/principia_chart_decoder_contract.md` § "The lock (projective microscope)"
- `decisions.md` § "R-867 — The lock's anchor is stored as the signed starting state, not as a latent point; approved as TASK-M8-117"
- `decisions.md` § "R-866 — Locking must not move the view: the pin is set at the clicked pixel, its initial in-plane excursion that pixel's offset from the view's centre, and tilt and zoom turn about the pin, so the pinned pixel stays fixed on screen; an optional "centre on pin" action and a setting to snap automatically; exact on nonlinear charts, or the task says why *(amends R-69, R-473 J7 and render_gui_spec §G4)*"
- `decisions.md` § "R-849 — A typed combination with no registered inverse gets a general numerical solve on the CPU, recording its branch rule, marking its seams and labelled CPU-only; and under the lock a chart path never jumps: the anchor slides continuously to its nearest point in a constrained chart *(closes RQ-733; on RQ-721, closed by R-850)*"

## Deliverables
- `crates/engine/src/lock.rs`: the words in the lock group; a chart-mode switch recording them.
- The offset form and the centre's select in each lowering.
- The sliding anchor for a B the anchor can't exist in, and the definition of its nearest point in B written into chart_composition § 17.8 (R-849, R-72; REQ-CHART-146).
- The anchor stored as the signed starting state, its mirror half kept, not as a latent point (R-867); what `z_locked` still serves beside it (the sliders' re-basing, snap-back, a chart-mode switch) written into chart_composition § 17.8 under physics review (R-72; REQ-CHART-115).

## Acceptance tests
- `cargo test -p engine path_lock_centre` — one set of words at the probe on every frame, ends included, affine and nonlinear, δ = 0 and δ ≠ 0, snap-back bit for bit, the area factor at the centre within 0.5 of the ends' on swapped and orthogonal planes, all of it where the anchor stays in B; where it can't (a region D, a rest and a Burrau B), the stored words at `s = 0`, B's nearest point to the anchor at `s = 1`, the slide continuous between (the largest step in the probe's state between frames falling to at most 0.6 of the previous largest step each time the frame step is halved, twice over), no label and no jump; a time-axis chart to an IC-only chart keeps its labelled jump; controls: the select removed, offsets blended linearly, a labelled jump where the anchor can't exist in B, an unlabelled snap to B's nearest point at one mid-path frame (REQ-CHART-115).
- Review: § 17.8's definition of the nearest point in B, approved by the physics reviewer (REQ-CHART-146).

## Notes
- **Build order (R-863):** stage 2, locked paths: the owner's stage is the based morph, then the sliding anchor, with the test that the centre's IC is constant. This task is the whole stage: the based morph with the stored anchor, and then the sliding anchor R-849 gives it (the owner's RQ-721 rider, merged by PR #443), with `path_lock_centre` covering both arms and the definition of B's nearest point (REQ-CHART-146). The constant-centre test applies where the anchor stays in B; on a sliding path the centre follows the anchor (R-849), so `path_lock_centre` checks R-849's slide instead: the stored words at `s = 0`, B's nearest point at `s = 1`, continuous between, with no label and no jump, as the acceptance test says. RQ-744 (D), filed while that rider was not yet on `main`, is moot. It depends on stage 1's task, TASK-M8-124, and merges only with its tests green. It also waits, through TASK-M8-106 and TASK-M8-102, on three stage-3 tasks (TASK-M4-33, TASK-M4-34 and TASK-M8-100), against the order: RQ-744 (A).
- The linearised decoder's per-frame tangent plane (design § 17.8) needs REQ-DEC-033 first.
- Waits on its rulings item: RQ-721. Nothing here is implemented until ruled (the order's A6, "Nothing implemented until ruled"); TASK-M4-37 in Depends on holds it.
- *7 Nov 2026 (R-849, R-850):* RQ-721 and its rider are ruled. Under the lock no path makes a labelled jump: where the anchor can't exist in chart B, it slides continuously to its nearest point in B and the centre pixel follows it, the model's Theorem 8 its construction. "Nearest" is a definition (R-72) this task writes under physics review, the owner's example being to keep shape and masses where B allows and ease the held-fixed coordinates to B's values along the path. The jump stays only for a time-axis chart to an IC-only chart.
- *8 Nov 2026 (R-867):* the owner approved, as this task, the lock's anchor stored as the signed starting state, not a latent point: REQ-CHART-115 now says so. What PR #445 had recorded for this task (stage 2, R-863; the sliding anchor, R-849) stands.
- *8 Nov 2026 (R-866, applied per R-369):* locking doesn't move the view; TASK-M8-125 builds it in stage 1, and this task depends on it. *Was:* "- **Depends on:** TASK-M4-37, TASK-M8-106, TASK-M2-23, TASK-M8-124".
- **Waits on RQ-749** (`rq: RQ-749` on REQ-CHART-115): what `path_lock_centre` reads once the pin is off the centre, R-863's "the centre IC is constant" against R-866's pin at the clicked pixel.
