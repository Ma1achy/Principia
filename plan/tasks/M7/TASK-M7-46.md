# TASK-M7-46 — The state map's docs port: principia_state_map.md, SM14's rows, the ruling and the queue (R-542)

- **Milestone:** M7
- **Closes:** REQ-COL-112
- **Depends on:** none
- **Needs (earlier milestones):** none
- **Reviewers:** code, qa, physics, gui
- **Pitfalls:** none
- **Size:** ~1,650 lines (docs)
- **Status:** done — closed by R-542 (its PR, rulings/R-542); it has no task PR

## Goal
Port the human's state map bundle of 21 Oct 2026: `principia_state_map.md` to `docs/design/`, every line and heading kept, the prototype's code transcribed where the file is silent; SM14's rows applied to the docs they name; R-542 with §0 verbatim and SM1 to SM15 as the applied text; SM13's Q1 to Q9 as RQ-480 to RQ-488 and the conflicts found as RQ-489 and RQ-496 to RQ-499. The human's message: "Tasks: one for the docs port (this PR)".

## References
- `decisions.md` § "R-542 — The state map, as `principia_state_map.md` gives it and its prototype's code transcribes it: each sample coloured by where its own system is now on a chosen chart, through one stain node, Chart position, on every chart, the shape sphere included, coloured by a base and layers; Shape n̂ a separate node, unchanged; a system that leaves the chart drawn at the edge it left by, its last position not stored; no sim struct changed; Chart position waits on RQ-489 *(extends R-64)*"
- `docs/design/principia_state_map.md` § "SM14. What this changes"
- `docs/design/principia_state_map.md` § "SM13. Queue, don't rule"

## Deliverables
- `docs/design/principia_state_map.md`, `docs/incoming/2026-10-21_state_map/`, `docs/gui/reference/state_map/`.
- SM14's rows in `principia_dd_simstate_payload.md` §5, `principia_colour_composition.md` §1.5, §2, §3 and §4.1, `principia_dd_colouring.md` §3.2 and `principia_render_gui_spec.md` Part II and §G6.
- R-542 in `decisions.md`; RQ-480 to RQ-489 and RQ-496 to RQ-499 in `REVIEW_QUEUE.md`.

## Acceptance tests
- Review checklist — REQ-COL-112's verify.

## Notes
- The precedent is TASK-M0-00, a task closed by a ruling's PR (applied per R-369, R-542 H9).
