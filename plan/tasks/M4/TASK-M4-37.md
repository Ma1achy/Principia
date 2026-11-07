# TASK-M4-37 — The owner's rulings on the chart composition design, applied (a gate: not an implementer's task)

- **Milestone:** M4
- **Closes:** REQ-CHART-125
- **Depends on:** none
- **Needs (earlier milestones):** none
- **Reviewers:** code, qa, physics
- **Pitfalls:** none
- **Size:** ~100 lines of docs

## Goal
Hold every task of the chart composition design until the owner has ruled it: the order's A6, "Nothing implemented until ruled". TASK-M4-32 to 36, TASK-M7-72 and TASK-M8-100 to 117 name this task in Depends on, the field the ready list reads, so they can't be offered while it is open.

## References
- `docs/design/principia_chart_composition.md` § "14. Doors: the rulings items"

## Deliverables
- The answers to RQ-712 to RQ-731 applied to the design doc, its status no longer Proposed, on branch `task/TASK-M4-37`: the rulings-apply work, not an implementation.

## Acceptance tests
- Review checklist: every one of RQ-712 to RQ-731 closed by a ruling in `decisions.md` and carried into the design doc; `python3 plan/check_plan.py` green (REQ-CHART-125).

## Notes
- Never dispatched to an implementer ahead of the owner's answer. Its branch name is what marks it done for the ready list, so it merges from `task/TASK-M4-37`.
