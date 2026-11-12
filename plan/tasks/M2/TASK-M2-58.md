# TASK-M2-58 — The owner's rulings on the chart composition design, applied (a gate: not an implementer's task)

- **Milestone:** M2
- **Closes:** REQ-CHART-125
- **Depends on:** none
- **Needs (earlier milestones):** none
- **Reviewers:** code, qa, physics
- **Pitfalls:** none
- **Size:** ~100 lines of docs

## Goal
Hold every task of the chart composition design until the owner has ruled it: the order's A6, "Nothing implemented until ruled". TASK-M4-32 to 34, TASK-M7-72, TASK-M8-100 to 120 (now, after R-916, TASK-M2-59, TASK-M2-60, TASK-M2-61 and TASK-M2-65 to TASK-M2-85 where they moved), TASK-M2-78 and TASK-M2-79 (R-847's fixed-`k` and scattering charts) and TASK-M2-61 (added by R-863, the owner's build order of 8 Nov 2026) name this task in Depends on, the field the ready list reads, so they can't be offered while it is open. Only those 28 tasks wait behind this gate; no existing task, in M4 or M9, does.

## References
- `docs/design/principia_chart_composition.md` § "14. Doors: the rulings items"

## Deliverables
- The answers to RQ-712 to RQ-731 applied to the design doc, its status no longer Proposed, on branch `task/TASK-M2-58`: the rulings-apply work, not an implementation.
- No edge from TASK-M4-06 to TASK-M4-34 (R-914, RQ-752 (a), 9 Nov 2026): it would have closed a cycle through stage 2, so it is dropped and never applied; region D's GPU reference is TASK-M4-34's own (REQ-RENDER-123), and TASK-M4-06 and the 217 other open tasks behind it are released. No existing task gains a dependency on this design while it is Proposed. *Was:* "- An edge to apply when RQ-714 rules: TASK-M4-06 gains TASK-M4-34 in Depends on, since TASK-M4-06's golden (REQ-RENDER-082) needs RQ-714's domain. No existing task gains a dependency on this design while it is Proposed. From then until TASK-M4-34 merges the edge holds 230 tasks at this plan: TASK-M4-06 and the 217 other existing open tasks behind it, and 12 of this design's tasks."
- RQ-729's edges, applied by R-916 in plan/tasks.yaml: TASK-M9-20, -22 and -28 gain TASK-M2-77 in Depends on, and take its presets in place of their own functions (design § 8.5).
- Closing step, as this task lands (the owner's chart re-plan corrections of 11 Nov 2026, item 3; applied per R-369: record): mark `plan/charts/m2-chart-system-replan.md`, `plan/charts/m2-chart-addendum-extensions-axes.md` and `plan/charts/m2-chart-replan-addition.md` as superseded by the composition design's body (`docs/design/principia_chart_composition.md`), a status line at the head of each, so implementers read one current design. *12 Nov 2026 (the chart audit's F10 and F20; applied per R-369: record):* before marking them, check that every open item O1 to O12 and every § D contradiction has a task home or a recorded disposal: O3 and O12 are disposed of in the re-plan's B.7 (:449, :458); O8's home is M-S on TASK-M6-07. *Was:* the step as it stands, ending "so implementers read one current design."

## Acceptance tests
- Review checklist: every one of RQ-712 to RQ-731 closed by a ruling in `decisions.md` and carried into the design doc; `python3 plan/check_plan.py` green (REQ-CHART-125).

## Notes
- When this gate releases the tasks, they are built in R-863's five stages, each merging only with the previous stage's tests green (design § 15); what the order leaves open is RQ-744.
- RQ-711 is read at this gate, beside RQ-712 to RQ-731: it also covers the base point, centre and window of region D's preset (TASK-M4-34) and the paper-frame presets (TASK-M2-77). Only those home values wait: the pair, the masks, the tests and the TASK-M4-06 edge do not.
- RQ-733 is read at this gate, beside RQ-711: the classification stands whichever way it rules, so nothing here waits on it.
- RQ-734 and RQ-735 are read at this gate too. Neither holds a task of the design: no task builds a final-condition chart, and the native tasks go ahead whichever way RQ-735 rules.
- RQ-736 is read at this gate, beside RQ-714: its filter order changes TASK-M4-34's preset, which renders the full disc, and its conflicts decide what that preset is before TASK-M8-120 exists. It holds that preset and TASK-M8-120, not the pair or any other task.
- Never dispatched to an implementer ahead of the owner's answer. Its branch name is what marks it done for the ready list, so it merges from `task/TASK-M2-58`.
- *7 Nov 2026 (the rulings PR of that day):* ruled, to carry into the design's body: RQ-712 (R-834), RQ-713 (R-835), RQ-715 (R-836), RQ-716's cap (R-837), RQ-717 (R-838), RQ-718 (R-846), RQ-719 (R-839), RQ-720 (R-840), RQ-721 (R-850, with its rider, R-849), RQ-722 (R-841), RQ-723, RQ-725 and RQ-726 (R-847), RQ-724 (R-848), RQ-727 (R-842), RQ-728 (R-843), RQ-730 (R-844), RQ-733 (R-849), RQ-736 (R-853, R-854, R-855), RQ-714's time and velocities (R-845, R-851) and the model's § 6b (R-852); the design's status block lists them. Still open: RQ-714's outside, so the TASK-M4-06 edge waits; RQ-716's 2 s on Vulkan; RQ-729, RQ-731 and RQ-732, the record PR's; RQ-711's zoom-row scale; RQ-734, RQ-735, RQ-738 and RQ-739.
- *8 Nov 2026 (R-871 to R-902):* every one of RQ-712 to RQ-731 is now ruled: RQ-714 (R-871), RQ-716 (R-872), RQ-729 (R-884), RQ-731 (R-891), with RQ-711 (R-888), RQ-734 (R-879), RQ-735 (R-889), RQ-739 (R-874), RQ-740 (R-876), RQ-741 (R-877), RQ-744 (R-873) and RQ-745 (R-875). This task applies the TASK-M4-06 edge (TASK-M4-34 in its Depends on) and RQ-729's edges (TASK-M9-20, -22 and -28 gain TASK-M2-77). RQ-734 now holds a task, TASK-M8-128 (final-condition charts), and RQ-735 one, REQ-CHART-155 on TASK-M8-102. RQ-738 stays open on the model's items, outside this gate.
- *9 Nov 2026 (R-914):* RQ-752 (a): the TASK-M4-06 edge the note above names is dropped, never applied; this task applies RQ-729's edges only. Region D's GPU reference is TASK-M4-34's (REQ-RENDER-123), and TASK-M4-34 depends on TASK-M4-06 instead.
- *9 Nov 2026 (R-916):* moved from M4 to M2 and renumbered: formerly TASK-M4-37; the header follows plan/tasks.yaml. *Was:* "Milestone: M4".
- *9 Nov 2026 (R-916):* every one of RQ-712 to RQ-731 is ruled, so this task marks the design no longer Proposed and is the first task of M2's chart core; RQ-729's edges are applied in plan/tasks.yaml by the rulings PR itself. *Was (Deliverables):* "- Edges to apply if RQ-729 rules for the presets: TASK-M9-20, -22 and -28 gain TASK-M8-112 in Depends on, and take its presets in place of their own functions (design § 8.5)." *Was (Goal, mangled by the renumbering and restored):* "TASK-M4-32 to 34, TASK-M7-72, TASK-M8-100 to 120".
- *11 Nov 2026 (the re-plan's addition (`plan/charts/m2-chart-replan-addition.md`) § F.4; the re-plan's § A.1 and the addendum's § A.3, CHANGE):* the gate also carries into the design's body, each with its *Was*: R-925, R-926 and the answers to RQ-758 to RQ-770 (R-934 to R-936, R-943); the answers to RQ-771 to RQ-781 (R-932, R-937 to R-942); and R-944, R-945, R-947 to R-952 with the addition's R-369 decisions, from the design's dated notes. (D) (marks to the lock's tolerance mid-morph, the filter on) and (F) (a tagged end that carries a state blends) go into the body only once the physics reviewer has confirmed them; until then the dated notes stand as conditional.
- *11 Nov 2026 (the owner's chart re-plan corrections, item 3; applied per R-369: record):* the closing step in Deliverables is added now so it is not lost: the owner's words, "When TASK-M2-58 lands, mark plan/charts/m2-chart-system-replan.md, m2-chart-addendum-extensions-axes.md and m2-chart-replan-addition.md as superseded by the composition design's body, so implementers read one current design." `plan/tasks.yaml` lists no steps, so it is unchanged. R-923's hold (integrator work before charts) still holds the chart tasks.
