# TASK-M4-37 — The owner's rulings on the chart composition design, applied (a gate: not an implementer's task)

- **Milestone:** M4
- **Closes:** REQ-CHART-125
- **Depends on:** none
- **Needs (earlier milestones):** none
- **Reviewers:** code, qa, physics
- **Pitfalls:** none
- **Size:** ~100 lines of docs

## Goal
Hold every task of the chart composition design until the owner has ruled it: the order's A6, "Nothing implemented until ruled". TASK-M4-32 to 34, TASK-M7-72 and TASK-M8-100 to 119 name this task in Depends on, the field the ready list reads, so they can't be offered while it is open. Only those 24 tasks wait behind this gate; no existing task, in M4 or M9, does.

## References
- `docs/design/principia_chart_composition.md` § "14. Doors: the rulings items"

## Deliverables
- The answers to RQ-712 to RQ-731 applied to the design doc, its status no longer Proposed, on branch `task/TASK-M4-37`: the rulings-apply work, not an implementation.
- An edge to apply when RQ-714 rules: TASK-M4-06 gains TASK-M4-34 in Depends on, since TASK-M4-06's golden (REQ-RENDER-082) needs RQ-714's domain. No existing task gains a dependency on this design while it is Proposed. From then until TASK-M4-34 merges the edge holds 229 tasks at this plan: TASK-M4-06 and the 217 other existing open tasks behind it, and 11 of this design's tasks.
- Edges to apply if RQ-729 rules for the presets: TASK-M9-20, -22 and -28 gain TASK-M8-112 in Depends on, and take its presets in place of their own functions (design § 8.5).

## Acceptance tests
- Review checklist: every one of RQ-712 to RQ-731 closed by a ruling in `decisions.md` and carried into the design doc; `python3 plan/check_plan.py` green (REQ-CHART-125).

## Notes
- Never dispatched to an implementer ahead of the owner's answer. Its branch name is what marks it done for the ready list, so it merges from `task/TASK-M4-37`.
