# TASK-M4-37 — The owner's rulings on the chart composition design, applied (a gate: not an implementer's task)

- **Milestone:** M4
- **Closes:** REQ-CHART-125
- **Depends on:** none
- **Needs (earlier milestones):** none
- **Reviewers:** code, qa, physics
- **Pitfalls:** none
- **Size:** ~100 lines of docs

## Goal
Hold every task of the chart composition design until the owner has ruled it: the order's A6, "Nothing implemented until ruled". TASK-M4-32 to 34, TASK-M7-72 and TASK-M8-100 to 120 name this task in Depends on, the field the ready list reads, so they can't be offered while it is open. Only those 25 tasks wait behind this gate; no existing task, in M4 or M9, does.

## References
- `docs/design/principia_chart_composition.md` § "14. Doors: the rulings items"

## Deliverables
- The answers to RQ-712 to RQ-731 applied to the design doc, its status no longer Proposed, on branch `task/TASK-M4-37`: the rulings-apply work, not an implementation.
- An edge to apply when RQ-714 rules: TASK-M4-06 gains TASK-M4-34 in Depends on, since TASK-M4-06's golden (REQ-RENDER-082) needs RQ-714's domain. No existing task gains a dependency on this design while it is Proposed. From then until TASK-M4-34 merges the edge holds 230 tasks at this plan: TASK-M4-06 and the 217 other existing open tasks behind it, and 12 of this design's tasks.
- Edges to apply if RQ-729 rules for the presets: TASK-M9-20, -22 and -28 gain TASK-M8-112 in Depends on, and take its presets in place of their own functions (design § 8.5).

## Acceptance tests
- Review checklist: every one of RQ-712 to RQ-731 closed by a ruling in `decisions.md` and carried into the design doc; `python3 plan/check_plan.py` green (REQ-CHART-125).

## Notes
- RQ-711 is read at this gate, beside RQ-712 to RQ-731: it also covers the base point, centre and window of region D's preset (TASK-M4-34) and the paper-frame presets (TASK-M8-112). Only those home values wait: the pair, the masks, the tests and the TASK-M4-06 edge do not.
- RQ-733 is read at this gate, beside RQ-711: the classification stands whichever way it rules, so nothing here waits on it.
- RQ-734 and RQ-735 are read at this gate too. Neither holds a task of the design: no task builds a final-condition chart, and the native tasks go ahead whichever way RQ-735 rules.
- RQ-736 is read at this gate, beside RQ-714: its filter order changes TASK-M4-34's preset, which renders the full disc, and its conflicts decide what that preset is before TASK-M8-120 exists. It holds that preset and TASK-M8-120, not the pair or any other task.
- Never dispatched to an implementer ahead of the owner's answer. Its branch name is what marks it done for the ready list, so it merges from `task/TASK-M4-37`.
- *7 Nov 2026 (the rulings PR of that day):* ruled, to carry into the design's body: RQ-712 (R-834), RQ-713 (R-835), RQ-715 (R-836), RQ-716's cap (R-837), RQ-717 (R-838), RQ-718 (R-846), RQ-719 (R-839), RQ-720 (R-840), RQ-721 (R-850, with its rider, R-849), RQ-722 (R-841), RQ-723, RQ-725 and RQ-726 (R-847), RQ-724 (R-848), RQ-727 (R-842), RQ-728 (R-843), RQ-730 (R-844), RQ-733 (R-849), RQ-736 (R-853, R-854, R-855), RQ-714's time and velocities (R-845, R-851) and the model's § 6b (R-852); the design's status block lists them. Still open: RQ-714's outside, so the TASK-M4-06 edge waits; RQ-716's 2 s on Vulkan; RQ-729, RQ-731 and RQ-732, the record PR's; RQ-711's zoom-row scale; RQ-734, RQ-735, RQ-738 and RQ-739.
