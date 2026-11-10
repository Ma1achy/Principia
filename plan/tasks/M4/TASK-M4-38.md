# TASK-M4-38 — Per-backend accuracy of cos near π/2 against f64 (REQ-RENDER-122)

- **Milestone:** M4
- **Closes:** REQ-RENDER-122
- **Depends on:** TASK-M4-27
- **Needs (earlier milestones):** none
- **Reviewers:** code, qa, physics
- **Pitfalls:** none
- **Size:** ~150 lines

## Goal
R-822 takes RQ-629's option (b) at M4: WGSL and Vulkan bound `sin` and `cos` only by an absolute error, where the
decode's `cos α` near `π/2` needs a relative one. The GPU suite checks each backend's `cos` near `π/2` against f64,
relative to its value, so a backend whose `cos α` there is worse than the decode's `ε_phys` allows is caught before
ROUNDTRIP's acceptance (REQ-RENDER-122).

## References
- `decisions.md` § "R-822 — GPU `sin` and `cos` near π/2: a per-backend check against f64 at M4, the kernel's own `sin` and `cos` ruled with RQ-687 on M4's cross-backend numbers; RQ-679 deferred to M8 and RQ-687 to M4; BR-1 runs both the authors' starts and the seeded draw, each labelled *(defers RQ-629, RQ-679 and RQ-687; closes RQ-696)*"

## Deliverables
- A per-backend check of `cos` near `π/2` against f64, relative to its value, in the GPU suite (R-822, RQ-629 (b) at M4; REQ-RENDER-122).
- Negative controls for this task's tests (R-176).

## Acceptance tests
- `cargo test -p engine gpu_cos_near_half_pi` — REQ-RENDER-122's arm on each backend in the suite, with its control.
- `python3 plan/check_plan.py`.

## Notes
- Split from TASK-M4-27 (applied per R-369, 10 Nov 2026): the arm was added to TASK-M4-27 after its PR's approval.
  The deliverable and the acceptance line above are TASK-M4-27's, moved verbatim (`plan/tasks/M4/TASK-M4-27.md`
  keeps them as *Was*). R-822's record still reads "TASK-M4-27 builds it"; that names the task at the ruling's date.
- RQ-629 is in `REVIEW_QUEUE.md`, which a References bullet can't cite; R-822 above defers it and decides its option (b).
  Option (c), the kernel's own `sin` and `cos`, is ruled with RQ-687 on TASK-M4-27's and TASK-M4-28's cross-backend
  numbers, not here.
- REQ-RENDER-122's verify: on each backend in the suite, `cos` at f32 arguments approaching `π/2` is within the stated
  relative bound of f64 `cos` at the same arguments, the bound read from `ε_phys`'s budget; the control, a stand-in `cos`
  with only the 2⁻¹¹ absolute bound, fails.
